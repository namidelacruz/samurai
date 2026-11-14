#!/bin/sh
# Every check that can run without Hyprland, Quickshell or real hardware.
#   tests/run.sh            needs: cargo, lua5.4 (luac), python3 with PySide6 (pip install PySide6-Essentials)
set -e
cd "$(dirname "$0")/.."
echo "== rust tests";      cargo test --locked --quiet
echo "== release build";   cargo build --release --locked --quiet
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
export SAMURAI_CONFIG=$T/config SAMURAI_STATE=$T/state SAMURAI_HYPR=$T/hypr
echo "== generated Hyprland config: syntax, then a run against a stand-in hl"
./target/release/samurai apply >/dev/null && ./target/release/samurai check
lua5.4 tests/mock_hl.lua "$T/hypr/samurai/generated.lua"
echo "== seal: forged once, refused twice"
./target/release/samurai seal forge >/dev/null && ./target/release/samurai seal decoy >/dev/null
! ./target/release/samurai seal forge >/dev/null 2>&1
echo "== shell scripts parse";  sh -n install.sh && sh -n packaging/bin/samurai-greeter-setup
echo "== QML render check (35 scenes, both grounds, incognito, every hub page)"
SAMURAI_BINDS="$(./target/release/samurai binds list)" SAMURAI_SEAL=$T/config/seal QT_QPA_PLATFORM=offscreen python3 tests/qml/render.py "$T/render" | tail -1
unset SAMURAI_CONFIG SAMURAI_STATE SAMURAI_HYPR
echo "== QML wiring check (shell.qml and the greeter against stand-in Quickshell modules)"
QT_QPA_PLATFORM=offscreen python3 tests/qml/wiring.py ./target/release/samurai | tail -1
echo "all checks passed (none of them is Hyprland, Quickshell or hardware)"
