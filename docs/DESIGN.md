# Samurai — design document

The look and behaviour of the Samurai X11 desktop, with the numbers needed to rebuild it.
Sources of truth in code: colours and measures in `wm/src/theme.rs`, the ink in
`samurai/src/seal.rs`, settings in `tools/schema.py` (→ `settings/schema.json`).

## 1. The idea

A desktop that gets out of the way and is unmistakably its own.

- **One key.** Super opens the overview: every window as a card and one search line. There is no
  taskbar, no dock, no workspaces, no tray. Anything you would look for in a bar (time, battery,
  network) shows in the overview or in a small pill on the empty desktop.
- **Windows float.** They snap to halves and quarters, fill the screen, or tuck away onto a shelf.
  Nothing tiles by itself.
- **Ink on two grounds.** Night (charcoal) and paper (warm white), one accent, and a wallpaper of
  marbled ink that is made once per machine and can never be made again.
- **Calm motion.** Windows grow out of where they came from and fold back to where they go.

## 2. Rules

1. Two grounds and one accent. Every other colour is a tint of these; the one exception is the
   close button, which turns red under the pointer.
2. One line weight (1 px at scale 1) for every edge and divider.
3. Corners round at 10 px for windows, 14 px for panels, full for pills.
4. A window's title strip takes the window's own top colour, sampled once it has drawn, so a
   dark editor gets a dark strip and a light page a light one. Apps that draw their own title
   (client-side decorations) get no strip at all.
5. Words over icons; plain words over jargon ("Tuck away", not "minimize to tray").
6. Nothing moves unless the user did something.

## 3. Colour

| Role | Night | Paper |
| --- | --- | --- |
| ground (desktop, behind everything) | `#101114` | `#ECE8DF` |
| surf (cards, Settings pages) | `#1A1C21` | `#FAF8F4` |
| chrome (title strips, fields) | `#1F2228` | `#F2EFE9` |
| raised (hover, pressed) | `#252930` | `#FFFFFF` |
| line | `#30343C` | `#DCD6CA` |
| text | `#ECEBE6` | `#1B1B1D` |
| dim (secondary text) | `#A3A098` | `#5F5C56` |
| glass (overview panel) | `rgba(24,26,31,.84)` | `rgba(250,248,244,.87)` |
| scrim (behind the overview) | `rgba(9,10,12,.62)` | `rgba(236,232,223,.60)` |
| shadow | `rgba(0,0,0,.59)` | `rgba(60,48,28,.27)` |

Accents (paper / night; night uses a lighter tint so it reads on charcoal):

| Name | Paper | Night |
| --- | --- | --- |
| indigo (default) | `#2F4B8F` | `#93ABEB` |
| moss | `#4A6B3A` | `#9CC28A` |
| rust | `#9A4A2A` | `#E39A78` |
| plum | `#6B3F73` | `#C9A0D6` |
| slate | `#44505E` | `#A9B6C6` |

Text on the accent: `#0E1424` at night, white on paper. The soft accent (selected rows, focus
wash) is the accent at 16 % (night) or 10 % (paper).

The terminal's sixteen colours and the Qt palette are generated from the same table, so every app
on screen agrees.

## 4. Type

- **Inter** for everything you read; 13.5 px for body text in Settings and the login screen.
- **JetBrains Mono** for the terminal (11 pt by default) and anything typed as code.
- Weights: regular and semibold only.
- Scale (Settings › Look › Size) multiplies every measure; `Xft.dpi` is set to 96 × scale so apps
  follow.

## 5. Measures

At scale 1 (everything multiplies with Settings › Look › Size):

| Thing | Size |
| --- | --- |
| Title strip | 36 px |
| Grab margin round every window (for resizing) | 6 px, invisible |
| Shadow reach | 40 px |
| Gap between snapped windows and the screen edge | 8 px (a setting) |
| Overview search line | up to 640 px wide; result rows 52 px |
| The mark | bottom left of the empty desktop, in a 56 px band; the time pill sits top right |

## 6. The window

```
╭──────────────────────────────────────────────╮
│  Title of the window               –  ▢  ✕ │  ← 36 px strip, the window's own top colour
├──────────────────────────────────────────────┤
│                                              │
│            the app                           │
│                                              │
╰──────────────────────────────────────────────╯
```

- Buttons on the strip: tuck (–), fill (▢), close (✕). Double-click the strip to fill.
  Apps that draw their own title bar can still be moved, resized and snapped the same way.
- Drag the strip to move; drag to a screen edge to snap (a soft preview shows where it lands).
- Super + drag anywhere in the window moves it; Super + right-drag resizes.
- Focus follows the click by default; focus on hover is a setting.

## 7. The overview (Super)

- The desktop dims behind a glass layer. The top part holds the open windows as cards in a grid
  that keeps their proportions; below them, under a small "APPS · n" label, every installed app:
  its own icon from the icon theme (48 px) and its name, in cells of 104 × 98 px, pinned apps
  first (a small accent dot) and the rest by name. Settings › Look › "Apps in the overview" turns
  the grid into a compact list (24 px icons, 40 px rows, as many columns as fit). With no windows
  open, the apps take the whole space and the selection starts there.
- There is no search field. A quiet line at the top says what typing does; once you type, it
  becomes a small filter pill and a results list (Windows, Apps, Settings, Calculator, Run).
  `= 2^10` is a sum (Enter copies it); `> cmd` runs a command in a terminal.
- Tab switches between windows and apps; arrow keys move (Up/Down by a whole row in the apps);
  Enter opens; the wheel scrolls the apps; right-click pins or unpins; Escape or Super closes.
  Delete on a card closes that window.
- The quick panel on the right is compact (284 px): the time, six small tiles (Wi-Fi,
  Bluetooth, Quiet, Night light, night/paper, Screenshot: icon and one word, the state is the
  fill), a row each for Wi-Fi and Bluetooth that opens into its list (networks with signal bars
  and a lock; paired devices), an inline password field for a new secured network, slim volume
  and brightness sliders, the battery, and a row of Lock, Sleep, Restart, Power off, then
  Settings. Its height follows what it holds.

## 8. Motion

| Movement | Gentle (default) | Full | Off |
| --- | --- | --- | --- |
| Window opens (fades in) | 180 ms | 180 ms | none |
| Snap, fill, restore (the window morphs to its new place) | 200 ms | 280 ms | none |
| Tuck (fades away toward the shelf) | 220 ms | 310 ms | none |
| Overview in / out | 220 ms | 310 ms | none |

All with a cubic ease-out (`1 − (1 − t)³`). A full-screen game or video gets the screen directly
(unredirected) while it is the only thing showing, so it pays nothing for the compositor.

## 9. The ink (一期一会)

The wallpaper is a marbling: drops of ink on water, then combed. Reference:
`samurai/src/seal.rs`.

1. 32 bytes from the operating system's random source seed a xoshiro256** generator.
2. Drops of ink and of clear water fall in turn on a few spots (forty rounds), each new drop
   pushing the earlier rings outward with the area-preserving drop formula
   `p' = c + (p − c)·√(1 + r²/|p − c|²)`; the clear drops make the thin rings.
3. The surface is combed: a slow wave, three tine strokes, a swirl and a last cross-wave.
4. The rings are written as SVG paths for both grounds; the seed is overwritten.
5. The mark is the first six bytes of SHA-256 of the night SVG, written as `3F2A · 91C0 · 7B44`.

The login screen never shows your ink: it is on screen before anyone has proved who they are.
It shows a public decoy made from a fixed seed.

## 10. Lock and login

- Lock: covers every screen, with the clock and one password field. It checks with PAM (`/etc/pam.d/samurai-lock`), keeps typing while a check is
  running, and ignores key chords so a shortcut never lands in the field. It also locks before
  sleep (xss-lock) and after the idle time in Settings › Power.
- Login: the same layout on the decoy ink, the user's initial in a circle, the session picker
  under the field, restart and power off in the corner. Any extra question the system asks (a
  one-time code) appears in place of the password field.

## 11. Settings

Ten pages, one list of settings (`tools/schema.py`). Every change is saved and applied as you
make it; there is no Apply button. Each setting has a label in plain words and one line of
help. Pages: Look, Displays, Network, Sound, Keyboard, Mouse, Apps, Power, Notifications, System.
The Network page has the campus Wi-Fi fixes (UC-Secure / AirUC-Secure, eduroam) and a repair
button with a readable log. The System page shows the X server (X.Org or XLibre) and the update
state.

## 12. App defaults

| Kind | App |
| --- | --- |
| Terminal | Alacritty, in the Samurai colours, with a two-line prompt (folder, git branch and changes, exit code, time) |
| Files | Thunar |
| Browser | Firefox |
| Text | Mousepad |
| Images | imv |
| PDF | Zathura |
| Video and music | mpv |
| Mail | Thunderbird |

Settings › Apps changes any of them; the choice is written to `mimeapps.list` through
`xdg-mime`.

## 13. Accessibility

- Dim text and every accent meet WCAG AA contrast (4.5:1 or more) on both grounds; the lowest
  is dim text on paper, 5.4:1.
- Everything in the overview, Settings and the login screen works from the keyboard.
- Motion can be turned off (Settings › Look › Motion: Off).
- Size scales everything from 100 % to 250 %.
