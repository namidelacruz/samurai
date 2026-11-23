# Samurai 侍

An X11 desktop for Arch Linux with its own window manager. There is no taskbar and there are no
workspaces: one key, **Super**, opens an overview of every window with a search line. Windows
float, snap and tuck away with the same animations everywhere, because the window manager draws
the screen itself (it is its own compositor, like Hyprland or niri on Wayland). It runs on
**X.Org** or **XLibre**.

- **Super opens everything.** A tap on Super shows every open window as a card, plus one search
  line: type to find a window, an app, a setting, or a sum (`= 12*7`), or `>` and a command.
  Enter opens it. Alt+Tab goes back and forth between windows like you are used to.
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

## Keys

| Key | Does |
| --- | --- |
| `Super` (tap) | Overview and search |
| `Alt+Tab` / `Alt+Shift+Tab` | Next / previous window |
| `Super+Return` / `Super+E` / `Super+B` / `Super+,` | Terminal / files / browser / Settings |
| `Super+Q` | Close the window |
| `Super+Up` / `Super+Down` | Fill the screen / back (or tuck away) |
| `Super+Left` / `Super+Right` | Left / right half |
| `Super+Shift+arrows` | Quarters |
| `Super+C` | Centre |
| `Super+F` | Full screen |
| `Super+M` / `Super+Shift+M` | Tuck away / bring back |
| `Super+L` | Lock |
| `Super+Shift+B` | Night / paper |
| `Super+Shift+S`, `Print`, `Alt+Print` | Screenshot of an area / the screen / the window |
| `Super+Shift+Escape` | Log out |

All of them can be changed in Settings › Keyboard.

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
