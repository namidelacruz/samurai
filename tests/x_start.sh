#!/bin/sh
# samurai-x starts X for a user whose home is "/" (the login screen's user on many systems).
# Before the fix, X stopped with: Cannot open log file "//.local/share/xorg/Xorg.0.log".
# Needs root (to run as a throwaway user), Xvfb and python-xlib; skips otherwise.
here=$(cd "$(dirname "$0")/.." && pwd)
[ "$(id -u)" = 0 ] && command -v Xvfb >/dev/null && command -v runuser >/dev/null || { echo "skip: needs root, Xvfb and runuser"; exit 0; }
u=samurai-xtest-$$
useradd -M -d / "$u" || exit 1
t=$(mktemp -d); cp -r "$here/tests/x11" "$t/"; chmod -R a+rX "$t"; chmod 1777 "$t"
out=$(cd / && runuser -u "$u" -- env -i PATH="$t/x11/bin:/usr/bin:/bin" HOME=/ XDG_RUNTIME_DIR="$t" \
      sh "$here/packaging/bin/samurai-x" /usr/bin/python3 "$t/x11/probe.py" 2>&1)
userdel "$u" 2>/dev/null; rm -rf "$t"
ok=0
echo "$out" | grep -q "connected with cookie" && echo "  ok   X starts for a user whose home is /" || { echo "  FAIL X starts for a user whose home is /"; ok=1; }
echo "$out" | grep -q "refused without cookie" && echo "  ok   other users cannot open the screen" || { echo "  FAIL other users cannot open the screen"; ok=1; }
[ $ok = 0 ] || echo "$out" | tail -n 8
exit $ok
