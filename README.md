# Samurai 侍

A desktop for niri and Quickshell. Minimal brutalism, Japanese vocabulary, black or white
with one Pantone red. **The frame is the desktop:** one line runs round the screen, and the bar,
the rail and every panel grow out of it.

- **Two grounds.** Black or white, switched live (`Mod+Shift+B`). Nothing but ink and ground trade places.
- **一期一会 — a one-time wallpaper.** Drawn once per machine from 32 random bytes, for both
  grounds; the bytes are destroyed after drawing. It cannot be drawn again, by you or by Samurai.
- **Settings hub** (`Mod+,`): 24 pages, 181 settings, searchable, saved as you go, applied live,
  including your own keybinds with clash detection.
- **Minimized windows go to the rail**, so the bar stays free. (niri has no minimize: the window
  is parked on a workspace named `min`, "the shelf", which the bar leaves out.)
- **UC-Secure / AirUC-Secure and eduroam** setup and repair are fixed in Settings › Connect.
- **Incognito** (`Mod+Shift+I`): hides names, titles, network, notices and the wallpaper, and
  tilts the grid so the mode is unmistakable.
- **Lock screen and login screen** in the same design.
- **Design document:** [docs/DESIGN.md](docs/DESIGN.md) has everything needed to rebuild the
  look as a website.

## Install

Needs niri 25.11 or newer (written against 26.04; per-app blur needs 26.04), Quickshell, greetd
for the login screen, a Rust toolchain and git. Fonts: IBM Plex Mono and Noto Sans CJK JP.

```sh
./install.sh              # one status bar; detail in ~/.local/state/samurai/install.log
./install.sh --no-greeter # leave your login screen as it is
./install.sh --keep-niri-config  # do not make Samurai niri's default
./install.sh --uninstall  # remove it; settings and seal stay, old niri config and login screen come back
```

Log out and pick **Samurai**.

**Samurai becomes niri's default.** Samurai's config lives in `~/.config/samurai/niri/`. The
installer replaces `~/.config/niri/config.kdl` with one `include` line pointing there, so every
niri session, however it is started, is Samurai. Your previous `config.kdl` is kept beside it as
`config.kdl.before-samurai` and `--uninstall` puts it back. It is not merged in: its binds would
clash with Samurai's. Anything you want from it goes in `~/.config/samurai/niri/user.kdl`.
niri checks the new file before it replaces the old one; if niri refuses it, yours stays.
If niri is already running when you install, it switches to Samurai on the spot.

A config niri refuses never replaces a working one: every change is checked with
`niri validate` first, and a setting whose result is refused goes back to its old value. If
your `user.kdl` is what niri refuses, the session starts Samurai without it; if even that
fails, it starts niri with niri's own config. Why is written to
`~/.local/state/samurai/session.log`.

### The login screen

The installer sets it up unless you pass `--no-greeter`. It installs greetd with pacman or dnf
if it is missing, writes `/etc/greetd/config.toml` (the old one is kept as
`config.toml.before-samurai`) and `/etc/samurai/`, and makes greetd the login manager **from the
next boot** (the one it replaces is remembered in `/etc/samurai/previous-display-manager`).
Nothing running is stopped. If a part is missing (greetd, agreety, Quickshell, niri), nothing is
changed and the installer says so in its last lines.

The login screen is Quickshell inside a bare niri. If that cannot start, or dies within eight
seconds, greetd's own text login (`agreety`) takes over on the same screen, so a broken
graphical greeter cannot lock you out. To go back by hand:
`sudo /opt/samurai/bin/samurai-greeter-setup --remove` (from another console: `Ctrl+Alt+F2`).

The installer never touches the bootloader, the initramfs or firmware settings.

## Keys

`Mod` is Super unless you change it. `Mod+/` shows the full list.

| | | | |
|---|---|---|---|
| `Mod+Space` | Start | `Mod+Return` | Terminal |
| `Mod+A` | Quick settings | `Mod+Q` | Close window |
| `Mod+,` | Settings | `Mod+M` / `Mod+Shift+M` | Minimize to the rail / bring back |
| `Mod+Shift+B` | Black / white | `Mod+1…0` | Workspace |
| `Mod+O` | Overview | `Mod+R` / `Mod+-` / `Mod+=` | Window width |
| `Mod+Shift+I` | Incognito | `Mod+Alt+L` | Lock |

## What is and is not checked

Run `tests/run.sh`. It covers: 47 Rust tests; the generated niri config, and every niri-backed
setting at its extremes (88 values, plus hostile text in every free-text field), judged by
niri's own config parser; the seal being refused a second time; a render of 35 scenes of the
interface (both grounds, Incognito, lock, login, every settings page) that fails on any QML
warning; and a wiring check that loads the real `shell.qml` and greeter against stand-in
Quickshell modules and a stand-in `niri` command, and drives settings, windows, minimize,
panels, lock and login. The installer was run end to end as an ordinary user on a test system
with an existing login manager, then reinstalled and uninstalled.

The config parser used here is niri's `niri-config` crate at 25.11, 26.04 and main, built as a
small program; it is the same code `niri validate` runs, but it is not a running niri.

**None of that is niri itself, Quickshell or hardware.** Samurai has not been run inside a real
niri session, against the real Quickshell, on a real GPU, with real greetd/PAM, or on a real
campus network. Expect to fix things on first boot.

## Not built yet

Shown greyed out in the hub as 未 NOT BUILT YET (35 settings): window title strips, tray and
media in the bar, on-screen volume display, desktop widgets other than the clock (calendar,
system, weather, visualizer), voice typing, hotspot, GPU mode and passthrough writing
(`samurai gpu check` only reports readiness), file search in Start, power confirmation, lock
screen notices and media, idle dimming, shell motion. The store and plugins pages work from the
command line through a plain list, not a gallery.

## Layout

```
samurai/        the `samurai` command: settings, niri config generator, keybinds, seal, updates
services/core/  `shadow`: the Wi-Fi campus setup, repair and diagnosis, unchanged from Shadow
shell/          the Quickshell interface (lib, frame, hub, lock, greeter) and the settings schema
packaging/      the session starter, the login-screen runner and setup script, .desktop files
tools/schema.py the source of shell/settings.schema.json
docs/DESIGN.md  the design, reproducible
tests/          run.sh, the settings sweep, a stand-in niri command, the QML render and wiring checks
```
