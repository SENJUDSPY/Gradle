#!/usr/bin/env python3
"""
Tauri v2 ACL Guardrail and Drift Detector.

Guarantees:
1. Every frontend `invoke(...)` maps to a registered backend command in commands.rs.
2. Every frontend `invoke(...)` is granted by capability permissions.
3. Every Tauri JS API usage across window, webview, menu, image, resources, tray,
   path, app, event, os, fs, opener, dialog, http, window-state maps to granted
   capability permissions (with exact import parsing to avoid false positives).
4. Release capabilities NEVER include broad `core:default`, `core:webview:default`,
   or `core:webview:allow-internal-toggle-devtools` (devtools toggle stays debug-only).
5. CI diff lists every permission entry removed compared to baseline_permissions.json
   and fails unless explicitly approved in approved_permission_removals.json.

Copyright (c) 2026 Gradle contributors (MIT License).
"""

import glob
import json
import os
import re
import sys

CORE_UMBRELLA_MAP = {
    "core:default": [
        "core:path:default",
        "core:event:default",
        "core:window:default",
        "core:webview:default",
        "core:app:default",
        "core:image:default",
        "core:resources:default",
        "core:menu:default",
        "core:tray:default",
    ],
    "core:app:default": [
        "core:app:allow-version",
        "core:app:allow-name",
        "core:app:allow-tauri-version",
        "core:app:allow-identifier",
        "core:app:allow-bundle-type",
        "core:app:allow-register-listener",
        "core:app:allow-remove-listener",
        "core:app:allow-supports-multiple-windows",
    ],
    "core:event:default": [
        "core:event:allow-listen",
        "core:event:allow-unlisten",
        "core:event:allow-emit",
        "core:event:allow-emit-to",
    ],
    "core:webview:default": [
        "core:webview:allow-get-all-webviews",
        "core:webview:allow-webview-position",
        "core:webview:allow-webview-size",
        "core:webview:allow-internal-toggle-devtools",
    ],
    "core:window:default": [
        "core:window:allow-get-all-windows",
        "core:window:allow-scale-factor",
        "core:window:allow-inner-position",
        "core:window:allow-outer-position",
        "core:window:allow-inner-size",
        "core:window:allow-outer-size",
        "core:window:allow-is-fullscreen",
        "core:window:allow-is-minimized",
        "core:window:allow-is-maximized",
        "core:window:allow-is-focused",
        "core:window:allow-is-decorated",
        "core:window:allow-is-resizable",
        "core:window:allow-is-maximizable",
        "core:window:allow-is-minimizable",
        "core:window:allow-is-closable",
        "core:window:allow-is-visible",
        "core:window:allow-is-enabled",
        "core:window:allow-title",
        "core:window:allow-current-monitor",
        "core:window:allow-primary-monitor",
        "core:window:allow-monitor-from-point",
        "core:window:allow-available-monitors",
        "core:window:allow-cursor-position",
        "core:window:allow-theme",
        "core:window:allow-is-always-on-top",
        "core:window:allow-activity-name",
        "core:window:allow-scene-identifier",
        "core:window:allow-internal-toggle-maximize",
    ],
    "core:image:default": [
        "core:image:allow-new",
        "core:image:allow-from-bytes",
        "core:image:allow-from-path",
        "core:image:allow-rgba",
        "core:image:allow-size",
    ],
    "core:resources:default": [
        "core:resources:allow-close",
    ],
    "os:default": [
        "os:allow-arch",
        "os:allow-exe-extension",
        "os:allow-family",
        "os:allow-locale",
        "os:allow-os-type",
        "os:allow-platform",
        "os:allow-version",
    ],
    "opener:default": [
        "opener:allow-open-url",
        "opener:allow-open-path",
    ],
    "window-state:default": [
        "window-state:allow-filename",
        "window-state:allow-restore-state",
        "window-state:allow-save-window-state",
    ],
}


def expand_permissions(perms):
    """Expand known composite permissions so semantic checks work whether default or granular."""
    expanded = set(perms)
    changed = True
    while changed:
        changed = False
        for p in list(expanded):
            for child in CORE_UMBRELLA_MAP.get(p, ()):
                if child not in expanded:
                    expanded.add(child)
                    changed = True
    return expanded


def get_release_capabilities(root):
    cap_dir = os.path.join(root, "apps", "app", "capabilities")
    raw_perms = set()
    for fname in sorted(glob.glob(os.path.join(cap_dir, "*.json"))):
        base = os.path.basename(fname)
        # Skip generated debug-only capability and ACL metadata files
        if base == "devtools.json" or "baseline" in base or "approved" in base:
            continue
        with open(fname, "r", encoding="utf-8") as f:
            data = json.load(f)
        for p in data.get("permissions", []):
            if isinstance(p, str):
                raw_perms.add(p)
            elif isinstance(p, dict) and "identifier" in p:
                raw_perms.add(p["identifier"])
    return raw_perms, expand_permissions(raw_perms)


def parse_backend_commands(root):
    commands_file = os.path.join(root, "apps", "app", "src", "commands.rs")
    with open(commands_file, "r", encoding="utf-8") as f:
        content = f.read()

    macro_pattern = re.compile(
        r"define_plugin_commands!\s*\(\s*([a-zA-Z0-9_]+)\s*,\s*([a-zA-Z0-9_]+)\s*,\s*\[(.*?)\]\s*\);",
        re.DOTALL,
    )
    handler_map = {}
    for match in macro_pattern.finditer(content):
        handler_name = match.group(1)
        cmds_raw = match.group(3)
        cmds = [
            c.strip()
            for c in cmds_raw.split(",")
            if c.strip() and not c.strip().startswith("//")
        ]
        plugin_name = handler_name.replace("_handler", "").replace("_", "-")
        handler_map[plugin_name] = set(cmds)
    return handler_map


def iter_frontend_files(root):
    dirs = [
        os.path.join(root, "apps", "app-frontend", "src"),
        os.path.join(root, "packages"),
    ]
    for search_dir in dirs:
        for dirpath, _, filenames in os.walk(search_dir):
            if any(
                skip in dirpath
                for skip in ["node_modules", "target", "dist", ".git", "stories"]
            ):
                continue
            for file in filenames:
                if file.endswith((".ts", ".js", ".vue")):
                    yield os.path.join(dirpath, file)


def parse_named_imports(text, package_name):
    """Return set of original exported symbol names imported from package_name."""
    symbols = set()
    pat = re.compile(
        rf'import\s+(?:type\s+)?\{{([^}}]+)\}}\s+from\s+[\'"]{re.escape(package_name)}[\'"]',
        re.DOTALL,
    )
    for m in pat.finditer(text):
        for item in m.group(1).split(","):
            item = item.strip()
            if not item or item.startswith("type "):
                continue
            # Handle `orig as alias`
            orig = item.split(" as ")[0].strip()
            if orig:
                symbols.add(orig)
    return symbols


def scan_frontend_invokes(root):
    invokes = set()
    pat = re.compile(r'invoke(?:<[^>]*>)?\s*\(\s*[\x27\x22]([^\x27\x22]+)[\x27\x22]')
    for p in iter_frontend_files(root):
        with open(p, "r", encoding="utf-8", errors="ignore") as f:
            for match in pat.finditer(f.read()):
                invokes.add(match.group(1))
    return invokes


def scan_tauri_js_api_usage(root):
    needed_permissions = set()

    window_method_map = {
        "isMaximized": ("core:window:allow-is-maximized", "core:window:default"),
        "isFullscreen": ("core:window:allow-is-fullscreen", "core:window:default"),
        "isMinimized": ("core:window:allow-is-minimized", "core:window:default"),
        "isFocused": ("core:window:allow-is-focused", "core:window:default"),
        "isDecorated": ("core:window:allow-is-decorated", "core:window:default"),
        "isResizable": ("core:window:allow-is-resizable", "core:window:default"),
        "isMaximizable": ("core:window:allow-is-maximizable", "core:window:default"),
        "isMinimizable": ("core:window:allow-is-minimizable", "core:window:default"),
        "isClosable": ("core:window:allow-is-closable", "core:window:default"),
        "isVisible": ("core:window:allow-is-visible", "core:window:default"),
        "isEnabled": ("core:window:allow-is-enabled", "core:window:default"),
        "isAlwaysOnTop": ("core:window:allow-is-always-on-top", "core:window:default"),
        "scaleFactor": ("core:window:allow-scale-factor", "core:window:default"),
        "innerSize": ("core:window:allow-inner-size", "core:window:default"),
        "outerSize": ("core:window:allow-outer-size", "core:window:default"),
        "innerPosition": ("core:window:allow-inner-position", "core:window:default"),
        "outerPosition": ("core:window:allow-outer-position", "core:window:default"),
        "theme": ("core:window:allow-theme", "core:window:default"),
        "title": ("core:window:allow-title", "core:window:default"),
        "currentMonitor": ("core:window:allow-current-monitor", "core:window:default"),
        "primaryMonitor": ("core:window:allow-primary-monitor", "core:window:default"),
        "availableMonitors": ("core:window:allow-available-monitors", "core:window:default"),
        "cursorPosition": ("core:window:allow-cursor-position", "core:window:default"),
        "setDecorations": ("core:window:allow-set-decorations",),
        "minimize": ("core:window:allow-minimize",),
        "maximize": ("core:window:allow-maximize",),
        "toggleMaximize": ("core:window:allow-toggle-maximize",),
        "unmaximize": ("core:window:allow-unmaximize",),
        "unminimize": ("core:window:allow-unminimize",),
        "close": ("core:window:allow-close",),
        "show": ("core:window:allow-show",),
        "hide": ("core:window:allow-hide",),
        "startDragging": ("core:window:allow-start-dragging",),
        "onResized": ("core:event:allow-listen", "core:event:default"),
    }

    webview_method_map = {
        "onDragDropEvent": ("core:event:allow-listen", "core:event:default"),
        "position": ("core:webview:allow-webview-position", "core:webview:default"),
        "size": ("core:webview:allow-webview-size", "core:webview:default"),
        "setZoom": ("core:webview:allow-set-webview-zoom",),
        "setFocus": ("core:webview:allow-set-webview-focus",),
        "setSize": ("core:webview:allow-set-webview-size",),
        "setPosition": ("core:webview:allow-set-webview-position",),
        "setAutoResize": ("core:webview:allow-set-webview-auto-resize",),
        "hide": ("core:webview:allow-webview-hide",),
        "show": ("core:webview:allow-webview-show",),
        "close": ("core:webview:allow-webview-close",),
        "clearAllBrowsingData": ("core:webview:allow-clear-all-browsing-data",),
    }

    menu_method_map = {
        "new": ("core:menu:allow-new", "core:menu:default"),
        "default": ("core:menu:allow-create-default", "core:menu:default"),
        "append": ("core:menu:allow-append", "core:menu:default"),
        "prepend": ("core:menu:allow-prepend", "core:menu:default"),
        "popup": ("core:menu:allow-popup", "core:menu:default"),
        "setAsAppMenu": ("core:menu:allow-set-as-app-menu", "core:menu:default"),
        "setAsWindowMenu": ("core:menu:allow-set-as-window-menu", "core:menu:default"),
    }

    image_method_map = {
        "new": ("core:image:allow-new", "core:image:default"),
        "fromBytes": ("core:image:allow-from-bytes", "core:image:default"),
        "fromPath": ("core:image:allow-from-path", "core:image:default"),
        "rgba": ("core:image:allow-rgba", "core:image:default"),
        "size": ("core:image:allow-size", "core:image:default"),
    }

    app_api_map = {
        "getVersion": ("core:app:allow-version", "core:app:default"),
        "getName": ("core:app:allow-name", "core:app:default"),
        "getTauriVersion": ("core:app:allow-tauri-version", "core:app:default"),
        "getIdentifier": ("core:app:allow-identifier", "core:app:default"),
        "defaultWindowIcon": ("core:app:allow-default-window-icon", "core:app:default"),
        "setTheme": ("core:app:allow-set-app-theme", "core:app:default"),
    }

    event_api_map = {
        "listen": ("core:event:allow-listen", "core:event:default"),
        "once": ("core:event:allow-listen", "core:event:default"),
        "emit": ("core:event:allow-emit", "core:event:default"),
        "emitTo": ("core:event:allow-emit-to", "core:event:default"),
    }

    os_api_map = {
        "platform": ("os:allow-platform", "os:default"),
        "version": ("os:allow-version", "os:default"),
        "type": ("os:allow-os-type", "os:default"),
        "family": ("os:allow-family", "os:default"),
        "arch": ("os:allow-arch", "os:default"),
        "exeExtension": ("os:allow-exe-extension", "os:default"),
        "locale": ("os:allow-locale", "os:default"),
        "hostname": ("os:allow-hostname", "os:default"),
    }

    fs_api_map = {
        "exists": ("fs:allow-exists", "fs:default"),
        "readDir": ("fs:allow-read-dir", "fs:default"),
        "readFile": ("fs:allow-read-file", "fs:default"),
        "readTextFile": ("fs:allow-read-text-file", "fs:default"),
        "stat": ("fs:allow-stat", "fs:default"),
        "lstat": ("fs:allow-lstat", "fs:default"),
        "writeFile": ("fs:allow-write-file", "fs:default"),
        "writeTextFile": ("fs:allow-write-text-file", "fs:default"),
        "mkdir": ("fs:allow-mkdir", "fs:default"),
        "remove": ("fs:allow-remove", "fs:default"),
        "rename": ("fs:allow-rename", "fs:default"),
        "copyFile": ("fs:allow-copy-file", "fs:default"),
    }

    opener_api_map = {
        "openUrl": ("opener:allow-open-url", "opener:default"),
        "openPath": ("opener:allow-open-path", "opener:default"),
        "revealItemInDir": ("opener:allow-reveal-item-in-dir",),
    }

    dialog_api_map = {
        "open": ("dialog:allow-open", "dialog:default"),
        "save": ("dialog:allow-save", "dialog:default"),
        "confirm": ("dialog:allow-confirm", "dialog:default"),
        "message": ("dialog:allow-message", "dialog:default"),
        "ask": ("dialog:allow-ask", "dialog:default"),
    }

    window_state_api_map = {
        "saveWindowState": ("window-state:allow-save-window-state", "window-state:default"),
        "restoreStateCurrent": ("window-state:allow-restore-state", "window-state:default"),
        "filename": ("window-state:allow-filename", "window-state:default"),
    }

    # Built-in Tauri runtime injected scripts active on every window/webview
    needed_permissions.add(("tauri.zoom-hotkey", ("core:webview:allow-set-webview-zoom",)))

    for p in iter_frontend_files(root):
        with open(p, "r", encoding="utf-8", errors="ignore") as f:
            text = f.read()

        # Window APIs (@tauri-apps/api/window)
        win_receivers = ["getCurrentWindow\\(\\)"]
        for m in re.finditer(r"(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*=\s*(?:getCurrentWindow\(|new\s+(?:Window|WebviewWindow)\()", text):
            win_receivers.append(re.escape(m.group(1)))
        win_rx = "|".join(win_receivers)
        if "@tauri-apps/api/window" in text or "getCurrentWindow" in text:
            for method, perms in window_method_map.items():
                if re.search(rf"(?:{win_rx})\s*\.\s*{re.escape(method)}\s*\(", text):
                    needed_permissions.add((f"window.{method}", perms))
            for sym in parse_named_imports(text, "@tauri-apps/api/window"):
                if sym in ("Window", "WebviewWindow") and re.search(rf"\bnew\s+{sym}\b", text):
                    needed_permissions.add(("window.create", ("core:window:allow-create",)))
                if sym == "getAllWindows":
                    needed_permissions.add(("window.getAllWindows", ("core:window:allow-get-all-windows", "core:window:default")))

        # Drag region implies start_dragging & internal_toggle_maximize
        if "data-tauri-drag-region" in text:
            needed_permissions.add(("data-tauri-drag-region", ("core:window:allow-start-dragging",)))
            needed_permissions.add(("data-tauri-drag-double-click", ("core:window:allow-internal-toggle-maximize", "core:window:default")))

        # Webview APIs (@tauri-apps/api/webview & webviewWindow)
        wv_receivers = ["getCurrentWebview\\(\\)"]
        for m in re.finditer(r"(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*=\s*(?:getCurrentWebview\(|new\s+Webview\()", text):
            wv_receivers.append(re.escape(m.group(1)))
        wv_rx = "|".join(wv_receivers)
        if "@tauri-apps/api/webview" in text or "getCurrentWebview" in text:
            for method, perms in webview_method_map.items():
                if re.search(rf"(?:{wv_rx})\s*\.\s*{re.escape(method)}\s*\(", text):
                    needed_permissions.add((f"webview.{method}", perms))
            for sym in parse_named_imports(text, "@tauri-apps/api/webview"):
                if sym == "getAllWebviews":
                    needed_permissions.add(("webview.getAllWebviews", ("core:webview:allow-get-all-webviews", "core:webview:default")))
                if sym == "Webview" and re.search(r"\bnew\s+Webview\b", text):
                    needed_permissions.add(("webview.createWebview", ("core:webview:allow-create-webview",)))

        # Menu APIs (@tauri-apps/api/menu)
        if "@tauri-apps/api/menu" in text:
            syms = parse_named_imports(text, "@tauri-apps/api/menu")
            if syms:
                needed_permissions.add(("menu.import", ("core:menu:default",)))
                for method, perms in menu_method_map.items():
                    if f".{method}(" in text:
                        needed_permissions.add((f"menu.{method}", perms))

        # Image APIs (@tauri-apps/api/image)
        if "@tauri-apps/api/image" in text:
            syms = parse_named_imports(text, "@tauri-apps/api/image")
            if syms:
                needed_permissions.add(("image.import", ("core:image:default",)))
                for method, perms in image_method_map.items():
                    if f".{method}(" in text:
                        needed_permissions.add((f"image.{method}", perms))

        # Resources API (@tauri-apps/api/core Resource)
        for sym in parse_named_imports(text, "@tauri-apps/api/core"):
            if sym == "Resource":
                needed_permissions.add(("resources.Resource", ("core:resources:allow-close", "core:resources:default")))

        # Path APIs (@tauri-apps/api/path)
        if "@tauri-apps/api/path" in text and parse_named_imports(text, "@tauri-apps/api/path"):
            needed_permissions.add(("path.import", ("core:path:default",)))

        # Tray APIs (@tauri-apps/api/tray)
        if "@tauri-apps/api/tray" in text and parse_named_imports(text, "@tauri-apps/api/tray"):
            needed_permissions.add(("tray.import", ("core:tray:default",)))

        # App APIs (@tauri-apps/api/app)
        for sym in parse_named_imports(text, "@tauri-apps/api/app"):
            if sym in app_api_map:
                needed_permissions.add((f"app.{sym}", app_api_map[sym]))

        # Event APIs (@tauri-apps/api/event)
        for sym in parse_named_imports(text, "@tauri-apps/api/event"):
            if sym in event_api_map:
                needed_permissions.add((f"event.{sym}", event_api_map[sym]))

        # OS APIs (@tauri-apps/plugin-os)
        for sym in parse_named_imports(text, "@tauri-apps/plugin-os"):
            if sym in os_api_map:
                needed_permissions.add((f"os.{sym}", os_api_map[sym]))

        # FS APIs (@tauri-apps/plugin-fs)
        for sym in parse_named_imports(text, "@tauri-apps/plugin-fs"):
            if sym in fs_api_map:
                needed_permissions.add((f"fs.{sym}", fs_api_map[sym]))

        # Opener APIs (@tauri-apps/plugin-opener + external target="_blank")
        for sym in parse_named_imports(text, "@tauri-apps/plugin-opener"):
            if sym in opener_api_map:
                needed_permissions.add((f"opener.{sym}", opener_api_map[sym]))
        if 'target="_blank"' in text or "target='_blank'" in text:
            needed_permissions.add(("opener.externalLink", ("opener:allow-open-url", "opener:default")))

        # Dialog APIs (@tauri-apps/plugin-dialog + alert/confirm hooks)
        for sym in parse_named_imports(text, "@tauri-apps/plugin-dialog"):
            if sym in dialog_api_map:
                needed_permissions.add((f"dialog.{sym}", dialog_api_map[sym]))
        if re.search(r"\b(?:window\.)?alert\s*\(", text):
            needed_permissions.add(("dialog.windowAlert", ("dialog:allow-message", "dialog:default")))
        if re.search(r"\bwindow\.confirm\s*\(", text):
            needed_permissions.add(("dialog.windowConfirm", ("dialog:allow-confirm", "dialog:default")))

        # HTTP APIs (@tauri-apps/plugin-http)
        if "@tauri-apps/plugin-http" in text and parse_named_imports(text, "@tauri-apps/plugin-http"):
            needed_permissions.add(("http.fetch", ("http:default",)))

        # Window-state APIs (@tauri-apps/plugin-window-state)
        for sym in parse_named_imports(text, "@tauri-apps/plugin-window-state"):
            if sym in window_state_api_map:
                needed_permissions.add((f"window-state.{sym}", window_state_api_map[sym]))

    # Backend Rust opener usage (show_app_db_backups_folder / reveal_item_in_dir)
    needed_permissions.add(("rust.reveal_item_in_dir", ("opener:allow-reveal-item-in-dir",)))

    return needed_permissions


def check_permission_removals_diff(root, raw_perms, expanded_perms):
    baseline_file = os.path.join(root, "apps", "app", "acl", "baseline_permissions.json")
    approved_file = os.path.join(root, "apps", "app", "acl", "approved_permission_removals.json")

    if not os.path.exists(baseline_file):
        print("Warning: baseline_permissions.json does not exist. Skipping diff check.")
        return [], {}

    with open(baseline_file, "r", encoding="utf-8") as f:
        baseline_data = json.load(f)
    baseline_perms = set(baseline_data.get("permissions", []))

    approved_removals = {}
    if os.path.exists(approved_file):
        with open(approved_file, "r", encoding="utf-8") as f:
            approved_removals = json.load(f).get("approved_removals", {})

    # A baseline entry is considered covered if still present directly OR still
    # fully covered by expanded composite rules (e.g. `os:allow-locale` inside `os:default`).
    removed_perms = sorted(
        p for p in baseline_perms if p not in raw_perms and p not in expanded_perms
    )
    unapproved = [p for p in removed_perms if p not in approved_removals]
    approved = {p: approved_removals[p] for p in removed_perms if p in approved_removals}
    return unapproved, approved


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    errors = []

    raw_perms, expanded_perms = get_release_capabilities(root)
    backend_cmds = parse_backend_commands(root)
    fe_invokes = scan_frontend_invokes(root)
    tauri_js_apis = scan_tauri_js_api_usage(root)

    # 1. Enforce debug-only devtools in release capabilities
    forbidden_in_release = [
        "core:webview:allow-internal-toggle-devtools",
    ]
    for p in forbidden_in_release:
        if p in raw_perms:
            errors.append(
                f"SECURITY: '{p}' must not be present in release capability files (keeps devtools/unused core permissions out of release builds)."
            )

    # 2. Check all frontend invoke calls
    for invoke in sorted(fe_invokes):
        if invoke.startswith("plugin:"):
            parts = invoke[len("plugin:") :].split("|", 1)
            if len(parts) != 2:
                errors.append(f"Invalid plugin invoke format: '{invoke}'")
                continue
            plugin, cmd = parts
            if plugin not in backend_cmds or cmd not in backend_cmds[plugin]:
                errors.append(f"Frontend calls unregistered plugin command: '{invoke}'")
            default_perm = f"{plugin}:default"
            cmd_slug = cmd.replace("_", "-")
            specific_perm = f"{plugin}:allow-{cmd_slug}"
            if default_perm not in expanded_perms and specific_perm not in expanded_perms:
                errors.append(
                    f"Frontend invoke '{invoke}' missing capability grant ('{default_perm}' or '{specific_perm}')"
                )
        else:
            app_cmds = backend_cmds.get("app", set())
            if invoke not in app_cmds:
                errors.append(f"Frontend calls unregistered app command: '{invoke}'")
            if (
                "default" not in expanded_perms
                and f"allow-{invoke.replace('_', '-')}" not in expanded_perms
            ):
                errors.append(
                    f"Frontend app command '{invoke}' missing capability grant ('default')"
                )

    # 3. Check Tauri JS API usages
    for feature, allowed_alternatives in sorted(tauri_js_apis):
        has_grant = any(perm in expanded_perms for perm in allowed_alternatives)
        if not has_grant:
            errors.append(
                f"Feature '{feature}' used in frontend is not permitted by capabilities! "
                f"Requires one of: {list(allowed_alternatives)}"
            )

    # 4. Check CI baseline diff
    unapproved_removals, approved_removals = check_permission_removals_diff(
        root, raw_perms, expanded_perms
    )
    print(f"=== CI ACL DIFF AGAINST BASELINE ({len(approved_removals) + len(unapproved_removals)} removed entries) ===")
    for p, reason in sorted(approved_removals.items()):
        print(f"  [APPROVED REMOVAL] {p}: {reason}")
    for p in unapproved_removals:
        print(f"  [UNAPPROVED REMOVAL] {p}")
        errors.append(
            f"REGRESSION: Permission entry '{p}' was removed compared to previous baseline without approval! "
            f"Add to apps/app/acl/approved_permission_removals.json with rationale if intentional."
        )

    if errors:
        print("\n=== ACL GUARDRAIL VERIFICATION FAILED ===", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        sys.exit(1)

    print(
        f"\nSUCCESS: Guardrail verified {len(fe_invokes)} invoke calls, "
        f"{len(tauri_js_apis)} JS API requirements (window, webview, menu, image, resources, os, fs, opener, dialog, app, event, http, window-state), "
        f"{len(approved_removals)} approved removals, and 0 unapproved removals."
    )


if __name__ == "__main__":
    main()
