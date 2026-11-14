# Samurai — design document

This document is enough to rebuild the Samurai look anywhere, including as a website, without
the desktop code. The desktop (niri + Quickshell) is one implementation of it; `shell/` is
the reference. Where this document and the code disagree, the code is a bug.

Contents: 1 idea · 2 rules · 3 colour · 4 type · 5 grid and measures · 6 kanji · 7 components ·
8 states · 9 layouts · 10 the one-time drawing · 11 Incognito · 12 motion · 13 accessibility ·
14 website port (CSS and HTML reference) · 15 checklist

---

## 1. The idea

**The frame is the interface.** One thin line runs around the screen. Everything the system
shows grows out of that line and shares it: the bar is the frame's top edge thickened, the rail
is its left edge thickened, a panel is a rectangle whose two sides *are* the frame. Nothing
floats, nothing has a shadow, nothing has its own border separate from the grid.

The style is minimal brutalism (the "raw internet" end of it: visible grid, monospaced type,
plain controls, almost no animation), Japanese in vocabulary, two colours and one red.

One sentence to test any new screen against: *if you erased every fill, would the lines alone
still show the whole layout?* If yes, it is Samurai.

## 2. Rules

These are hard rules. A design that breaks one is a different design.

1. **Two grounds, one red.** The ground is pure black or pure white. Ink is the opposite. There
   is exactly one accent, a Pantone red. No greys except one dim ink for secondary text. No
   gradients, no transparency over content, no blur, no shadow.
2. **Square.** Corner radius is 0 everywhere, including focus rings, images and the red disc's
   container. The only curve allowed is the disc itself.
3. **Lines are 1px and shared.** Two neighbouring cells have one line between them, not two.
   A line belongs to the grid, not to a component.
4. **Selected = inverted.** A selected or pressed thing swaps ink and ground. It does not change
   hue, grow, glow or lift.
5. **Red means one of three things only:** the place you are (current workspace, current day),
   something that needs you (an update, an error, a held keybind), or a destructive action. Red
   is always a *fill* with white text on it. Red is never small text and never a hover colour.
6. **Kanji carries the label, Latin explains it.** Every destination has a one- or two-character
   kanji mark and an uppercase mono Latin label. Either can be hidden; the kanji is never
   decoration without meaning (see section 6).
7. **Controls look like what they are.** A button is a boxed cell. A text field is a box you type
   in. A toggle is two cells, one filled. No custom sliders-as-art, no icon-only mystery buttons.
8. **Nothing moves unless asked.** State changes are cuts, not fades (section 12).
9. **The drawing is made once.** Each installation (or, on the web, each visitor) gets one
   generated line drawing and it cannot be made again (section 10).

## 3. Colour

| Token | Black ground | White ground | Use |
|---|---|---|---|
| `ground` | `#000000` | `#FFFFFF` | every surface |
| `ink` | `#FFFFFF` | `#000000` | text, lines, filled (selected) cells |
| `dim` | `#A3A3A3` | `#5A5A5A` | secondary text only, never lines of the frame |
| `red` | `#C8102E` | `#C8102E` | Pantone 186 C. Does not change with the ground |
| `on-red` | `#FFFFFF` | `#FFFFFF` | text on red, on both grounds |
| `hover` | ink at 14% | ink at 14% | the only tint; under the pointer |

Alternative reds the settings offer (same rules apply): Pantone 485 C `#DA291C`, Pantone 032 C
`#EF3340`.

Contrast (WCAG ratio): ink on ground 21:1 · dim on black 8.3:1 · dim on white 6.9:1 · white on
red 5.9:1 · red on white 5.9:1 · **red on black 3.6:1**, which is why red is never small text
on the black ground: on black, red appears only as a fill behind white text, or as a line or
shape at least 4px thick.

Switching ground swaps `ground`/`ink`/`dim` and nothing else. Layout, line positions, red and
type do not move. A screenshot of the white ground must be the exact negative of the black one
except for the red.

## 4. Type

- **Latin:** IBM Plex Mono. Weights 400 (text) and 700 (labels, selected, headings). No italics.
- **Japanese:** Noto Sans CJK JP (web: Noto Sans JP). Weight 700 for marks, 400 for dates.
- Fallbacks: any monospace; any CJK sans. Never a serif, never a rounded face.

| Role | Size | Weight | Case / tracking |
|---|---|---|---|
| Body, control label | 13px | 400 | sentence case |
| Small / meta | 11px | 400 | sentence case; section labels uppercase, +2px tracking |
| Control label (Latin, in a cell) | 13px | 700 when selected | UPPERCASE |
| Kanji in a cell | 16px | 700 | — |
| Page kanji | 44px | 700 | paired with a 13px 700 uppercase Latin word on its baseline, +2px tracking |
| Panel kanji | 44px (30px in a sidebar) | 700 | same pairing |
| Clock on the lock face | 16% of screen height | 700 | — |
| Lock kanji (red column) | half the column width | 700 | stacked vertically |

Line height 1.35 for running text; single-line labels are vertically centred in their cell.
Numbers are tabular (mono does this for free). Dates may be written in kanji numerals
(`十一月十四日 土`); workspace numbers use the formal numerals 壱 弐 参 肆 伍 陸 漆 捌 玖 拾.

## 5. Grid and measures

All measures are multiples of 4px. At scale 1:

| Measure | Value |
|---|---|
| Line | 1px |
| Hit target (smallest clickable thing) | 44 × 44px |
| Bar height | 44px |
| Rail width | 48px |
| Frame inset (gap between the screen edge and the frame's right and bottom lines) | 12px |
| Cell padding, horizontal | 12px |
| Content gutter (text from a panel's edge) | 20px |
| Row height in lists and settings | 52px (44 + 8) |
| Selected-row marker / notice stripe | 8px red bar, full height, left edge |
| "Changed from default" marker | 4px red bar, full height, left edge |
| Focus ring | 1px line inset 3px inside the cell, in the cell's text colour |

The frame on a screen of width W and height H:

```
(0,0)┌──────┬───────────────────────────── bar, 44 high, full width ───────────────┐
     │ 影   │ 壱 弐 参 肆 │ 端 KITTY  title…          更新 1 │通信│音 45│電 82│ date time │
     ├──────┼────────────────────────────────────────────────────────────────────┐ │
     │ rail │                                                                    │ │ ← right line
     │ 48   │                 the desktop / the page content                     │ │   at W − 12
     │ wide │                                                                    │ │
     │ 設   │                                                           mark box │ │
     │ 電   │                                                                    │ │
     └──────┴────────────────────────────────────────────────────────────────────┘ │
                                              ↑ bottom line at H − 12
```

- The bar's bottom line and the rail's right line are the frame's top and left sides.
- The right and bottom sides are 1px lines 12px in from the screen edge.
- A panel opens *in a corner of the frame* and uses the frame's two lines as two of its sides,
  so it only ever draws the other two. Start opens from the top-left corner, Quick and Widgets
  from the top-right, Power from the bottom-left.
- Windows (or page content) sit inside the frame with a gap (default 16px to the frame, 8px
  between each other) and a 1px ink line, no rounding.

## 6. Kanji vocabulary

One mark per meaning. Do not invent synonyms; reuse these.

| Mark | Meaning | Mark | Meaning |
|---|---|---|---|
| 影 | the system / home (the corner mark) | 設 設定 | settings |
| 起動 | start | 即 | quick settings |
| 電 電源 | power / battery | 施錠 | lock |
| 休止 | sleep | 退出 | log out |
| 再起 再起動 | restart | 閉 | close |
| 探 | search | 加 | add |
| 行 | run | 入 / 切 | on / off |
| 更新 | update | 通信 | Wi-Fi |
| 無線 | Bluetooth | 音 | sound |
| 輝 | brightness | 夜 | night light |
| 静 | do not disturb | 地 | ground (black/white) |
| 隠 | Incognito | 撮 / 録 | screenshot / record |
| 鍵 | keys | 修 / 診 | repair / diagnose |
| 一期一会 | "once, one meeting": the one-time drawing | 入城 | "enter the castle": log in |

Apps are a single kanji by kind: 端 terminal · 蔵 files · 網 browser · 書 editor · 話 chat ·
信 mail · 音 music · 遊 games · 画 images · 読 reader · 線 network · 務 tasks · 算 calculator ·
具 anything else.

Settings pages: 画面 displays · 入力 input · 鍵 keybinds · 接続 connect · 描画 GPU · 録画 record ·
音声 dictate · 言語 region · 外観 look · 枠 frame · 起動 start · 小物 widgets · 施錠 lock · 動 motion ·
店 store · 追加 plugins · 窓則 window rules · 層則 layer rules · 自動 autostart · 環境 env ·
個別 per-app · 修復 fix · 更新 updates · 隠 incognito.

Label language is a setting: both (default), Japanese only, English only. A website should keep
"both" and put the Latin label in the accessible name.

## 7. Components

There is **one** control, the cell. Everything else is cells arranged on lines.

### 7.1 Cell
A rectangle on the grid. Minimum 44px high. Contents: optional kanji (16px, 700) then optional
Latin label (13px, uppercase), 8px apart, centred. It can carry a line on any side (`left`,
`right`, `top`, `bottom`) or all four (`box`).

| Variant | Ground | Text |
|---|---|---|
| plain | transparent | ink |
| hover | ink at 14% | ink |
| selected / pressed / "on" | ink | ground, 700 |
| accent | red | white, 700 |
| disabled | as plain, whole cell at 40% opacity | — |
| focused | as its variant, plus the inner 1px ring | — |

### 7.2 Field
A 44px-high box with a 1px ink line and the ground as its fill (so it stays readable on top of
the drawing). Focus thickens the line to 2px; nothing else changes. Placeholder is dim.
Selection is red with white text. Passwords use the platform's standard masking.

### 7.3 Toggle
Two boxed cells side by side: `切 OFF` and `入 ON`. The live one is filled. Never a sliding pill.

### 7.4 Stepper
`−` cell · field (88px wide) · `+` cell, sharing lines. Typing a number and pressing Enter
works; out-of-range values are clamped, not refused.

### 7.5 Choice
Up to four options: one boxed cell each, in a row, the chosen one filled. More than four:
`‹` · the current value as a filled 200px cell · `›`.

### 7.6 Meter (volume, brightness)
A 16px-high box with a 1px line; the filled part is solid ink from the left. Label to its left
(`音 VOL`), the number to its right. No knob.

### 7.7 Row (a setting, a list entry)
52px high. Label 20px from the left, control right-aligned 20px from the right, one dim line
at 50% under it. A setting changed from its default gets the 4px red bar at the left and a `↺`
boxed cell before its control.

### 7.8 List editor
A bold label, a dim line of column names joined with ` · `, one row per entry with a `×` boxed
cell at the right, then an add line: a field whose placeholder is an example entry in the same
` · ` shape, and a boxed `加 ADD` cell.

### 7.9 Slip (notice, message)
A box with a 1px line, ground fill, an 8px red bar down the left edge, a bold one-line title
and up to three lines of body. Click dismisses. An error message under a field is the inverse:
a red fill with white bold text, 44px high.

### 7.10 Card
A boxed area with the 8px red bar at the left, a 26px kanji and bold Latin title on one line,
dim explanatory text, then its fields and one filled action cell. Used for the two campus
Wi-Fi setups (UC-Secure + AirUC-Secure, eduroam).

### 7.11 Panel
A ground-filled rectangle that draws only the sides the frame does not already give it. Head:
44px kanji + uppercase Latin on its baseline, 20px from the left, 16px from the top. Sections
are separated by full-width 1px ink lines; section labels are 11px dim uppercase. A panel may
end in a foot row of cells divided by lines.

### 7.12 Bar
44px. Left to right: the 影 corner cell (rail-width, opens Start) · workspace cells (one formal
numeral each; current = red fill, occupied = ink, empty = dim) · the focused app's kanji, its
name in bold uppercase and the window title in dim · (space) · status cells, each with a line
on its left: update (red, only when one is ready), network, sound, battery, date and time.

### 7.13 Rail
48px. One 48px cell per app, a single kanji. Focused = filled. Running elsewhere = a 2px red
tick at the left edge. **Minimized windows live here**: their cell is dim and carries a small
hollow square at its top-left; clicking it brings the window back. Foot: 設 (settings) and 電
(power).

### 7.14 The mark
Bottom-right inside the frame: a boxed two-line label (`ONE TIME · ONE MACHINE` / the 16-hex
mark, e.g. `15A2·2B54·1239·066B`, both 11px, +2px tracking) next to a 64px red square holding
影 in white. On the web this is the signature block of the page.

## 8. States

| State | How it shows |
|---|---|
| Hover | 14% ink tint. No colour change, no underline, no movement |
| Pressed / selected / on | inverted |
| Focus (keyboard) | inner 1px ring; fields thicken to 2px |
| Disabled | 40% opacity, no hover |
| Busy | the label changes to a word (`checking…`); no spinner |
| Needs attention | red fill + white text, or the 8px red bar |
| Error | red fill + white bold text directly under what failed, starting with `✕` |
| Warning | `▲` and bold ink text; red only if something was actually held back |
| OK / free | `○` and dim text |
| Empty | one dim sentence saying what is missing and the one action that fills it |

## 9. Layouts

**Desktop.** Section 5. The drawing fills the ground behind everything; windows cover it.

**Settings hub.** A window, 1080 × 720 by default. Left sidebar 232px with a right line: head
(設定 SETTINGS), a search field, then groups (系 SYSTEM · 机 DESKTOP · 追 ADD-ONS · 深 ADVANCED ·
他 OTHER), each group opened by a full-width line and an 11px dim label, each page a 36px cell
with its kanji at 16px from the left and its Latin name at 64px; the current page is filled; a
count of changed settings sits at the right. Right side: page head (44px kanji + Latin), a dim
one-sentence blurb, a full-width line, any hand-made block for that page, then the rows. Foot:
a 45px bar with a line above: "N changed on this page · saved as you go · applied live" at the
left, `RESET THIS PAGE` and `閉 CLOSE` cells at the right.

**Lock and login.** The frame *closes*: all four lines drawn, 12px in. A red column 16% of the
screen wide fills the left side inside the frame, with two stacked white kanji (施 錠 to lock,
入 城 to log in) and a tracked uppercase word at its foot (`LOCKED` / `ENTER`). To its right,
48px in: the clock and the date at the top; at the bottom, the user's name, one password field
(460px with a filled 開 cell), and the message row. Bottom-right corner: sleep, restart and
power cells with lines on their left and top. Bottom-left: `一期一会` and the mark, dim. The login
screen shows the public drawing (section 11), never a user's own, because it is on screen
before anyone has proved who they are.

**Installer.** One line, nothing else:
`侍 ████████████░░░░░░░░ 62%  forging this machine's seal`. Detail goes to a log file. Failure
replaces the bar with `✕ stopped at: <step>` and the log's path.

## 10. The one-time drawing (一期一会)

Each installation has one wallpaper pair (black and white ground) that cannot be made again.
It is not chosen from a set and not derived from anything about the machine. It is drawn from
32 bytes of operating-system randomness, written out as two SVG files, and the 32 bytes are
then overwritten in memory and never stored. With 2^256 possible seeds and no copy of the seed,
neither a person nor the software can redraw it. Asking for a new one destroys the current one.

### Algorithm (reference: `samurai/src/seal.rs`)

Canvas 1440 × 900, `preserveAspectRatio="xMidYMid slice"`. PRNG: xoshiro256\*\* seeded with the
32 bytes as four little-endian 64-bit words; `f()` = top 53 bits ÷ 2^53, so 0 ≤ f < 1. Draw
values in exactly this order:

1. Centre: `cx = W·(0.35 + 0.4f)`, `cy = H·(0.3 + 0.4f)`.
2. Frequencies: `a = 1 + ⌊5f⌋`, `b = 1 + ⌊5f⌋`, `c = 1 + ⌊7f⌋`, `d = 1 + ⌊7f⌋`.
3. Phases: `p1 = 2πf`, `p2 = 2πf`.
4. Radii: `r1 = 260 + 320f`, `r2 = 200 + 380f`.
5. **460 chords.** For `i = 0…459`, `u = 2π·i/460`, and a fresh jitter `j = 9·(f − 0.5)` for each
   of the four coordinates (taken in the order x1, y1, x2, y2):
   - `x1 = cx + r1·cos(a·u + p1) + j`, `y1 = cy + 0.62·r1·sin(b·u) + j`
   - `x2 = cx + 1.3·r2·cos(c·u + p2) + j`, `y2 = cy + r2·sin(d·u + p1) + j`
   - a straight line from (x1, y1) to (x2, y2).
6. **3 red cuts.** Each a straight line from `(0, H·f)` to `(W, H·f)`.
7. **The disc.** Centre `(round(W·(0.55 + 0.35f)), round(H·(0.2 + 0.5f)))`, radius
   `round(70 + 110f)`.
8. **4 frame cuts.** Four more values of `f`: the position, as a fraction along each side of the
   frame (top, right, bottom, left), of a short red mark on the frame itself.
9. Zero the PRNG state and the seed.

Painting order, bottom to top: ground rectangle · red disc (solid) · the 460 chords as one path
(ink, stroke 0.55, opacity 0.5, no fill) · the 3 cuts as one path (red, stroke 1.5). The white
file is the same geometry with ground and ink swapped. The chords lie *over* the disc, so the
disc reads as something behind a screen of lines.

The **mark** is the first 8 bytes of SHA-256 of the black SVG's text, as 16 uppercase hex
digits in groups of four joined by `·`. It identifies the drawing without being able to
reproduce it.

### On a website

Decide what "once" means and say it on the page:

- **Once per site (recommended for a brand site):** run the generator one time at build time,
  commit the two SVGs and the mark, delete the seed. Every visitor sees *the* Samurai drawing.
- **Once per visitor:** on first visit, `crypto.getRandomValues(new Uint8Array(32))`, draw,
  store the SVG text (not the seed) in `localStorage`, overwrite the array with zeros. Offer no
  "regenerate" button; clearing site data is the only way to lose it, and then it is gone.

Never use `Math.random()`, never derive the seed from the user agent, time or IP, and never
keep the seed. A JavaScript port needs 64-bit integer arithmetic (`BigInt`) for xoshiro256\*\*.
A different generator is acceptable on the web as long as it keeps: thin ink chords from two
closed parametric curves, one solid red disc under them, three full-width red cuts, and the
seed-destroyed rule.

## 11. Incognito

Incognito hides what identifies the person and the machine and, on purpose, breaks rule 3: it is
the "chaotic" counterpart of the same parts, so it is obvious at a glance that the mode is on.

- **Hidden:** user and host names (`someone @ somewhere`), window titles (`window`), the network
  name (`????`), notice text (`a notice`), the mark (`████·████·████·████`), and the drawing,
  which is replaced by the **decoy**: the same generator run with the fixed public seed
  `samurai-incognito-decoy-00000000`, identical everywhere, so it identifies nobody.
- **Broken grid:** each surface gets a fixed tilt of up to ±6° from a per-slot hash
  (`frac(sin(slot·12.9898)·43758.5453)` mapped to −1…1, times the strength). The tilt is the
  same every time, so nothing jumps; panels use half the tilt so they stay readable. The
  drawing is rotated 7° and scaled 1.25.
- **Signals:** a red strip across the top repeating `隠 INCOGNITO`; the corner square shows 隠
  instead of 影; a boxed notice at the bottom-left says what is hidden with a `LEAVE` cell.
- **Unchanged:** every control still works, sits under the pointer where it is drawn, and keeps
  its 44px target. Colours, type and sizes do not change. The tilt can be turned off while
  keeping the hiding.

On a website, Incognito is a "private view" toggle: same rules, applied to anything that names
the visitor.

## 12. Motion

Default: none. Opening, closing, switching ground, selecting: all are instant cuts. This is a
feature of the style, not a missing one. If motion is turned on, it is limited to: a panel
growing from its corner of the frame along one axis in at most 120ms, linear; nothing fades,
nothing bounces, nothing moves on hover. Respect `prefers-reduced-motion` by staying at "none".

## 13. Accessibility

- Every target is at least 44 × 44px. Rows are 52px.
- Colour is never the only signal: selection is inversion plus bold; attention is red plus a
  word or a `▲`/`✕`/`○` glyph.
- Text contrast is at least 5.9:1 everywhere (section 3). Red text on black is not allowed.
- Keyboard: everything reachable with Tab, activated with Enter or Space, panels closed with
  Escape; focus is always visible (inner ring or 2px field line).
- Kanji marks are labels, so on the web each needs an accessible name in the page's language
  (`aria-label` or visible Latin text); decorative repeats get `aria-hidden="true"`.
- Use real controls: `<button>`, `<input>`, `<input type="range">` (styled as the meter),
  `<fieldset>` with radio inputs for toggles and choices. No `div` with a `role`.
- Both grounds must pass the same checks; test the white ground as carefully as the black.

## 14. Website port

### Tokens

```css
:root {
  --ground: #000; --ink: #fff; --dim: #a3a3a3;
  --red: #c8102e; --on-red: #fff;
  --line: 1px; --hit: 44px; --bar: 44px; --rail: 48px; --inset: 12px;
  --pad: 12px; --gutter: 20px; --row: 52px;
  --mono: "IBM Plex Mono", ui-monospace, monospace;
  --jp: "Noto Sans JP", "Noto Sans CJK JP", sans-serif;
  --px: 13px; --small: 11px;
  color-scheme: dark;
}
:root[data-ground="white"] { --ground: #fff; --ink: #000; --dim: #5a5a5a; color-scheme: light; }
@media (prefers-color-scheme: light) {
  :root:not([data-ground]) { --ground: #fff; --ink: #000; --dim: #5a5a5a; color-scheme: light; }
}
* { box-sizing: border-box; border-radius: 0; }
html { background: var(--ground); color: var(--ink); font: 400 var(--px)/1.35 var(--mono); }
body { margin: 0; min-height: 100vh; }
:lang(ja), .jp { font-family: var(--jp); font-weight: 700; }
::selection { background: var(--red); color: var(--on-red); }
```

### The frame as a page

```css
.frame { display: grid; min-height: 100vh;
  grid-template: "corner bar" var(--bar) "rail main" 1fr / var(--rail) 1fr; }
.bar  { grid-area: bar;  display: flex; border-bottom: var(--line) solid var(--ink); }
.rail { grid-area: rail; display: flex; flex-direction: column; border-right: var(--line) solid var(--ink); }
.corner { grid-area: corner; border-right: var(--line) solid var(--ink); border-bottom: var(--line) solid var(--ink); }
.main { grid-area: main; position: relative; padding: 16px calc(16px + var(--inset)) calc(16px + var(--inset)) 16px; }
/* the right and bottom sides of the frame */
.main::after { content: ""; position: absolute; inset: 0 var(--inset) var(--inset) 0; pointer-events: none;
  border-right: var(--line) solid var(--ink); border-bottom: var(--line) solid var(--ink); }
/* the drawing sits behind the content, inside the frame */
.main > .seal { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; z-index: -1; }
```

On narrow screens (under 640px) the rail becomes a bottom row of the same cells (48px high,
line on top) and the bar keeps only the corner mark, the page name and one status cell. The
inset stays 12px. Nothing collapses into a hamburger icon; if items do not fit, the row scrolls
sideways.

### Cell, field, toggle, row, slip

```css
.cell { display: inline-flex; align-items: center; justify-content: center; gap: 8px;
  min-height: var(--hit); min-width: var(--hit); padding: 0 var(--pad);
  background: none; color: var(--ink); border: 0; font: inherit; text-transform: uppercase; cursor: pointer; }
.cell .jp { font-size: 16px; text-transform: none; }
.cell:hover { background: color-mix(in srgb, var(--ink) 14%, transparent); }
.cell[aria-pressed="true"], .cell[aria-current], .cell.on { background: var(--ink); color: var(--ground); font-weight: 700; }
.cell.accent { background: var(--red); color: var(--on-red); font-weight: 700; }
.cell:disabled { opacity: .4; pointer-events: none; }
.cell:focus-visible { outline: var(--line) solid currentColor; outline-offset: -4px; }
.cell.box { border: var(--line) solid var(--ink); }
.cell.box + .cell.box { border-left: 0; }           /* neighbours share a line */
.l { border-left: var(--line) solid var(--ink); }  .t { border-top: var(--line) solid var(--ink); }

.field { height: var(--hit); padding: 0 10px; background: var(--ground); color: var(--ink);
  border: var(--line) solid var(--ink); font: inherit; outline: 0; }
.field:focus { border-width: 2px; padding: 0 9px; }
.field::placeholder { color: var(--dim); }

.toggle { display: inline-flex; border: 0; padding: 0; margin: 0; }
.toggle input { position: absolute; opacity: 0; }
/* each <label> inside .toggle also carries class="cell box" */
.toggle input:checked + label { background: var(--ink); color: var(--ground); font-weight: 700; }
.toggle input:focus-visible + label { outline: var(--line) solid currentColor; outline-offset: -4px; }

.row { display: flex; align-items: center; justify-content: space-between; min-height: var(--row);
  padding: 0 var(--gutter); border-bottom: var(--line) solid color-mix(in srgb, var(--dim) 50%, transparent); }
.row.changed { box-shadow: inset 4px 0 0 var(--red); }

.slip { position: relative; padding: 10px 12px 10px 20px; background: var(--ground); border: var(--line) solid var(--ink); }
.slip::before { content: ""; position: absolute; inset: 0 auto 0 0; width: 8px; background: var(--red); }
.error { min-height: var(--hit); padding: 0 10px; display: flex; align-items: center;
  background: var(--red); color: var(--on-red); font-weight: 700; }

.meter { appearance: none; height: 16px; background: none; border: var(--line) solid var(--ink); }
.meter::-webkit-slider-thumb { appearance: none; width: 1px; height: 14px; background: var(--ink);
  box-shadow: -100vw 0 0 100vw var(--ink); }          /* fills to the left; clip with overflow: hidden */
.meter { overflow: hidden; }
.meter::-moz-range-progress { height: 14px; background: var(--ink); }
```

(`box-shadow: inset` on `.row.changed` is a flat 4px bar, not a shadow in the visual sense; use
a pseudo-element if a linter forbids it.)

### Markup

```html
<div class="frame">
  <a class="corner cell" href="/" aria-label="Samurai home"><span class="jp" aria-hidden="true">影</span></a>
  <header class="bar">
    <nav aria-label="Sections">
      <a class="cell" href="/" aria-current="page"><span class="jp" aria-hidden="true">壱</span> Home</a>
      <a class="cell" href="/docs"><span class="jp" aria-hidden="true">弐</span> Docs</a>
    </nav>
    <span style="flex:1"></span>
    <button class="cell l" id="ground" aria-label="Switch black and white"><span class="jp" aria-hidden="true">地</span> Base</button>
  </header>
  <nav class="rail" aria-label="Pages">
    <a class="cell" href="/install" aria-label="Install"><span class="jp" aria-hidden="true">入</span></a>
    <a class="cell" href="/settings" aria-label="Settings"><span class="jp" aria-hidden="true">設</span></a>
  </nav>
  <main class="main">
    <img class="seal" src="/seal-black.svg" alt="">
    <h1><span class="jp">外観</span> <small>LOOK</small></h1>
    …
    <footer class="mark">ONE TIME · ONE SITE<br>15A2·2B54·1239·066B</footer>
  </main>
</div>
<script>
  // ground: follows the system until the visitor chooses; the choice is remembered
  const root = document.documentElement, key = "samurai-ground";
  try { const g = localStorage.getItem(key); if (g) root.dataset.ground = g; } catch (e) {}
  document.getElementById("ground").addEventListener("click", () => {
    const now = root.dataset.ground || (matchMedia("(prefers-color-scheme: light)").matches ? "white" : "black");
    root.dataset.ground = now === "white" ? "black" : "white";
    try { localStorage.setItem(key, root.dataset.ground); } catch (e) {}
    document.querySelector(".seal").src = "/seal-" + root.dataset.ground + ".svg";
  });
</script>
```

Headings on the web: `h1` is the page kanji (44px) with the Latin word as `<small>` (13px, 700,
uppercase, +2px tracking) on its baseline. `h2` is a section label: 11px, dim, uppercase,
+2px tracking, with a full-width 1px ink line under it. There is no `h3` style; if a page needs
one, it needs another page.

Body text on the web may be wider than a control label but never wider than 72 characters.
Links in running text are ink with a 1px ink underline; hover inverts them. Links are never red.

## 15. Checklist

Before calling a screen or page "Samurai":

- [ ] Only `#000`/`#fff`, one dim, one red. No other colour appears, including in images' frames.
- [ ] No radius, shadow, gradient, blur or transparency over content.
- [ ] Every line is 1px and shared; erase the fills and the layout is still readable.
- [ ] Selection is inversion. Red appears only for "here", "needs you", "destroys".
- [ ] No red text on black. Text on red is white.
- [ ] Every target ≥ 44px; focus is visible; Escape closes what Enter opened.
- [ ] Kanji used are from section 6 and have accessible names.
- [ ] It is correct on both grounds, and the white one is the negative of the black one.
- [ ] Nothing animates by default.
- [ ] The drawing came from destroyed randomness, and there is no button that makes another
      without destroying the current one.
