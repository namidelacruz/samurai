#!/bin/sh
# samurai-key-pam on copies of PAM files: only our marked lines change, the key line goes first,
# two-factor needs a tested key and is never set for sudo, and disable puts every file back.
here=$(cd "$(dirname "$0")/.." && pwd)
t=$(mktemp -d); trap 'rm -rf "$t"' EXIT
mkdir -p "$t/pam"
printf '#%%PAM-1.0\nauth       required     pam_securetty.so\nauth       include      system-login\naccount    include      system-login\n' > "$t/pam/greetd"
printf '#%%PAM-1.0\n# The Samurai lock screen checks your password with this (installed by Samurai).\nauth      include   system-auth\naccount   include   system-auth\n' > "$t/pam/samurai-lock"
printf '#%%PAM-1.0\nauth\t\tinclude\t\tsystem-auth\naccount\t\tinclude\t\tsystem-auth\n' > "$t/pam/sudo"
cp -r "$t/pam" "$t/orig"
K() { SAMURAI_PAM_DIR="$t/pam" SAMURAI_PAM_BACKUP="$t/bak" sh "$here/packaging/bin/samurai-key-pam" "$@" 2>&1; }
fails=0
check() { if [ "$2" = 0 ]; then echo "  ok   $1"; else echo "  FAIL $1"; fails=$((fails + 1)); fi; }
K enable usb either --sudo >/dev/null
[ "$(grep -m1 '^auth' "$t/pam/greetd")" = "auth sufficient pam_exec.so quiet /opt/samurai/bin/samurai key check # samurai-key" ]; check "key or password: the key line comes first in the login screen's rules" $?
grep -q "samurai-key" "$t/pam/sudo"; check "sudo too, when asked (key or password only)" $?
[ "$(grep -vc 'samurai-key' "$t/pam/greetd")" = "$(grep -c '' "$t/orig/greetd")" ]; check "nothing else in the file changed" $?
K enable usb either >/dev/null; [ "$(grep -c samurai-key "$t/pam/greetd")" = 1 ] && ! grep -q samurai-key "$t/pam/sudo"; check "enabling again: one line, never two; sudo dropped when not asked" $?
out=$(K enable usb both); [ $? != 0 ] || true; echo "$out" | grep -q "just tested" && grep -q "sufficient" "$t/pam/greetd"; check "two-factor without a tested key is refused, nothing changes" $?
out=$(K enable usb both --sudo --tested nami); echo "$out" | grep -q "never set for sudo"; check "two-factor is never set for sudo" $?
K enable u2f both --tested nami >/dev/null; grep -q "^auth required pam_u2f.so cue # samurai-key" "$t/pam/samurai-lock"; check "a tested key: two-factor (key and password)" $?
K disable >/dev/null
diff -r "$t/orig" "$t/pam" >/dev/null; check "disable puts every file back exactly" $?
[ -f "$t/bak/greetd" ]; check "the original files were backed up first" $?
[ $fails = 0 ]
