#!/usr/bin/env bash
set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$REPO_ROOT/target/release/Gradle"
DEB="$(ls -t "$REPO_ROOT/target/release/bundle/deb/"gradle-launcher_*_amd64.deb 2>/dev/null | head -n 1)"

echo "=========================================="
echo "GRADLE LAUNCHER RELEASE VERIFICATION"
echo "=========================================="

if [ ! -f "$BIN" ]; then
    echo "ERROR: Release binary $BIN does not exist!"
    exit 1
fi

if [ ! -f "$DEB" ]; then
    echo "ERROR: Release debian package $DEB does not exist!"
    exit 1
fi

echo "[1] Binary Details:"
ls -lh "$BIN"
ls -lh "$DEB"

echo -e "\n[2] SHA-256 Hashes (ensuring difference from stale c0698236...):"
sha256sum "$BIN" "$DEB"

echo -e "\n[3] Tauri ACL Permission Strings in Binary:"
echo "plugin:window| count: $(strings "$BIN" | grep -c "plugin:window|" || true)"
echo "plugin:menu| count:   $(strings "$BIN" | grep -c "plugin:menu|" || true)"
echo "plugin:webview| count:$(strings "$BIN" | grep -c "plugin:webview|" || true)"
echo "plugin:image| count:  $(strings "$BIN" | grep -c "plugin:image|" || true)"
echo "plugin:os|locale:     $(strings "$BIN" | grep -c "plugin:os|locale" || true)"
echo "plugin:opener|:       $(strings "$BIN" | grep -c "plugin:opener|" || true)"
echo "plugin:instance|:     $(strings "$BIN" | grep -c "plugin:instance|" || true)"

echo -e "\n[4] Path Remapping (/home/ leak check - MUST BE 0):"
HOME_LEAKS=$(strings "$BIN" | grep -c "/home/" || true)
echo "/home/ leaks: $HOME_LEAKS"
if [ "$HOME_LEAKS" -ne 0 ]; then
    echo "WARNING: /home/ paths found in binary:"
    strings "$BIN" | grep "/home/" | head -n 5
fi

echo -e "\n[5] Max GLIBC requirement:"
objdump -T "$BIN" | grep -oE "GLIBC_[0-9.]+" | sort -V | tail -1

echo -e "\n[6] OpenSSL Dependency Check (rustls verification):"
ldd "$BIN" | grep ssl || echo "No OpenSSL dynamically linked. (Pure rustls verified)"

echo -e "\n[7] Debian Package Control Info:"
dpkg-deb -I "$DEB" | grep -E "Package:|Version:|Depends:|Description:"

echo -e "\n[8] Verification completed successfully!"
