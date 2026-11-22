#!/usr/bin/env python3
"""
Verification script for Tauri v2 ACL commands, permissions, and capabilities parity.
Fails if any command registered in an invoke handler is missing from build.rs,
permissions TOML files, capabilities, or if any frontend invoke is unpermitted.

Single Source of Truth: apps/app/src/commands.rs
"""
import glob
import json
import os
import re
import sys

def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    os.chdir(root)

    # 1. Parse Single Source of Truth (apps/app/src/commands.rs)
    commands_file = "apps/app/src/commands.rs"
    with open(commands_file, "r", encoding="utf-8") as f:
        commands_content = f.read()

    macro_pattern = re.compile(
        r'define_plugin_commands!\s*\(\s*([a-zA-Z0-9_]+)\s*,\s*([a-zA-Z0-9_]+)\s*,\s*\[(.*?)\]\s*\);',
        re.DOTALL
    )
    handler_to_cmds = {}
    const_to_cmds = {}
    for match in macro_pattern.finditer(commands_content):
        handler_name = match.group(1)
        const_name = match.group(2)
        cmds_raw = match.group(3)
        cmds = [
            c.strip()
            for c in cmds_raw.split(',')
            if c.strip() and not c.strip().startswith('//')
        ]
        handler_to_cmds[handler_name] = set(cmds)
        const_to_cmds[const_name] = set(cmds)

    # 2. Parse plugins in build.rs
    build_file = "apps/app/build.rs"
    with open(build_file, "r", encoding="utf-8") as f:
        build_content = f.read()

    build_plugins = {}
    # Pattern for commands::CONST_NAME
    plugin_pattern_const = re.compile(
        r'\.plugin\s*\(\s*"([^"]+)"\s*,\s*InlinedPlugin::new\(\)\s*\.commands\(commands::([a-zA-Z0-9_]+)\)',
        re.DOTALL
    )
    for match in plugin_pattern_const.finditer(build_content):
        plugin_name = match.group(1)
        const_name = match.group(2)
        build_plugins[plugin_name] = const_to_cmds.get(const_name, set())

    # AppManifest pattern
    app_manifest_pattern = re.compile(
        r'AppManifest::new\(\)\s*\.commands\(commands::([a-zA-Z0-9_]+)\)',
        re.DOTALL
    )
    app_match = app_manifest_pattern.search(build_content)
    if app_match:
        build_plugins["app"] = const_to_cmds.get(app_match.group(1), set())

    # Fallback pattern for inline &[...]
    plugin_pattern_inline = re.compile(
        r'\.plugin\s*\(\s*"([^"]+)",\s*InlinedPlugin::new\(\)\s*\.commands\(&\[(.*?)\]\)',
        re.DOTALL
    )
    for match in plugin_pattern_inline.finditer(build_content):
        plugin_name = match.group(1)
        commands_raw = match.group(2)
        cmds = [c.strip().strip('"') for c in commands_raw.split(',') if c.strip().strip('"')]
        build_plugins[plugin_name] = set(cmds)

    # 3. Parse invoke handlers in apps/app/src/api
    api_files = glob.glob("apps/app/src/api/**/*.rs", recursive=True)
    api_plugins = {}
    for path in api_files:
        with open(path, "r", encoding="utf-8") as f:
            content = f.read()

        # Check macro handler calls: crate::commands::<handler>!()
        macro_handler_pattern = re.compile(
            r'Builder::(?:<[^>]+>::)?new\("([^"]+)"\).*?invoke_handler\s*\(\s*crate::commands::([a-zA-Z0-9_]+)!\(\)',
            re.DOTALL
        )
        for match in macro_handler_pattern.finditer(content):
            plugin_name = match.group(1)
            handler_name = match.group(2)
            if handler_name in handler_to_cmds:
                api_plugins[plugin_name] = handler_to_cmds[handler_name]

        # Check inline generate_handler![...]
        inline_handler_pattern = re.compile(
            r'Builder::(?:<[^>]+>::)?new\("([^"]+)"\).*?invoke_handler\s*\(\s*tauri::generate_handler!\[(.*?)\]',
            re.DOTALL
        )
        for match in inline_handler_pattern.finditer(content):
            plugin_name = match.group(1)
            cmds_raw = match.group(2)
            cmds_clean = re.sub(r'//.*', '', cmds_raw)
            cmds = []
            for c in cmds_clean.split(','):
                c = c.strip()
                if not c:
                    continue
                if '::' in c:
                    c = c.split('::')[-1]
                cmds.append(c)
            api_plugins[plugin_name] = set(cmds)

    # 4. Parse capabilities
    cap_plugins = set()
    for cap_path in glob.glob("apps/app/capabilities/*.json"):
        if os.path.basename(cap_path) == "devtools.json":
            continue
        with open(cap_path, "r", encoding="utf-8") as f:
            cap_data = json.load(f)
        for p in cap_data.get("permissions", []):
            if isinstance(p, str) and ":" in p:
                prefix, rule = p.split(":", 1)
                if rule == "default":
                    cap_plugins.add(prefix)

    errors = []

    # Check 1: All handler plugins are present in build.rs and commands match
    for plugin, cmds in sorted(api_plugins.items()):
        if plugin not in build_plugins:
            errors.append(f"Plugin '{plugin}' is registered in API handler but missing from build.rs!")
            continue

        missing_cmds = cmds - build_plugins[plugin]
        if missing_cmds:
            errors.append(f"Plugin '{plugin}' is missing commands in build.rs: {sorted(list(missing_cmds))}")

        # Check 2: All handler plugins are allowed in capabilities
        if plugin not in cap_plugins:
            errors.append(f"Plugin '{plugin}' is registered but missing '{plugin}:default' in capabilities/plugins.json!")

    # Check 3: Every command has a corresponding permission in permissions/<plugin>.toml
    for plugin, cmds in sorted(build_plugins.items()):
        toml_path = f"apps/app/permissions/{plugin}.toml"
        if not os.path.exists(toml_path):
            errors.append(f"Missing permissions file: {toml_path}")
            continue
        with open(toml_path, "r", encoding="utf-8") as f:
            toml_content = f.read()
        for cmd in cmds:
            slug = cmd.replace("_", "-")
            if f'"{slug}"' not in toml_content and f'"allow-{slug}"' not in toml_content and f'"allow-{cmd}"' not in toml_content:
                errors.append(f"Plugin '{plugin}' command '{cmd}' missing permission entry in {toml_path}")

    # Check 4: Audit all frontend invoke calls
    fe_invokes = set()
    fe_pattern = re.compile(r'["\']plugin:([a-zA-Z0-9_-]+)\|([a-zA-Z0-9_]+)["\']')
    for dirpath, _, filenames in os.walk("apps/app-frontend/src"):
        for file in filenames:
            if file.endswith(('.ts', '.js', '.vue')):
                with open(os.path.join(dirpath, file), "r", encoding="utf-8") as f:
                    content = f.read()
                for match in fe_pattern.finditer(content):
                    fe_invokes.add((match.group(1), match.group(2)))

    for plugin, cmd in sorted(fe_invokes):
        if plugin not in api_plugins or cmd not in api_plugins[plugin]:
            errors.append(f"Frontend calls unregistered invoke 'plugin:{plugin}|{cmd}'!")
        if plugin not in cap_plugins:
            errors.append(f"Frontend calls invoke 'plugin:{plugin}|{cmd}' without capability '{plugin}:default'!")

    if errors:
        print("=== ACL COMMAND VERIFICATION FAILED ===", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        sys.exit(1)

    total_cmds = sum(len(c) for c in api_plugins.values())
    print(f"SUCCESS: {len(api_plugins)} plugins, {total_cmds} backend commands, and {len(fe_invokes)} frontend invoke calls verified with 100% ACL parity.")

if __name__ == "__main__":
    main()
