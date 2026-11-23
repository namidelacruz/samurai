#!/bin/sh
# Every automatic test that can run without real hardware. Parts whose tools are missing are
# skipped and say so. Run from the source folder: sh tests/run.sh
cd "$(dirname "$0")/.." || exit 1
fails=0
part() { printf '\n── %s\n' "$1"; }
result() { if [ "$1" = 0 ]; then echo "   passed"; else echo "   FAILED"; fails=$((fails + 1)); fi; }

part "Rust unit tests"
cargo test --release -q 2>&1 | grep -E "test result|FAILED|panicked" ; result "$(cargo test --release -q >/dev/null 2>&1; echo $?)"

part "shell scripts parse"
rc=0; for f in install.sh packaging/bin/*; do sh -n "$f" || { echo "   $f"; rc=1; }; done; result $rc

part "window manager in Xvfb"
cargo build --release -q -p samurai-wm && python3 tests/wm_xvfb.py; result $?

part "Settings pages render (offscreen)"
if python3 -c 'import PySide6' 2>/dev/null; then QT_QPA_PLATFORM=offscreen python3 tests/settings_shots.py >/tmp/samurai-shots.log 2>&1; rc=$?; tail -n 2 /tmp/samurai-shots.log; result $rc
else echo "   skip: PySide6 is not installed"; fi

part "login screen against greetd's fakegreet"
FG=${FAKEGREET:-$(command -v fakegreet || true)}
if [ -n "$FG" ] && python3 -c 'import PySide6' 2>/dev/null; then
    d=$(mktemp -d); here=$(pwd)
    (cd "$d" && SAMURAI_GREETER_DONE="$d/done" "$FG" "python3 -u $here/tests/greeter_fake.py > $d/out 2>&1; echo \$? > $d/rc" >/dev/null 2>&1 &)
    i=0; while [ ! -f "$d/rc" ] && [ $i -lt 60 ]; do sleep 0.5; i=$((i + 1)); done
    pkill -f "^$FG" 2>/dev/null; pkill -x fakegreet 2>/dev/null
    grep -v propagate "$d/out" 2>/dev/null | sed 's/^/ /'
    rc=$(cat "$d/rc" 2>/dev/null || echo 1); [ -f "$d/done" ] || rc=1; rm -rf "$d"; result "$rc"
else echo "   skip: set FAKEGREET=/path/to/fakegreet (built from greetd's source) to run this"; fi

printf '\n%s\n' "$( [ $fails = 0 ] && echo 'all parts passed' || echo "$fails part(s) failed" )"
[ $fails = 0 ]
