#!/bin/sh
# The login screen's failure paths, with stand-ins for X and the login screen itself.
# None of them may end at greetd's text login; each must end at a login screen, a retry, or
# (from a console) a clear message.
here=$(cd "$(dirname "$0")/.." && pwd)
t=$(mktemp -d); trap 'rm -rf "$t"' EXIT
fails=0
check() { if [ "$2" = 0 ]; then echo "  ok   $1"; else echo "  FAIL $1"; fails=$((fails + 1)); fi; }
mkdir -p "$t/p/bin" "$t/p/greeter" "$t/path"
# a text login that must never be reached
printf '#!/bin/sh\necho called >> "%s/agreety-called"\n' "$t" > "$t/path/agreety"
# samurai-x stand-in: runs the "greeter" directly (its behaviour comes from $BEHAVE)
cat > "$t/p/bin/samurai-x" <<'X'
#!/bin/sh
shift   # python3
exec sh "$@"
X
cat > "$t/p/greeter/greeter.py" <<'G'
# stand-in greeter (run with sh): $BEHAVE picks what it does
n=$(cat "$STATE.n" 2>/dev/null || echo 0); n=$((n + 1)); echo $n > "$STATE.n"
echo "stand-in run $n safe=${SAMURAI_GREETER_SAFE:-0} backend=${QT_QUICK_BACKEND:-gpu}"
case "$BEHAVE" in
  normal-breaks) [ "${SAMURAI_GREETER_SAFE:-0}" = 1 ] && { echo ok > "$SAMURAI_GREETER_DONE"; exit 0; }; exit 1;;
  always-breaks) exit 1;;
  crash-later) sleep 2; exit 7;;
  login) echo ok > "$SAMURAI_GREETER_DONE"; exit 0;;
esac
G
chmod +x "$t/p/bin/samurai-x" "$t/path/agreety"
run() { # run BEHAVE [extra env…]
    b=$1; shift
    rm -f "$t/st".n "$t/st".done
    env PATH="$t/path:$PATH" SAMURAI_PREFIX="$t/p" SAMURAI_GREETER_STATE="$t/st" STATE="$t/st" BEHAVE="$b" \
        SAMURAI_GREETER_BACKOFF=0 SAMURAI_GREETER_MAX_TRIES=6 "$@" sh "$here/packaging/bin/samurai-greeter-run" 2>&1
}
run login >/dev/null; rc=$?
check "a login ends the launcher with 0" $([ $rc = 0 ] && echo 0 || echo 1)
run normal-breaks >/dev/null; rc=$?
grep -q "safe=1 backend=software" "$t/st.log" && [ $rc = 0 ]; check "the screen breaks twice: safe mode (software drawing) takes over and the login works" $?
run always-breaks >/dev/null; rc=$?
[ $rc != 0 ] && [ "$(grep -c '^stand-in run' "$t/st.log")" = 6 ] && grep -q "safe mode" "$t/st.log"
check "it never stops trying on its own (6 of 6 tries made, safe mode used)" $?
run crash-later SAMURAI_GREETER_LONG=1 >/dev/null; rc=$?
[ $rc = 7 ] && [ "$(grep -c '^stand-in run' "$t/st.log")" = 1 ]; check "a crash after it was working: greetd gets to start a fresh one" $?
[ ! -f "$t/agreety-called" ]; check "the text login was never started" $?
ls "$t"/st.log.1 >/dev/null 2>&1; check "earlier logs are kept (.1 …)" $?

# the real login screen program
if python3 -c 'import PySide6' 2>/dev/null; then
    echo '{"ground": 5, "users": "x", "scale": "big"' > "$t/bad.json"
    for safe in 0 1; do
        SAMURAI_GREETER_CONF="$t/bad.json" SAMURAI_GREETER_FAKE=1 SAMURAI_GREETER_CHECK=1 SAMURAI_GREETER_SAFE=$safe \
            QT_QPA_PLATFORM=offscreen timeout 60 python3 "$here/greeter/greeter.py" 2>&1 | grep -q "check ok"
        check "a damaged settings file: the screen still loads (safe mode $safe)" $?
    done
    mkdir -p "$t/fake/PySide6"; echo 'raise ImportError("broken install")' > "$t/fake/PySide6/__init__.py"
    PYTHONPATH="$t/fake" timeout 20 python3 "$here/greeter/greeter.py" > "$t/imp" 2>&1; rc=$?
    [ $rc = 4 ] && grep -q "PySide6 cannot be loaded" "$t/imp"; check "PySide6 broken: it stops at once with its own code, so the launcher retries" $?
    QT_QPA_PLATFORM=offscreen timeout 30 python3 - "$here" "$t" <<'PY' 2>/dev/null
import os, socket, sys, threading, time
here, t = sys.argv[1], sys.argv[2]
sock = os.path.join(t, "greetd.sock")
srv = socket.socket(socket.AF_UNIX); srv.bind(sock); srv.listen(1)
def close_at_once():
    c, _ = srv.accept(); c.recv(100); c.close()
threading.Thread(target=close_at_once, daemon=True).start()
os.environ["GREETD_SOCK"] = sock; os.environ["SAMURAI_GREETER_CONF"] = "/dev/null"
sys.path.insert(0, os.path.join(here, "greeter"))
import greeter
from PySide6.QtGui import QGuiApplication
app = QGuiApplication([])
c = greeter.Core(); said = []
c.said.connect(lambda m, bad: said.append((m, bad)))
c.login("user", "pw", 0)
end = time.time() + 10
while (c.busy or not said) and time.time() < end:
    app.processEvents(); time.sleep(0.02)
ok = said and said[-1][1] and "did not answer" in said[-1][0] and not c.busy
print("socket-ok" if ok else f"socket-bad {said}")
sys.exit(0 if ok else 1)
PY
    check "greetd closes the connection: a message, and the screen stays usable" $?
else
    echo "  skip the real login screen parts (PySide6 is not installed)"
fi

# the session from a plain console: says what went wrong and returns, never loops
mkdir -p "$t/s/bin" "$t/home"
printf '#!/bin/sh\necho "(EE) Fatal server error: stand-in"; exit 1\n' > "$t/s/bin/samurai-x"; chmod +x "$t/s/bin/samurai-x"
out=$(HOME="$t/home" XDG_STATE_HOME="$t/home/state" SAMURAI_PREFIX="$t/s" timeout 20 script -qec "sh $here/packaging/bin/samurai-session" /dev/null 2>&1); rc=$?
echo "$out" | grep -q "Samurai could not start" && echo "$out" | grep -q "Fatal server error" && [ $rc != 0 ] && [ $rc != 124 ]
check "from a console: a clear message with the error, back to the shell" $?

[ $fails = 0 ] && echo "all login failure checks passed" || echo "$fails login failure check(s) FAILED"
[ $fails = 0 ]
