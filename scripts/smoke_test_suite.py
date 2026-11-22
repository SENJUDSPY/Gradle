#!/usr/bin/env python3
"""
Comprehensive Smoke Test Suite for Gradle Launcher.

Covers every screen and control workflow:
1. Guardrails & CI Diff: verify_acl_commands.py + verify_acl_guardrail.py
2. Real Tauri RuntimeAuthority ACL verification (all startup, home, library,
   instance/content, hosting, settings, window controls, open backups folder,
   Rust Edition setup/play commands + devtools debug-only enforcement)
3. Startup & Window Controls: isMaximized, isFullscreen, setDecorations,
   minimize, maximize, toggleMaximize, internal_toggle_maximize, startDragging,
   close, saveWindowState, set_webview_zoom
4. Home, Library, Instance Page, Content Tab, Hosting & Settings permissions
5. Settings & "Open backups folder": utils + opener (open_url, reveal_item_in_dir)
6. DB Migration Compatibility + Rust Edition Setup & Play (cargo tests with live logs)

Copyright (c) 2026 Gradle contributors (MIT License).
"""

import json
import os
import subprocess
import sys


def log_section(title):
    print(f"\n{'='*20} {title} {'='*20}")


def run_step(step_name, fn):
    print(f"[*] Running: {step_name} ...", end=" ", flush=True)
    try:
        res = fn()
        print("PASSED")
        if res:
            print(f"    -> {res}")
        return res
    except Exception as e:
        print("FAILED")
        print(f"    Error: {e}", file=sys.stderr)
        raise


def build_cargo_env():
    env = os.environ.copy()
    env["JAVA_HOME"] = "/home/godxzailas/.local/share/jdk-21"
    env["PATH"] = f"{env['JAVA_HOME']}/bin:/home/godxzailas/.local/bin:" + env.get("PATH", "")
    sysroot_pc = (
        "/home/godxzailas/.local/sysroot/usr/lib/x86_64-linux-gnu/pkgconfig:"
        "/home/godxzailas/.local/sysroot/usr/share/pkgconfig:"
        "/home/godxzailas/.local/sysroot/usr/lib/pkgconfig"
    )
    env["PKG_CONFIG_PATH"] = sysroot_pc + (":" + env["PKG_CONFIG_PATH"] if env.get("PKG_CONFIG_PATH") else "")
    env["PKG_CONFIG_SYSROOT_DIR"] = "/home/godxzailas/.local/sysroot"
    sysroot_lib = "/home/godxzailas/.local/sysroot/usr/lib/x86_64-linux-gnu"
    env["LIBRARY_PATH"] = sysroot_lib + (":" + env["LIBRARY_PATH"] if env.get("LIBRARY_PATH") else "")
    env["LD_LIBRARY_PATH"] = sysroot_lib + (":" + env["LD_LIBRARY_PATH"] if env.get("LD_LIBRARY_PATH") else "")
    return env


def test_guardrails(root):
    for script in ["verify_acl_commands.py", "verify_acl_guardrail.py"]:
        res = subprocess.run(
            ["python3", os.path.join(root, "scripts", script)],
            cwd=root,
            capture_output=True,
            text=True,
        )
        assert res.returncode == 0, f"{script} failed:\n{res.stderr}\n{res.stdout}"
    return "ACL parity & JS API guardrails + CI baseline diff passed."


def test_runtime_authority_rust(root):
    cmd = [
        "cargo", "test", "-p", "theseus_gui", "--bin", "theseus_gui",
        "acl_tests", "--", "--nocapture",
    ]
    res = subprocess.run(cmd, cwd=root, capture_output=True, text=True, env=build_cargo_env())
    assert res.returncode == 0, f"Rust acl_tests failed:\n{res.stderr}\n{res.stdout}"
    assert "3 passed; 0 failed" in res.stdout, f"Unexpected test summary:\n{res.stdout}"
    return "Real Tauri RuntimeAuthority allowed all required commands & pruned unused core/devtools-in-release."


def test_startup_and_window(root):
    main_file = os.path.join(root, "apps", "app", "capabilities", "main.json")
    with open(main_file, "r", encoding="utf-8") as f:
        main_cap = json.load(f)
    perms = {p if isinstance(p, str) else p.get("identifier") for p in main_cap.get("permissions", [])}

    assert "core:default" in perms, "Missing required core:default in main.json"

    required_window_perms = [
        "core:default",
        "core:window:allow-internal-toggle-maximize",
        "core:window:allow-start-dragging",
        "core:window:allow-close",
        "core:window:allow-minimize",
        "core:window:allow-maximize",
        "core:window:allow-toggle-maximize",
        "core:window:allow-unmaximize",
        "core:window:allow-set-decorations",
        "core:webview:allow-set-webview-zoom",
    ]
    for p in required_window_perms:
        assert p in perms, f"Missing required window/core permission in main.json: {p}"
    return "Window & core capabilities verified (drag, double-click, minimize, maximize, close, zoom)."


def test_home_library_hosting_content(root):
    main_file = os.path.join(root, "apps", "app", "capabilities", "main.json")
    with open(main_file, "r", encoding="utf-8") as f:
        main_cap = json.load(f)
    perms = {p if isinstance(p, str) else p.get("identifier") for p in main_cap.get("permissions", [])}

    required = [
        "cache:default",
        "instance:default",
        "tags:default",
        "p2p:default",
        "worlds:default",
        "horizon:default",
        "files:default",
        "fs:allow-read-file",
        "fs:scope",
        "dialog:allow-open",
        "dialog:allow-save",
        "dialog:allow-message",
        "settings:default",
        "utils:default",
        "opener:default",
        "opener:allow-reveal-item-in-dir",
        "os:default",
        "window-state:allow-save-window-state",
        "fs:allow-exists",
        "allow-instance-content",
        "allow-files-save-external",
    ]
    for p in required:
        assert p in perms, f"Missing required capability in main.json: {p}"

    app_toml = os.path.join(root, "apps", "app", "permissions", "app.toml")
    with open(app_toml, "r", encoding="utf-8") as f:
        content = f.read()
    assert "allow-instance-content" in content, "instance_content missing from app.toml"
    assert "allow-files-save-external" in content, "files_save_external missing from app.toml"
    return "Home, Library, Instance Content, Hosting, Settings & Open Backups Folder capabilities verified."


def test_rust_edition_and_db_smoke(root):
    cmd = [
        "cargo", "test", "-p", "theseus", "--lib",
        "state::db::tests", "--", "--nocapture",
    ]
    res = subprocess.run(cmd, cwd=root, capture_output=True, text=True, env=build_cargo_env())
    assert res.returncode == 0, f"DB & Rust Edition tests failed:\n{res.stderr}\n{res.stdout}"
    assert "[Gradle Rust Edition] Initializing Horizon runtime..." in res.stdout
    assert "[Gradle Rust Edition] Game loop ready. FPS: 60." in res.stdout
    return "Migrations, DB compatibility, and Rust Edition setup + 60 FPS loop verified."


def main():
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    log_section("GRADLE LAUNCHER ACL & SCREEN SMOKE TEST SUITE")

    run_step("1. Frontend Invoke & JS API Guardrails + CI Diff", lambda: test_guardrails(root))
    run_step("2. Tauri RuntimeAuthority ACL Matrix (Rust)", lambda: test_runtime_authority_rust(root))
    run_step("3. Startup & Window Controls (drag, dbl-click, min/max/close, zoom)", lambda: test_startup_and_window(root))
    run_step("4. Home, Library, Instance Content, Hosting, Settings, Open Backups Folder", lambda: test_home_library_hosting_content(root))
    run_step("5. DB Migrations, Rust Edition Setup & Play (live loop log)", lambda: test_rust_edition_and_db_smoke(root))

    log_section("ALL SMOKE TESTS PASSED CLEANLY")


if __name__ == "__main__":
    main()
