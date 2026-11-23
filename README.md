# Samurai 侍

An X11 desktop for Arch Linux with its own window manager. There is no taskbar and there are no
workspaces: one key, **Super**, opens an overview of every window with a search line. Windows
float, snap and tuck away with the same animations everywhere, because the window manager draws
the screen itself (it is its own compositor, like Hyprland or niri on Wayland). It runs on
**X.Org** or **XLibre**.

- **Super opens everything.** A tap on Super shows your open windows as cards and, under them,
  every installed app with its own icon (as a grid, or a compact list: Settings › Look). Click
  one, or move with the arrow keys and press Enter; right-click an app to pin it first. There is
  no search bar in the way, but typing still filters at once: windows, apps, settings, a sum
  (`= 12*7`), or `>` and a command. Alt+Tab goes back and forth between windows as usual.
- **No taskbar, no workspaces.** A tucked window (`Super+M`) waits on a thin shelf at the
  bottom of the overview. Snap with `Super+arrows`, or drag a window to an edge.
- **Ink, made once.** The wallpaper is a marbled ink drawing made on your machine from random
  numbers that are thrown away after drawing: nobody, not even Samurai, can make it again. Its
  mark (like `3F2A · 91C0 · 7B44`) sits quietly in a corner of the empty desktop.
- **Night and paper.** Two grounds and one accent colour (indigo, moss, rust, plum or slate).
  Switch with `Super+Shift+B`, or let it follow the sun.
- **Apps look like they belong.** GTK, Qt (5 and 6) and the terminal get the same colours,
  fonts, icons and size. Samurai only writes files it made itself (they start with
  `samurai: written by Samurai`); a config of your own is left alone.
- **Settings that read plainly.** Ten pages: Look, Displays, Network, Sound, Keyboard, Mouse,
  Apps, Power, Notifications, System. Every setting is searchable from Super too.
- **Compact quick controls.** In the overview: small tiles for Wi-Fi, Bluetooth, Quiet, Night
  light, night/paper and screenshots; a row each for Wi-Fi and Bluetooth that opens into the
  list of networks or paired devices (a secured network asks for its password right there);
  slim volume and brightness sliders.
- **Wi-Fi before you log in.** The login screen has a Wi-Fi button: pick a network, type its
  password, or sign in to UC-Secure / eduroam, without logging in first.
- **Campus Wi-Fi fixed in Settings.** UC-Secure / AirUC-Secure and eduroam set up and repair in
  Settings › Network, with a log you can read.
- **Its own login screen and lock screen**, in the same design. The lock checks your password
  with PAM; the login runs through greetd and falls back to greetd's text login if it ever
  cannot start.

## Install

On Arch Linux, as your own user (it asks for your password once):

```sh
curl -fsSL https://raw.githubusercontent.com/namidelacruz/samurai/main/install.sh | sh
```

or from a downloaded or cloned copy:

```sh
./install.sh
```

The installer shows one status line naming what it is doing right now (`installing packages ·
12 of 40: alacritty`, `building Samurai · compiling x11rb`, …); every line of detail goes to
`~/.local/state/samurai/install.log`. If a step fails it stops there, shows the last lines of
the log, and changes nothing after that step.

It installs every supported feature:

| Part | Packages |
| --- | --- |
| X server (only if there is none: X.Org or XLibre is kept) | `xorg-server`, `xf86-input-libinput` |
| Starting X, screens, input | `xorg-xinit`, `xorg-xauth`, `xorg-xrandr`, `xorg-xinput`, `xorg-xset`, `xorg-xsetroot`, `xorg-xrdb` |
| Terminal, notices, idle lock, password prompts | `alacritty`, `dunst`, `libnotify`, `xss-lock`, `polkit-gnome` |
| Look of apps | `xsettingsd`, `adw-gtk-theme`, `papirus-icon-theme`, `qt5ct`, `qt6ct`, `inter-font`, `ttf-jetbrains-mono`, `noto-fonts`, `noto-fonts-cjk`, `noto-fonts-emoji` |
| Night light, screenshots, clipboard | `gammastep`, `maim`, `xclip`, `xdotool` |
| Network, sound, brightness, media keys | `networkmanager`, `pipewire`, `wireplumber`, `pipewire-pulse` (not if PulseAudio is installed), `brightnessctl`, `playerctl` |
| Settings and login screen | `pyside6`, `greetd` |
| Portals, disks, folders | `xdg-desktop-portal`, `xdg-desktop-portal-gtk`, `xdg-utils`, `xdg-user-dirs`, `gvfs`, `udisks2` |
| Default apps | `firefox`, `thunar`, `mousepad`, `imv`, `zathura` (+ `zathura-pdf-mupdf`), `mpv`, `thunderbird`, `file-roller` |
| Building Samurai | `git`, `rust` (or your rustup), `gcc`, `pkgconf` |

Packages come only from the repositories pacman already uses, with `--needed` (nothing is
reinstalled, removed or replaced) and without refreshing the package lists. If pacman says a file
was not found, run `sudo pacman -Syu` once and start the installer again.

```sh
./install.sh --no-greeter    # leave your login screen as it is
./install.sh --no-apps       # skip the default apps
./install.sh --no-packages   # install no packages; you manage them
./install.sh --uninstall     # put back what was there before
samurai update install       # later: fetch the newest version and install it
```

Then log out and choose **Samurai**. With the login screen set up, it shows from the next boot.

What the installer changes, and what `--uninstall` puts back:

- `/opt/samurai` (the programs), links in `/usr/local/bin`, `/usr/share/xsessions/samurai.desktop`,
  `/usr/share/applications/samurai-settings.desktop`, `/etc/pam.d/samurai-lock`.
- The login screen: `/etc/greetd/config.toml` (the old one is kept as
  `config.toml.before-samurai`) and which login manager starts at boot (the previous one is
  remembered and comes back). Nothing running is stopped or restarted.
- NetworkManager is turned on for the next boot, but only if nothing else manages the network.
- App settings Samurai writes under `~/.config` (each carries its mark), and one line in
  `~/.bashrc` for the prompt (marked `# samurai prompt`).
- It never touches the bootloader, the initramfs or firmware settings.

`--uninstall` leaves packages installed and keeps your Samurai settings and seal in
`~/.config/samurai`; delete that folder to forget them.

## With archinstall (while installing Arch)

archinstall can run commands inside the new system after it has installed it, as root. Samurai's
installer has a mode for that: `--for-user NAME` installs everything for the user NAME without
asking for a password, and its log goes to `/var/log/samurai/install.log`.

1. Start `archinstall` as usual. Set up the disk, a user account (the one that will use Samurai),
   audio (PipeWire) and the network (NetworkManager). Leave the profile as **Minimal**: Samurai
   brings its own desktop and login screen.
2. Choose **Save configuration** and save `user_configuration.json`, then leave archinstall.
3. Add Samurai to the end of that file (replace `nami` with your user name):

   ```json
   "custom_commands": [
       "pacman -S --needed --noconfirm git",
       "curl -fsSL https://raw.githubusercontent.com/namidelacruz/samurai/main/install.sh | sh -s -- --for-user nami"
   ]
   ```

   Older archinstall versions spell the key `"custom-commands"`; use whichever spelling the
   saved file already uses for its other keys.
4. Install with that file: `archinstall --config user_configuration.json`.

After the first boot, the Samurai login screen greets you. The same options work here as in a
normal install: add `--no-apps` or `--no-greeter` after `--for-user nami` if you want them.

The same line works from any root shell on a system that is already installed, for example
inside `arch-chroot`:

```sh
curl -fsSL https://raw.githubusercontent.com/namidelacruz/samurai/main/install.sh | sh -s -- --for-user nami
```

## X.Org or XLibre

Samurai works the same on both: it needs only what every X server since X.Org 1.20 has
(Composite, Damage, Render, XFixes, Shape, RandR, XTEST). The installer keeps whichever server you
have and only installs X.Org when there is none.

```sh
samurai-xserver status   # which one runs now
samurai-xserver xlibre   # guided switch to XLibre
samurai-xserver xorg     # and back
```

The switch replaces system packages, so it shows the plan and asks first, and pacman asks again
before it removes anything. It uses a pacman repository that carries XLibre if you have added one,
or an AUR helper (paru, yay) to build XLibre's AUR packages; it maps each installed
`xf86-input-*` / `xf86-video-*` driver to its `xlibre-*` version. It never adds a repository or
a signing key for you. If X does not start afterwards, the login falls back to a text login, and
`samurai-xserver xorg` switches back. Settings › System shows which server runs and has the same
button.

## If the login screen does not start

The login screen never drops you to a text login. If it cannot start, it tries again; after two
quick failures it switches to **safe mode** (no wallpaper, no animations, drawing done without the
graphics card), which gets past most driver problems; if that fails too it keeps retrying, waiting
a little longer each time (up to 30 seconds). Each attempt is logged in
`/tmp/samurai-greeter.<id>.log`, with the five runs before it kept as `.1` … `.5`. Before
Samurai makes greetd your login manager, it checks that the login screen loads, in both modes,
as greetd's own user; if it does not, your old login manager stays.

If Samurai's desktop itself cannot start, you go back to the login screen. Started by hand from a
console (`samurai-session`), it prints what went wrong and returns to the shell.

### The "Cannot open log file" error

**Symptom:** instead of the login screen you get a text login, or a screen ending in

```
(EE) Cannot open log file "//.local/share/xorg/Xorg.0.log"
xinit: giving up
xinit: unable to connect to X server: Connection refused
```

**Cause:** the login screen runs as greetd's own user, whose home folder is often `/`. X writes
its log into the home folder and refuses to start when it cannot. Fixed in commit `70f2cc1`;
installs from before that have the problem.

**Quick fix** (one line, from a console: `Ctrl+Alt+F2`, log in). It replaces only the file that
starts X, then restarts the login screen. Run it from that second console, because restarting
greetd closes whatever is running on the login screen's console:

```sh
sudo curl -fsSL https://raw.githubusercontent.com/namidelacruz/samurai/main/packaging/bin/samurai-x -o /opt/samurai/bin/samurai-x && sudo chmod 755 /opt/samurai/bin/samurai-x && sudo systemctl restart greetd
```

Without a network, create the folder X is looking for instead (`greeter` is the login screen's
user; `grep user /etc/greetd/config.toml` shows its name if it differs):

```sh
sudo install -d -o greeter /.local/share/xorg && sudo systemctl restart greetd
```

Delete `/.local` again once you have installed the update below.

**Proper fix:** update Samurai, which brings the same fix and everything else:

```sh
samurai update install
```

If the login screen still does not appear, its log is the newest `/tmp/samurai-greeter.*.log`
(`ls -t /tmp/samurai-greeter.*.log | head -1`) and the X server's log is in
`/run/user/<id>/samurai-xdata/xorg/` (or `/tmp/samurai-xdata-<id>/xorg/`).

## Wi-Fi on the login screen

The login screen runs as greetd's own user, which normally may not change the network. Setting
up the login screen therefore adds one small permission file,
`/etc/polkit-1/rules.d/50-samurai-greeter-wifi.rules`, that lets **that user only** scan for
Wi-Fi and join a network through NetworkManager, nothing else. Networks joined there are for the
whole machine, so they stay connected after you log in. Passwords go to NetworkManager directly
and are never written to a log. `samurai greeter remove` (or `./install.sh --uninstall`) deletes
the file again; a file of your own with that name is never touched. Without NetworkManager the
Wi-Fi button simply does not appear.

## If the login screen is too small or too big

The login screen runs before anyone has logged in, so it cannot read your own size setting while
it runs. Instead it uses a copy, `"scale"` in `/etc/samurai/greeter.json`, made when the login
screen was set up. On a high-resolution screen it can therefore come up tiny, or stay at an old
size after you change Settings › Look › Size.

**Fix:** set your size in Settings › Look › Size first, then copy it to the login screen (asks for
your password):

```sh
samurai greeter apply
```

Or set it directly, for example 1.5 for a 150 % screen (any value from 1 to 2.5):

```sh
sudo sed -i 's/"scale": *[0-9.]*/"scale": 1.5/' /etc/samurai/greeter.json
```

Either way the new size shows the next time the login screen starts (log out, or
`sudo systemctl restart greetd` from a second console).

## If Wi-Fi or the network does not work

Samurai's Wi-Fi settings and the campus fixes go through NetworkManager. Run these from a console
or a terminal; the first shows what is wrong:

```sh
systemctl is-enabled NetworkManager; systemctl is-active NetworkManager
nmcli general status; nmcli device status
rfkill list
```

- **NetworkManager is not running** (`disabled` / `inactive`). The installer only turns it on when
  nothing else manages the network, so if archinstall set up `iwd`, `systemd-networkd` or `dhcpcd`
  it was left off. Two network managers at once fight over the Wi-Fi, so switch the others off and
  NetworkManager on, in one line. Do this at the machine itself, not over SSH: the network
  drops for a moment.

  ```sh
  sudo systemctl disable --now iwd systemd-networkd dhcpcd 2>/dev/null; sudo systemctl enable --now NetworkManager
  ```

- **Wi-Fi is switched off** (`rfkill` says `Soft blocked: yes`, or `nmcli` says the radio is
  disabled):

  ```sh
  rfkill unblock wifi; nmcli radio wifi on
  ```

  `Hard blocked: yes` is a hardware switch or a key on the keyboard (often `Fn` plus an aerial
  symbol); no command changes that.

- **The Wi-Fi card shows as `unmanaged`** in `nmcli device status`: something in
  `/etc/NetworkManager/conf.d/` tells NetworkManager to leave it alone. Look there for an
  `unmanaged-devices=` line, remove it, then `sudo systemctl restart NetworkManager`.

- **UC-Secure / AirUC-Secure or eduroam stops connecting** (often after a system update): Settings
  › Network › Repair this connection, or from a terminal:

  ```sh
  shadow wifi repair
  shadow wifi log      # what the last attempt said, in plain words
  shadow wifi device   # is the Wi-Fi card there, with its firmware and driver
  ```

- **A saved network keeps failing:** forget it and join again (Settings › Network, or
  `nmcli connection delete "NAME"`).

## Keys

At the first login a short setup asks which shortcuts you want: **Samurai's** or
**Hyprland-style**, and lets you change any key there (a key given to two actions is pointed out
before you go on). It also asks night or paper, the size of everything, icons or a list for the
apps, and offers Wi-Fi. Skip it if you like; Settings › Keyboard opens it again.

| Action | Samurai | Hyprland-style |
| --- | --- | --- |
| Overview and apps | `Super` (tap) | `Super+R` (a tap on Super works too) |
| Terminal | `Super+Return` | `Super+Q` |
| Close the window | `Super+Q` | `Super+C` |
| Files / browser / Settings | `Super+E` / `Super+B` / `Super+,` | the same |
| Full screen / fill the screen | `Super+F` / `Super+Up` | `Super+F` / `Super+Shift+F` |
| Float or tile this window | `Super+V` | `Super+V` |
| Minimize (tuck away) / bring back | `Super+M` / `Super+Shift+M` | `Super+H` / `Super+Shift+M` |
| Show minimized windows | `Super+T` | `Super+T` |
| Move the window left / right | `Super+Left` / `Super+Right` | `Super+Shift+Left` / `Super+Shift+Right` |
| Focus the window left / right / up / down | `Super+Alt+arrows` | `Super+arrows` |
| Quarters (floating) | `Super+Shift+arrows` | (moves the window instead) |
| Next / previous window | `Alt+Tab` / `Alt+Shift+Tab` | the same |
| Lock / log out | `Super+L` / `Super+Shift+Escape` | `Super+L` / `Super+M` |
| Night / paper | `Super+Shift+B` | the same |
| Screenshot of an area / the screen / the window | `Super+Shift+S`, `Print`, `Alt+Print` | the same |

With windows floating, moving left or right puts the window on that half of the screen. Your own
lines in Settings › Keyboard always win over either set.

## Commands

```sh
samurai get KEY | set KEY VALUE | reset KEY | dump    # settings
samurai apply                                          # write every app's settings again
samurai ground toggle | theme night|paper              # the ground
samurai seal show | forge --erase-current              # the ink wallpaper
samurai status | version | update check | update install
samurai greeter apply | remove                         # the login screen
samurai-wm msg ACTION                                  # tell the window manager (samurai-wm msg actions lists them)
samurai-xserver status | xlibre | xorg
```

## Tests

```sh
sh tests/run.sh
```

runs the Rust unit tests, checks every shell script parses, runs the window manager in a virtual
X server (Xvfb) and drives it with key presses (windows managed, snapped, tucked, the Super
overview, restart keeping windows), renders every Settings page offscreen, and, with
`FAKEGREET=/path/to/fakegreet` (greetd's own test server), logs in through the login screen with a
wrong and then a right password. None of this is a test on real hardware or a real login.

## Layout of the source

| Folder | What |
| --- | --- |
| `wm/` | `samurai-wm`: window manager, compositor, overview, lock (Rust, x11rb) |
| `samurai/` | `samurai`: settings store, app theming, ink seal, session start (Rust) |
| `settings/` | Settings app (PySide6 + QML) and the schema it is built from |
| `greeter/` | The login screen (PySide6 + QML, speaks greetd's protocol) |
| `services/` | `shadow`: Wi-Fi and campus network helper |
| `packaging/` | Session, login and X server scripts; desktop entries; PAM file |
| `docs/DESIGN.md` | The design: colours, type, measures, motion, the ink |
