# Samurai 侍

A desktop for Hyprland and Quickshell. Minimal brutalism, Japanese vocabulary, black or white
with one Pantone red. **The frame is the desktop:** one line runs round the screen, and the bar,
the rail and every panel grow out of it.

- **Two grounds.** Black or white, switched live (`Mod+Shift+B`). Nothing but ink and ground trade places.
- **一期一会 — a one-time wallpaper.** Drawn once per machine from 32 random bytes, for both
  grounds; the bytes are destroyed after drawing. It cannot be drawn again, by you or by Samurai.
- **Settings hub** (`Mod+,`): 24 pages, 197 settings, searchable, saved as you go, applied live,
  including your own keybinds with clash detection.
- **Minimized windows go to the rail**, so the bar stays free.
- **UC-Secure / AirUC-Secure and eduroam** setup and repair are fixed in Settings › Connect.
- **Incognito** (`Mod+Shift+I`): hides names, titles, network, notices and the wallpaper, and
  tilts the grid so the mode is unmistakable.
- **Lock screen and login screen** in the same design.
- **Design document:** [docs/DESIGN.md](docs/DESIGN.md) has everything needed to rebuild the
  look as a website.

## Install

Needs Hyprland 0.57 or newer (Lua config), Quickshell, greetd (only for the login screen), a
Rust toolchain and git. Fonts: IBM Plex Mono and Noto Sans CJK JP.

```sh
./install.sh             # one status bar; detail in ~/.local/state/samurai/install.log
./install.sh --greeter   # also set up the Samurai login screen
./install.sh --uninstall # remove it; your settings and ~/.config/hypr stay
```

Log out and pick **Samurai**. If you already have your own `~/.config/hypr/hyprland.lua` it is
left alone and the installer tells you the one line to add.

The installer writes to `/opt/samurai`, `/usr/local/bin`, two `.desktop` files, and your own
config folders. With `--greeter` it also writes `/etc/greetd/config.toml` (the old one is kept
as `config.toml.before-samurai`) and `/etc/samurai/`. It never touches the bootloader, the
initramfs or firmware settings.

**If the login screen ever fails to appear:** switch to another console (`Ctrl+Alt+F2`), log in,
and run `sudo /opt/samurai/bin/samurai-greeter-setup --remove`.

## Keys

`Mod` is Super unless you change it. `Mod+/` shows the full list.

| | | | |
|---|---|---|---|
| `Mod+Space` | Start | `Mod+Return` | Terminal |
| `Mod+A` | Quick settings | `Mod+Q` | Close window |
| `Mod+,` | Settings | `Mod+M` / `Mod+Shift+M` | Minimize to the rail / bring back |
| `Mod+Shift+B` | Black / white | `Mod+1…0` | Workspace |
| `Mod+Shift+I` | Incognito | `Mod+Alt+L` | Lock |

## What is and is not checked

Run `tests/run.sh`. It covers: 47 Rust tests; the generated Hyprland Lua (syntax with `luac`,
then a full run against a stand-in `hl`); the seal being refused a second time; a render of 35
scenes of the interface (both grounds, Incognito, lock, login, every settings page) that fails
on any QML warning; and a wiring check that loads the real `shell.qml` and greeter against
stand-in Quickshell modules and drives settings, panels, lock and login.

**None of that is Hyprland, Quickshell or hardware.** Samurai has not been run inside a real
Hyprland session, against the real Quickshell, on a real GPU, with real greetd/PAM, or on a
real campus network. Expect to fix things on first boot; the session is built so that a broken
shell still leaves you a working Hyprland with a terminal on `Mod+Return`.

## Not built yet

Shown greyed out in the hub as 未 NOT BUILT YET (35 settings): window title strips, tray and
media in the bar, on-screen volume display, desktop widgets other than the clock (calendar,
system, weather, visualizer), voice typing, hotspot, GPU mode and passthrough writing
(`samurai gpu check` only reports readiness), file search in Start, power confirmation, lock
screen notices and media, idle dimming, shell motion. The store and plugins pages work from the
command line through a plain list, not a gallery.

## Layout

```
samurai/        the `samurai` command: settings, Hyprland config generator, keybinds, seal, updates
services/core/  `shadow`: the Wi-Fi campus setup, repair and diagnosis, unchanged from Shadow
shell/          the Quickshell interface (lib, frame, hub, lock, greeter) and the settings schema
packaging/      session files and the login-screen setup script
tools/schema.py the source of shell/settings.schema.json
docs/DESIGN.md  the design, reproducible
tests/          run.sh, the Lua stand-in, the QML render and wiring checks
```
