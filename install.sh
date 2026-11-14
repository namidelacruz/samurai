#!/bin/sh
# Samurai installer. One status bar; every line of detail goes to the log.
#   ./install.sh              install for this user, login screen included (greetd)
#   ./install.sh --no-greeter leave the login screen as it is
#   ./install.sh --update     same steps, quieter wording (used by `samurai update install`)
#   ./install.sh --uninstall  remove what the installer put in place (your settings stay)
# It installs to /opt/samurai, links commands into /usr/local/bin, and writes Samurai's own niri
# config under ~/.config/samurai/niri (your ~/.config/niri is not touched). The login screen
# step writes /etc/greetd/config.toml and makes greetd the login manager from the next boot; what
# was there before is kept and --uninstall puts it back.
# It never touches the bootloader, the initramfs or firmware settings.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
LOGDIR=${XDG_STATE_HOME:-$HOME/.local/state}/samurai
LOG=$LOGDIR/install.log
mkdir -p "$LOGDIR"; : > "$LOG"
MODE=install; GREETER=1
for a in "$@"; do case "$a" in
    --greeter) GREETER=1;; --no-greeter) GREETER=0;; --update) MODE=update;; --uninstall) MODE=uninstall;;
    -h|--help) sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'; exit 0;;
    *) echo "install.sh: unknown option $a" >&2; exit 2;;
esac; done
# an update keeps the choice made at install time
[ "$MODE" != update ] || [ -f /etc/samurai/greeter.json ] || GREETER=0
[ "$(id -u)" != 0 ] || { echo "Run install.sh as yourself, not as root: it asks for the password when it needs it." >&2; exit 1; }
# root steps: sudo at a terminal, pkexec from the settings window
if [ -t 0 ] && command -v sudo >/dev/null 2>&1; then AS_ROOT=sudo; elif command -v pkexec >/dev/null 2>&1; then AS_ROOT=pkexec; else AS_ROOT=sudo; fi

# ── the status bar: the only thing on screen ───────────────────────────────────────────────
WIDTH=32; STEP=0; TOTAL=1
bar() { # bar LABEL
    fill=$((STEP * WIDTH / TOTAL)); i=0; b=""
    while [ $i -lt $WIDTH ]; do if [ $i -lt $fill ]; then b="$b█"; else b="$b░"; fi; i=$((i + 1)); done
    if [ -t 1 ]; then printf '\r\033[K 侍 %s %3d%%  %s' "$b" $((STEP * 100 / TOTAL)) "$1"
    else printf ' %3d%%  %s\n' $((STEP * 100 / TOTAL)) "$1"; fi
}
fail() { # fail LABEL
    [ -t 1 ] && printf '\r\033[K'
    printf ' ✕ stopped at: %s\n   nothing after this step was changed. Detail: %s\n' "$1" "$LOG"
    exit 1
}
step() { # step LABEL COMMAND…   run one step, all output to the log
    label=$1; shift
    bar "$label"
    printf '\n== %s\n' "$label" >> "$LOG"
    "$@" >> "$LOG" 2>&1 || fail "$label"
    STEP=$((STEP + 1))
}

# ── steps ──────────────────────────────────────────────────────────────────────────────────
check_needs() {
    miss=""
    for c in cargo git; do command -v $c >/dev/null 2>&1 || miss="$miss $c"; done
    [ -z "$miss" ] || { echo "missing build tools:$miss"; return 1; }
    if command -v niri >/dev/null 2>&1; then echo "niri: $(niri --version 2>/dev/null)"
    else echo "note: niri is not installed yet (Samurai needs niri 25.11 or newer; 26.04 for per-app blur)"; fi
    command -v qs >/dev/null 2>&1 || command -v quickshell >/dev/null 2>&1 || echo "note: quickshell is not installed yet"
    for c in nmcli wpctl brightnessctl grim slurp wl-copy gammastep; do command -v $c >/dev/null 2>&1 || echo "note: optional tool $c is not installed"; done
    if command -v fc-list >/dev/null 2>&1; then
        fc-list | grep -qi "IBM Plex Mono" || echo "note: font IBM Plex Mono is not installed (a fallback mono font will be used)"
        fc-list | grep -qi "Noto Sans CJK JP\|Noto Sans JP" || echo "note: font Noto Sans CJK JP is not installed (kanji may use a fallback)"
    fi
    return 0
}
build() { cd "$HERE" && cargo build --release --locked; }
stage() { # everything is laid out in a staging folder first, then swapped in at once
    S=$LOGDIR/stage; rm -rf "$S"; mkdir -p "$S/bin" "$S/share"
    cp "$HERE/target/release/samurai" "$HERE/target/release/shadow" "$S/bin/" || return 1
    cp "$HERE"/packaging/bin/* "$S/bin/" && chmod 0755 "$S/bin/"* || return 1
    cp -r "$HERE/shell" "$S/shell" || return 1
    cp "$HERE/packaging/share/"*.desktop "$S/share/" || return 1
    (cd "$HERE" && git rev-parse HEAD 2>/dev/null) > "$S/share/COMMIT" || true
    "$S/bin/samurai" version && "$S/bin/samurai" schema > /dev/null
}
place() {
    S=$LOGDIR/stage
    $AS_ROOT sh -c '
        set -e
        rm -rf /opt/samurai.new && mkdir -p /opt && cp -r "$1" /opt/samurai.new && chown -R root:root /opt/samurai.new
        [ ! -d /opt/samurai ] || { rm -rf /opt/samurai.old; mv /opt/samurai /opt/samurai.old; }
        mv /opt/samurai.new /opt/samurai
        mkdir -p /usr/local/bin /usr/share/wayland-sessions /usr/share/applications
        ln -sfn /opt/samurai/bin/samurai /usr/local/bin/samurai
        [ -e /usr/local/bin/shadow ] && [ ! -L /usr/local/bin/shadow ] || ln -sfn /opt/samurai/bin/shadow /usr/local/bin/shadow
        install -m 0644 /opt/samurai/share/samurai.desktop /usr/share/wayland-sessions/samurai.desktop
        install -m 0644 /opt/samurai/share/samurai-hub.desktop /usr/share/applications/samurai-hub.desktop
    ' sh "$S" && rm -rf "$S"
}
user_files() {
    mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/quickshell"
    ln -sfn /opt/samurai/shell "${XDG_CONFIG_HOME:-$HOME/.config}/quickshell/samurai"
    /opt/samurai/bin/samurai apply && /opt/samurai/bin/samurai check
}
seal() { # the one-time seal (kept if this machine already has one) and the public decoy
    /opt/samurai/bin/samurai seal show | grep -q '"forged":true' || /opt/samurai/bin/samurai seal forge > /dev/null || return 1
    /opt/samurai/bin/samurai seal decoy
}
greeter() { # the login screen. A missing part is installed when the system has pacman or dnf;
            # if it still cannot be set up, that is a note, not a failed install (exit 3 = part missing).
    if ! command -v greetd >/dev/null 2>&1; then
        if command -v pacman >/dev/null 2>&1; then $AS_ROOT pacman -S --needed --noconfirm greetd || true
        elif command -v dnf >/dev/null 2>&1; then $AS_ROOT dnf install -y greetd || true
        fi
    fi
    $AS_ROOT /opt/samurai/bin/samurai-greeter-setup "$(/opt/samurai/bin/samurai get greeter.base | tr -d '"')" "$(id -un)" && return 0
    [ $? = 3 ] && { echo "note: LOGIN SCREEN NOT SET UP (see the line above); fix that and run ./install.sh again"; return 0; }
    return 1
}
reload() { # a running session picks the new version up; never fails the install
    # niri reloads its config by itself; only the frame needs a restart, and only inside a Samurai session
    [ -n "${NIRI_SOCKET:-}" ] && pgrep -f "(qs|quickshell) -c samurai" >/dev/null 2>&1 && /opt/samurai/bin/samurai shell restart || true
}
remove() {
    $AS_ROOT sh -c '
        [ ! -x /opt/samurai/bin/samurai-greeter-setup ] || /opt/samurai/bin/samurai-greeter-setup --remove || true
        rm -rf /opt/samurai /opt/samurai.old /opt/samurai.new /usr/share/wayland-sessions/samurai.desktop /usr/share/applications/samurai-hub.desktop
        [ "$(readlink /usr/local/bin/samurai 2>/dev/null)" != /opt/samurai/bin/samurai ] || rm -f /usr/local/bin/samurai
        [ "$(readlink /usr/local/bin/shadow 2>/dev/null)" != /opt/samurai/bin/shadow ] || rm -f /usr/local/bin/shadow
    '
    rm -f "${XDG_CONFIG_HOME:-$HOME/.config}/quickshell/samurai"
}

if [ "$MODE" = uninstall ]; then
    TOTAL=1; step "removing" remove; bar "removed"; [ -t 1 ] && echo
    echo " Your settings and your seal (~/.config/samurai) were left in place. The previous login screen is back from the next boot."
    exit 0
fi
TOTAL=$((7 + GREETER))
step "checking this machine" check_needs
step "building" build
step "preparing files" stage
step "installing (asks for your password once)" place
step "writing the niri config" user_files
step "forging this machine's seal" seal
[ $GREETER = 0 ] || step "setting up the login screen (takes effect at the next boot)" greeter
step "finishing" reload
bar "done"; [ -t 1 ] && echo
NOTES=$(grep -c '^note:' "$LOG" || true)
if [ "$MODE" = update ]; then echo " Samurai is up to date."; else
    echo " Samurai is installed. Log out and choose “Samurai” at the login screen."
    if grep -q "^login screen: set" "$LOG"; then echo " The Samurai login screen replaces your current one from the next boot."
    elif [ $GREETER = 1 ]; then echo " ! The login screen was NOT set up: $(grep -B1 '^note: LOGIN SCREEN' "$LOG" | head -1)"; fi
fi
[ "$NOTES" = 0 ] || echo " $NOTES notes about missing optional parts are in $LOG"
exit 0
