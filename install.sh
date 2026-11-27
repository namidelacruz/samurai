#!/bin/sh
# Samurai installer (Arch Linux). One status line; every line of detail goes to the log.
#
#   curl -fsSL https://raw.githubusercontent.com/namidelacruz/samurai/main/install.sh | sh
#   or, from a downloaded copy:  ./install.sh
#
#   ./install.sh                install everything: packages, the desktop, the login screen
#   ./install.sh --no-greeter   leave the login screen as it is
#   ./install.sh --no-packages  install no packages (you manage them yourself)
#   ./install.sh --no-apps      skip the default apps (browser, files, editor, viewers, mail)
#   ./install.sh --no-hyprland  skip the second session, Samurai on Hyprland (Wayland)
#   ./install.sh --update       same steps, quieter wording (used by `samurai update install`)
#   ./install.sh --uninstall    put back what was there before (your Samurai settings stay)
#   install.sh --for-user NAME  as root, for NAME (archinstall's custom commands, a chroot,
#                               or any setup with no one at the keyboard): no password asked
#
# It installs Samurai to /opt/samurai, links its commands into /usr/local/bin, adds "Samurai" to
# the sessions list, and (unless --no-greeter) makes greetd with the Samurai login screen the login
# manager from the next boot; the login manager it replaces is remembered and --uninstall puts it
# back. Packages come only from the repositories pacman already uses; nothing is removed or
# replaced, and an X server that is already there (X.Org or XLibre) is kept.
# It never touches the bootloader, the initramfs, firmware settings or running services.
set -u
REPO=${SAMURAI_REPO:-https://github.com/namidelacruz/samurai.git}
BRANCH=${SAMURAI_BRANCH:-main}

# ── run from curl: fetch the source first, then run its own installer ─────────────────────────
HERE=$(cd "$(dirname "$0")" 2>/dev/null && pwd)
if [ ! -f "$HERE/Cargo.toml" ] || [ ! -d "$HERE/wm" ]; then
    SRC=${XDG_DATA_HOME:-$HOME/.local/share}/samurai/src
    command -v git >/dev/null 2>&1 || {
        echo " Samurai needs git to fetch its source: sudo pacman -S --needed git base-devel rust" >&2; exit 1; }
    echo " fetching Samurai into $SRC …"
    if [ -d "$SRC/.git" ]; then git -C "$SRC" fetch -q --depth 1 origin "$BRANCH" && git -C "$SRC" checkout -q -B "$BRANCH" FETCH_HEAD
    else mkdir -p "$(dirname "$SRC")" && git clone -q --depth 1 -b "$BRANCH" "$REPO" "$SRC"; fi || {
        echo " could not fetch $REPO" >&2; exit 1; }
    # the piped script has no keyboard; the real installer gets the terminal back for passwords
    if ( : < /dev/tty ) 2>/dev/null; then exec sh "$SRC/install.sh" "$@" < /dev/tty; fi
    exec sh "$SRC/install.sh" "$@"
fi

MODE=install; GREETER=1; PACKAGES=1; APPS=1; HYPR=1; FOR=""
prev=""
for a in "$@"; do
    if [ "$prev" = --for-user ]; then FOR=$a; prev=""; continue; fi
    case "$a" in
    --for-user) prev=--for-user;; --for-user=*) FOR=${a#--for-user=};;
    --greeter) GREETER=1;; --no-greeter) GREETER=0;; --no-packages) PACKAGES=0;; --no-apps) APPS=0;; --no-hyprland) HYPR=0;;
    --update) MODE=update;; --uninstall) MODE=uninstall;;
    -h|--help) sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'; exit 0;;
    *) echo "install.sh: unknown option $a" >&2; exit 2;;
esac; done
[ "$prev" != --for-user ] || { echo "install.sh: --for-user needs a user name" >&2; exit 2; }
# as_user CMD…: a step that belongs to the person who will use the desktop
if [ -n "$FOR" ]; then
    [ "$(id -u)" = 0 ] || { echo "--for-user is for running as root (for example from archinstall)" >&2; exit 1; }
    UHOME=$(getent passwd "$FOR" | cut -d: -f6)
    [ -n "$UHOME" ] && [ -d "$UHOME" ] || { echo "install.sh: there is no user $FOR with a home folder" >&2; exit 1; }
    as_user() { runuser -u "$FOR" -- env HOME="$UHOME" USER="$FOR" LOGNAME="$FOR" XDG_CONFIG_HOME= XDG_STATE_HOME= XDG_DATA_HOME= "$@"; }
    LOGDIR=/var/log/samurai
else
    as_user() { "$@"; }
    LOGDIR=${XDG_STATE_HOME:-$HOME/.local/state}/samurai
fi
LOG=$LOGDIR/install.log
mkdir -p "$LOGDIR"; : > "$LOG"
# an update keeps the choices made at install time
[ "$MODE" != update ] || [ -f /etc/samurai/greeter.json ] || GREETER=0
[ "$(id -u)" != 0 ] || [ -n "$FOR" ] || { echo "Run install.sh as yourself, not as root: it asks for the password when it needs it (or, as root for another user: --for-user NAME)." >&2; exit 1; }
if [ "$MODE" != uninstall ] && ! command -v pacman >/dev/null 2>&1; then
    if [ $PACKAGES = 1 ]; then echo "Samurai's installer is for Arch Linux (pacman). Use --no-packages to install only Samurai." >&2; exit 2; fi
fi
# root steps: sudo when there is a terminal, pkexec from the settings window
if [ -n "$FOR" ]; then AS_ROOT=""
elif [ -t 1 ] && command -v sudo >/dev/null 2>&1; then AS_ROOT=sudo; elif command -v pkexec >/dev/null 2>&1; then AS_ROOT=pkexec; else AS_ROOT=sudo; fi

# ── the status line: the only thing on screen ─────────────────────────────────────────────────
WIDTH=24; STEP=0; TOTAL=1
TTY=0; [ -t 1 ] && TTY=1
COLS=$( { stty size < /dev/tty | cut -d' ' -f2; } 2>/dev/null ); case "$COLS" in ''|*[!0-9]*) COLS=80;; esac; [ "$COLS" -ge 40 ] || COLS=80
bar() { # bar LABEL [DETAIL]   one line: the bar, the step, and what it is doing right now
    fill=$((STEP * WIDTH / TOTAL)); i=0; b=""
    while [ $i -lt $WIDTH ]; do if [ $i -lt $fill ]; then b="$b━"; else b="$b─"; fi; i=$((i + 1)); done
    line=$(printf '%3d%%  %s%s' $((STEP * 100 / TOTAL)) "$1" "${2:+ · $2}")
    if [ $TTY = 1 ]; then
        room=$((COLS - WIDTH - 6)); [ $room -gt 10 ] || room=10
        printf '\r\033[K 侍 %s %s' "$b" "$(printf '%s' "$line" | cut -c1-$room)"
    else printf ' %s\n' "$line"; fi
}
fail() { # fail LABEL
    [ $TTY = 1 ] && printf '\r\033[K'
    printf ' ✕ stopped at: %s\n' "$1"
    tail -n 4 "$LOG" | sed 's/^/   │ /'
    printf '   Nothing after this step was changed. Full detail: %s\n' "$LOG"
    exit 1
}
step() { # step LABEL COMMAND…   run one step, all of its output to the log
    label=$1; shift
    bar "$label"
    printf '\n== %s\n' "$label" >> "$LOG"
    "$@" >> "$LOG" 2>&1 || fail "$label"
    STEP=$((STEP + 1))
}
watch() { # watch LABEL COMMAND…   like step, but the status line names the item being worked on
    label=$1; shift
    bar "$label"
    printf '\n== %s\n' "$label" >> "$LOG"
    rcf=$LOGDIR/.rc; echo 1 > "$rcf"
    { "$@" 2>&1; echo $? > "$rcf"; } | while IFS= read -r l; do
        printf '%s\n' "$l" >> "$LOG"
        case "$l" in
            *"installing "*|*"upgrading "*)
                # pacman: "(12/40) installing alacritty"
                n=$(printf '%s' "$l" | sed -n 's/.*(\s*\([0-9]*\)\/\([0-9]*\)).*/\1 of \2/p')
                p=$(printf '%s' "$l" | sed -n 's/.*\(installing\|upgrading\) \([^ .]*\).*/\2/p')
                [ -n "$p" ] && bar "$label" "${n:+$n: }$p";;
            *" downloading"*) p=$(printf '%s' "$l" | sed -n 's/^ *\([^ ]*\) downloading.*/\1/p'); bar "$label" "downloading ${p%%-[0-9]*}";;
            *"Compiling "*) p=$(printf '%s' "$l" | sed -n 's/.*Compiling \([^ ]*\) .*/\1/p'); bar "$label" "compiling $p";;
            *"Downloaded "*) p=$(printf '%s' "$l" | sed -n 's/.*Downloaded \([^ ]*\) .*/\1/p'); bar "$label" "fetched $p";;
            *"Finished "*) bar "$label" "linking";;
            *"checking keys"*|*"checking package integrity"*|*"loading package files"*|*"checking for file conflicts"*)
                bar "$label" "$(printf '%s' "$l" | sed 's/^[( 0-9/)]*//; s/\.\.\..*//')";;
        esac
    done
    [ "$(cat "$rcf")" = 0 ] || fail "$label"
    rm -f "$rcf"
    STEP=$((STEP + 1))
}

# ── what "everything" is ───────────────────────────────────────────────────────────────────────
# the desktop itself and every feature it has
CORE="xorg-xinit xorg-xauth xorg-xrandr xorg-xinput xorg-xset xorg-xsetroot xorg-xrdb
      alacritty dunst libnotify xsettingsd xss-lock polkit-gnome gammastep maim xclip xdotool
      networkmanager wpa_supplicant pipewire wireplumber brightnessctl playerctl
      bluez bluez-utils power-profiles-daemon upower fastfetch curl gnupg pam-u2f
      pyside6 adw-gtk-theme papirus-icon-theme qt5ct qt6ct
      inter-font ttf-jetbrains-mono noto-fonts noto-fonts-cjk noto-fonts-emoji
      xdg-desktop-portal xdg-desktop-portal-gtk xdg-utils xdg-user-dirs
      gvfs udisks2 dbus"
LOGIN="greetd"
BUILD="git rust gcc pkgconf"
# the second session on the login screen: Samurai on Hyprland (Wayland)
HYPR_LIST="hyprland waybar fuzzel hyprlock hypridle swaybg grim slurp wl-clipboard jq xdg-desktop-portal-hyprland xorg-xwayland"
APPS_LIST="firefox thunar thunar-volman tumbler mousepad imv zathura zathura-pdf-mupdf mpv thunderbird file-roller"

# an X server, unless one is already there (X.Org or XLibre: both are kept as they are)
xserver_pkgs() {
    if pacman -Qq xorg-server >/dev/null 2>&1 || pacman -Qqs '^xlibre-(x)?server' >/dev/null 2>&1 || command -v Xorg >/dev/null 2>&1; then
        echo ""
    else
        echo "xorg-server xf86-input-libinput"
    fi
}
# sound: pipewire's PulseAudio part only when PulseAudio itself is not installed (never replace it)
sound_pkgs() { pacman -Qq pulseaudio >/dev/null 2>&1 && echo "" || echo "pipewire-pulse"; }

plan_packages() { # writes the list to install and the notes for any pacman does not know
    # a system just made by pacstrap/archinstall may not have the package lists yet
    if [ -n "$FOR" ] && [ -z "$(ls /var/lib/pacman/sync 2>/dev/null)" ]; then pacman -Sy --noconfirm || return 1; fi
    want="$BUILD $CORE $(xserver_pkgs) $(sound_pkgs)"
    [ $GREETER = 0 ] || want="$want $LOGIN"
    [ $APPS = 0 ] || want="$want $APPS_LIST"
    [ $HYPR = 0 ] || want="$want $HYPR_LIST"
    : > "$LOGDIR/.pkgs"
    for p in $want; do
        pacman -Qq "$p" >/dev/null 2>&1 && continue
        # something else already provides it (rustup gives rust, an XLibre driver gives xf86-…)
        pacman -T "$p" >/dev/null 2>&1 && continue
        if pacman -Si "$p" >/dev/null 2>&1; then echo "$p" >> "$LOGDIR/.pkgs"
        else echo "note: package $p is not in your repositories; skipped"; fi
    done
    echo "to install: $(wc -l < "$LOGDIR/.pkgs") packages"
    cat "$LOGDIR/.pkgs"
    command -v cargo >/dev/null 2>&1 || grep -qx rust "$LOGDIR/.pkgs" || {
        echo "cargo (Rust) is needed to build Samurai and pacman cannot install rust here"; return 1; }
}
install_packages() {
    [ -s "$LOGDIR/.pkgs" ] || { echo "every package is already installed"; return 0; }
    # --needed: never reinstall; no -y: the package lists are not refreshed (no partial upgrade)
    # shellcheck disable=SC2046
    $AS_ROOT pacman -S --needed --noconfirm $(cat "$LOGDIR/.pkgs") || {
        echo "pacman could not install the packages. If it says a file was not found, your package"
        echo "lists are old: run 'sudo pacman -Syu' once, then run this installer again."
        return 1; }
}
check_needs() {
    for c in cargo git; do command -v $c >/dev/null 2>&1 || { echo "missing: $c"; return 1; }; done
    echo "X server: $( { X -version || Xorg -version; } 2>&1 | grep -m1 -i 'x server\|xlibre' || echo 'none yet')"
    for c in xinit alacritty dunst xsettingsd xss-lock nmcli wpctl brightnessctl gammastep maim; do
        command -v $c >/dev/null 2>&1 || echo "note: $c is not installed; its part of Samurai is off until it is"
    done
    python3 -c 'import PySide6.QtQuick' 2>/dev/null || echo "note: PySide6 is not installed: Settings and the login screen need it"
    return 0
}
build() { cd "$HERE" && SAMURAI_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo local) cargo build --release --locked; }
stage() { # everything is laid out in a staging folder first, then swapped in at once
    S=$LOGDIR/stage; rm -rf "$S"; mkdir -p "$S/bin" "$S/share" "$S/greeter" "$S/settings"
    for b in samurai samurai-wm shadow; do cp "$HERE/target/release/$b" "$S/bin/" || return 1; done
    cp "$HERE"/packaging/bin/* "$S/bin/" && chmod 0755 "$S/bin/"* || return 1
    cp "$HERE"/greeter/greeter.py "$HERE"/greeter/Greeter.qml "$S/greeter/" || return 1
    cp -r "$HERE"/settings/qml "$HERE"/settings/samurai_settings.py "$HERE"/settings/schema.json "$HERE"/settings/settings-index.json "$S/settings/" || return 1
    cp -r "$HERE"/packaging/share/. "$S/share/" || return 1
    cp "$HERE"/settings/settings-index.json "$S/share/" || return 1   # the overview searches settings
    (cd "$HERE" && git rev-parse HEAD 2>/dev/null) > "$S/share/COMMIT" || true
    find "$S" -name __pycache__ -prune -exec rm -rf {} +
    "$S/bin/samurai" version && "$S/bin/samurai" schema > /dev/null && "$S/bin/samurai-wm" --keys > /dev/null
}
place() {
    S=$LOGDIR/stage
    $AS_ROOT sh -c '
        set -e
        rm -rf /opt/samurai.new && mkdir -p /opt && cp -r "$1" /opt/samurai.new && chown -R root:root /opt/samurai.new
        [ ! -d /opt/samurai ] || { rm -rf /opt/samurai.old; mv /opt/samurai /opt/samurai.old; }
        mv /opt/samurai.new /opt/samurai
        mkdir -p /usr/local/bin /usr/share/xsessions /usr/share/wayland-sessions /usr/share/applications
        for b in samurai samurai-wm samurai-settings samurai-xserver samurai-key-pam; do ln -sfn /opt/samurai/bin/$b /usr/local/bin/$b; done
        # a shadow command of your own is kept
        [ -e /usr/local/bin/shadow ] && [ ! -L /usr/local/bin/shadow ] || ln -sfn /opt/samurai/bin/shadow /usr/local/bin/shadow
        install -m 0644 /opt/samurai/share/samurai.desktop /usr/share/xsessions/samurai.desktop
        install -m 0644 /opt/samurai/share/samurai-hyprland.desktop /usr/share/wayland-sessions/samurai-hyprland.desktop
        install -m 0644 /opt/samurai/share/samurai-settings.desktop /usr/share/applications/samurai-settings.desktop
        # the lock screen checks passwords with this; a file of your own is kept
        # a file of your own is kept, and so is one with key login on (its "# samurai-key" line)
        if [ ! -f /etc/pam.d/samurai-lock ] || { grep -q "installed by Samurai" /etc/pam.d/samurai-lock && ! grep -q "# samurai-key\$" /etc/pam.d/samurai-lock; }; then
            install -m 0644 /opt/samurai/share/pam.d/samurai-lock /etc/pam.d/samurai-lock
        fi
    ' sh "$S" && rm -rf "$S"
}
# the network: every Wi-Fi feature in Samurai goes through NetworkManager. Another manager (iwd,
# systemd-networkd, dhcpcd…) is only switched off when you agree (asked before the status line
# starts), or on a fresh system (--for-user). What was on is written down, and --uninstall turns
# it back on. Changes take effect at the next boot: nothing running is stopped now.
other_net() {
    command -v systemctl >/dev/null 2>&1 || return 0
    for u in iwd systemd-networkd dhcpcd connman netctl wicd; do
        systemctl is-enabled "$u.service" >/dev/null 2>&1 && printf '%s ' "$u"
    done
}
service_on() { # service_on UNIT LABEL: enable from the next boot, say so
    if systemctl is-enabled "$1" >/dev/null 2>&1; then echo "on: $2 (already)"
    elif $AS_ROOT systemctl enable "$1" >/dev/null 2>&1; then echo "on: $2 (from the next boot)"
    else echo "note: $2 could not be switched on ($1)"; fi
}
services() { # only enables (from the next boot); never starts or stops anything now
    command -v systemctl >/dev/null 2>&1 || return 0
    if systemctl list-unit-files NetworkManager.service >/dev/null 2>&1; then
        other=$(other_net | sed "s/ *$//")
        if [ -n "$other" ] && ! systemctl is-enabled NetworkManager.service >/dev/null 2>&1; then
            if [ "${SWITCH_NET:-0}" = 1 ]; then
                $AS_ROOT sh -c 'mkdir -p /etc/samurai && [ -f /etc/samurai/previous-network ] || echo "$1" > /etc/samurai/previous-network' sh "$other"
                for u in $other; do $AS_ROOT systemctl disable "$u.service" >/dev/null 2>&1 && echo "off: $u (NetworkManager takes over at the next boot)"; done
                service_on NetworkManager.service "NetworkManager"
            else
                echo "note: NetworkManager was not turned on because $other already manages the network; Samurai's Wi-Fi needs NetworkManager (run ./install.sh again and say yes to switch)"
            fi
        else
            service_on NetworkManager.service "NetworkManager"
        fi
    fi
    systemctl list-unit-files bluetooth.service >/dev/null 2>&1 && service_on bluetooth.service "Bluetooth"
    if systemctl list-unit-files power-profiles-daemon.service >/dev/null 2>&1; then
        if systemctl is-enabled tlp.service >/dev/null 2>&1 || systemctl is-enabled auto-cpufreq.service >/dev/null 2>&1; then
            echo "note: power profiles left off: tlp or auto-cpufreq already manages power"
        else
            service_on power-profiles-daemon.service "power profiles"
        fi
    fi
    # sound: PipeWire runs per user, started when an app needs it (sockets)
    for u in pipewire.socket pipewire-pulse.socket wireplumber.service; do
        systemctl --global is-enabled "$u" >/dev/null 2>&1 || $AS_ROOT systemctl --global enable "$u" >/dev/null 2>&1
    done
    systemctl --global is-enabled pipewire.socket >/dev/null 2>&1 && echo "on: sound (PipeWire)"
    return 0
}
user_files() {
    as_user xdg-user-dirs-update 2>/dev/null || true
    as_user /opt/samurai/bin/samurai apply
}
seal() { # the one-time seal (kept if this machine already has one) and the public decoy
    as_user /opt/samurai/bin/samurai seal show | grep -q '"forged":true' || as_user /opt/samurai/bin/samurai seal forge > /dev/null || return 1
    as_user /opt/samurai/bin/samurai seal decoy
}
greeter() { # exit 3 from the setup = a part is missing: a note, not a failed install
    $AS_ROOT /opt/samurai/bin/samurai-greeter-setup "$(as_user /opt/samurai/bin/samurai get look.ground 2>/dev/null | tr -d '"' | grep -x 'night\|paper' || echo night)" "${FOR:-$(id -un)}" && return 0
    [ $? = 3 ] && { echo "note: LOGIN SCREEN NOT SET UP (see the line above); fix that and run ./install.sh again"; return 0; }
    return 1
}
reload() { # a running Samurai picks the new version up; never fails the install
    if [ -n "${DISPLAY:-}" ] && [ "${XDG_CURRENT_DESKTOP:-}" = Samurai ]; then
        /opt/samurai/bin/samurai apply >/dev/null 2>&1 || true
        /opt/samurai/bin/samurai-wm msg restart >/dev/null 2>&1 || true
    fi
    return 0
}
remove() {
    $AS_ROOT sh -c '
        [ ! -x /opt/samurai/bin/samurai-greeter-setup ] || /opt/samurai/bin/samurai-greeter-setup --remove || true
        [ ! -x /opt/samurai/bin/samurai-key-pam ] || /opt/samurai/bin/samurai-key-pam disable || true
        ! grep -q "installed by Samurai" /etc/pam.d/samurai-lock 2>/dev/null || rm -f /etc/pam.d/samurai-lock
        rm -f /usr/share/xsessions/samurai.desktop /usr/share/wayland-sessions/samurai-hyprland.desktop /usr/share/applications/samurai-settings.desktop
        for b in samurai samurai-wm samurai-settings samurai-xserver samurai-key-pam shadow; do
            [ "$(readlink /usr/local/bin/$b 2>/dev/null)" != /opt/samurai/bin/$b ] || rm -f /usr/local/bin/$b
        done
        rm -rf /opt/samurai /opt/samurai.old /opt/samurai.new
        if [ -f /etc/samurai/previous-network ]; then
            for u in $(cat /etc/samurai/previous-network); do systemctl enable "$u.service" >/dev/null 2>&1; done
            systemctl disable NetworkManager.service >/dev/null 2>&1
            rm -f /etc/samurai/previous-network
            echo "network: the previous manager is back from the next boot"
        fi
    '
    # the app settings Samurai wrote (each carries its mark): the apps go back to their own defaults
    cfg=${XDG_CONFIG_HOME:-$HOME/.config}
    find "$cfg" -maxdepth 3 -type f -size -256k ! -path "$cfg/samurai/*" 2>/dev/null | while IFS= read -r f; do
        head -n 3 "$f" 2>/dev/null | grep -q "samurai: written by Samurai" && rm -f "$f" && echo "removed $f"
    done
    for rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
        [ -f "$rc" ] && grep -q "# samurai prompt" "$rc" && sed -i '/# samurai prompt$/d' "$rc" && echo "took the prompt out of $rc"
    done
    command -v gsettings >/dev/null 2>&1 && gsettings reset org.gnome.desktop.interface color-scheme 2>/dev/null
    return 0
}

if [ "$MODE" = uninstall ]; then
    TOTAL=1; step "removing Samurai" remove; bar "removed"; [ $TTY = 1 ] && echo
    echo " Samurai is removed. Your previous login screen is back from the next boot."
    echo " Apps use their own settings again. Packages were left installed; your Samurai settings and"
    echo " seal are kept in ~/.config/samurai (delete that folder to forget them)."
    exit 0
fi
TOTAL=$((7 + 2 * PACKAGES + GREETER))
# switching the network to NetworkManager is asked now, before the status line starts: never
# silently, never over SSH (the connection you are using could go at the next boot)
SWITCH_NET=0
OTHER_NET=$(other_net | sed "s/ *$//")
if [ -n "$OTHER_NET" ] && command -v systemctl >/dev/null 2>&1 && ! systemctl is-enabled NetworkManager.service >/dev/null 2>&1; then
    if [ -n "$FOR" ]; then SWITCH_NET=1
    elif [ $TTY = 1 ] && [ -z "${SSH_CONNECTION:-}" ] && [ "$MODE" != update ]; then
        echo " Your network is managed by: $OTHER_NET"
        echo " Samurai's Wi-Fi (settings, login screen, campus networks) needs NetworkManager. Switch to it"
        printf " from the next boot? (your saved Wi-Fi passwords in the old one are not moved) [y/N] "
        read -r a < /dev/tty || a=""
        case "$a" in y|Y|yes) SWITCH_NET=1;; esac
    fi
fi
# ask for the password once, before the status line starts
if [ -z "$FOR" ] && [ "$AS_ROOT" = sudo ] && [ $TTY = 1 ]; then
    echo " Samurai needs your password once, to install packages and put files in /opt and /etc."
    sudo -v || { echo " no password given: nothing was changed"; exit 1; }
fi
if [ $PACKAGES = 1 ]; then
    step "working out which packages are missing" plan_packages
    watch "installing packages (asks for your password)" install_packages
fi
step "checking this machine" check_needs
watch "building Samurai" build
step "preparing files" stage
step "installing Samurai to /opt/samurai" place
step "switching on network, Bluetooth, sound and power" services
step "writing app settings and the terminal style" user_files
step "forging this machine's seal" seal
[ $GREETER = 0 ] || step "setting up the login screen" greeter
step "finishing" reload
bar "done"; [ $TTY = 1 ] && echo
rm -f "$LOGDIR/.pkgs"
NOTES=$(grep -c '^note:' "$LOG" || true)
if [ "$MODE" = update ]; then echo " Samurai is up to date."; else
    if [ -n "$FOR" ]; then echo " Samurai is installed for $FOR. It starts at the next boot."
    else echo " Samurai is installed. Log out and choose “Samurai” at the login screen."; fi
    if grep -q "^login screen: set" "$LOG"; then echo " The Samurai login screen replaces your current one from the next boot."
    elif [ $GREETER = 1 ]; then echo " ! The login screen was NOT set up: $(grep -B1 '^note: LOGIN SCREEN' "$LOG" | head -1)"; fi
    echo " X server: $(/opt/samurai/bin/samurai-xserver status | head -n1 | sed 's/^X server: //'). To try XLibre: samurai-xserver xlibre"
fi
ON=$(grep '^on: ' "$LOG" | sed 's/^on: //' | tr '\n' ',' | sed 's/,$//; s/,/, /g')
[ -z "$ON" ] || echo " Switched on: $ON."
[ "$NOTES" = 0 ] || { echo " $NOTES notes (parts skipped or left off):"; grep '^note:' "$LOG" | sed 's/^note: /   · /'; }
exit 0
