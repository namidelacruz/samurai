#!/bin/sh
# The session restarts a crashing window manager, switches to safe mode after three crashes in a
# minute, and opens a rescue terminal if safe mode crashes too. A stand-in samurai-wm crashes N times.
here=$(cd "$(dirname "$0")/.." && pwd)
t=$(mktemp -d); trap 'rm -rf "$t"' EXIT
mkdir -p "$t/bin" "$t/h"
cat > "$t/bin/samurai-wm" <<W
#!/bin/sh
n=\$(cat "$t/n" 2>/dev/null || echo 0); n=\$((n+1)); echo \$n > "$t/n"
echo "run \$n safe=\$SAMURAI_WM_SAFE" >> "$t/runs"
[ \$n -le \$CRASHES ] && exit 101
exit 0
W
printf '#!/bin/sh\necho "samurai $*"\n' > "$t/bin/samurai"
printf '#!/bin/sh\necho RESCUE >> "%s/runs"\n' "$t" > "$t/bin/xterm"
chmod +x "$t/bin/"*
fails=0
go() { rm -f "$t/n" "$t/runs"; CRASHES=$1 HOME="$t/h" SAMURAI_PREFIX="$t" PATH="$t/bin:/usr/bin:/bin" DISPLAY=:0 timeout 60 sh "$here/packaging/bin/samurai-session" --inner >/dev/null 2>&1; tr '\n' ';' < "$t/runs"; }
check() { if [ "$2" = "$3" ]; then echo "  ok   $1"; else echo "  FAIL $1 (got $2)"; fails=$((fails + 1)); fi; }
check "a crash: it starts again, the session goes on" "$(go 1)" "run 1 safe=0;run 2 safe=0;"
check "three crashes in a minute: safe mode" "$(go 4)" "run 1 safe=0;run 2 safe=0;run 3 safe=0;run 4 safe=1;run 5 safe=1;"
check "safe mode crashing too: a rescue terminal, not a black screen" "$(go 9)" "run 1 safe=0;run 2 safe=0;run 3 safe=0;run 4 safe=1;run 5 safe=1;run 6 safe=1;RESCUE;"
[ $fails = 0 ]
