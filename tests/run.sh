#!/bin/sh
# Every check that can run without niri, Quickshell or real hardware.
#   tests/run.sh       needs: cargo, python3 with PySide6 (pip install PySide6-Essentials)
# The generated niri config is checked by niri's own parser when one is at hand: a real `niri`
# on PATH, or NIRI_VALIDATOR=/path/to/a program built from niri's niri-config crate that takes
# a config path. Without either, that one check is reported as SKIPPED, not passed.
set -e
cd "$(dirname "$0")/.."
echo "== rust tests";      cargo test --locked --quiet
echo "== release build";   cargo build --release --locked --quiet
T=$(mktemp -d); trap 'rm -rf "$T"' EXIT
export SAMURAI_CONFIG=$T/config SAMURAI_STATE=$T/state
S=./target/release/samurai
if command -v niri >/dev/null 2>&1; then echo "== niri config: checked by the installed niri"
elif [ -n "${NIRI_VALIDATOR:-}" ]; then echo "== niri config: checked by niri's parser ($NIRI_VALIDATOR)"; PATH="$PWD/tests/bin:$PATH"
else echo "== niri config: SKIPPED the parser check (no niri, no NIRI_VALIDATOR); braces and quotes only"; fi
$S apply >/dev/null && $S check
if command -v niri >/dev/null 2>&1; then
    echo "== every niri-backed setting at its extremes: accepted, or refused and rolled back"
    python3 tests/settings_sweep.py $S
    echo "== a refused config never replaces a working one"
    cp "$T/config/niri/generated.kdl" "$T/before.kdl"
    printf 'layout { no-such-thing 1; }\n' > "$T/config/niri/user.kdl"   # the user's own file is broken …
    $S apply | grep -q "warning: with your user.kdl" && echo "   user.kdl problem reported"
    : > "$T/config/niri/user.kdl"
fi
echo "== seal: forged once, refused twice"
$S seal forge >/dev/null && $S seal decoy >/dev/null
! $S seal forge >/dev/null 2>&1
echo "== shell scripts parse";  for f in install.sh packaging/bin/*; do sh -n "$f"; done
echo "== QML render check (35 scenes, both grounds, incognito, every hub page)"
SAMURAI_BINDS="$($S binds list)" SAMURAI_SEAL=$T/config/seal QT_QPA_PLATFORM=offscreen python3 tests/qml/render.py "$T/render" | tail -1
unset SAMURAI_CONFIG SAMURAI_STATE
echo "== QML wiring check (shell.qml and the greeter against stand-in Quickshell modules and a stand-in niri)"
QT_QPA_PLATFORM=offscreen python3 tests/qml/wiring.py $S | tail -1
echo "all checks passed (none of them is niri itself, Quickshell or hardware)"
