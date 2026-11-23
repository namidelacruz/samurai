#!/usr/bin/env python3
"""Source of the settings list. Run: python3 tools/schema.py  ->  settings/schema.json

One list drives three things: the pages of the Settings app, the checks in the `samurai` command
(a value of the wrong kind or out of range is refused), and the config files `samurai apply` writes.
Every setting has a default and one plain sentence saying what it does.
"""
import json, os, sys

def T(key, label, default, help="", **kw): return dict(key=key, label=label, type="toggle", default=default, help=help, **kw)
def I(key, label, default, lo, hi, step=1, unit="", help="", **kw): return dict(key=key, label=label, type="int", default=default, min=lo, max=hi, step=step, unit=unit, help=help, **kw)
def F(key, label, default, lo, hi, step=0.05, unit="", help="", **kw): return dict(key=key, label=label, type="float", default=default, min=lo, max=hi, step=step, unit=unit, help=help, **kw)
def E(key, label, default, options, help="", **kw): return dict(key=key, label=label, type="enum", default=default, options=options, help=help, **kw)
def S(key, label, default="", help="", **kw): return dict(key=key, label=label, type="text", default=default, help=help, **kw)
def L(key, label, columns, example, help="", **kw): return dict(key=key, label=label, type="list", default=[], columns=columns, example=example, help=help, **kw)
def sec(title, *items): return dict(title=title, items=list(items))
def page(pid, title, icon, lead, *sections, custom=None): return dict(id=pid, title=title, icon=icon, lead=lead, custom=custom, sections=list(sections))

def opts(*pairs):
    """[(value, label)] for enums whose stored value differs from what is shown."""
    return [dict(value=v, label=l) for v, l in pairs]

ACCENTS = opts(("indigo", "Indigo"), ("moss", "Moss"), ("rust", "Rust"), ("plum", "Plum"), ("slate", "Slate"))
ACTIONS = ["overview", "terminal", "files", "browser", "settings", "close", "fill", "fullscreen", "restore", "snap left", "snap right",
           "snap top-left", "snap top-right", "snap bottom-left", "snap bottom-right", "center", "tuck", "untuck", "focus next",
           "focus prev", "lock", "ground", "screenshot area", "screenshot screen", "screenshot window", "volume up", "volume down",
           "volume mute", "brightness up", "brightness down", "spawn", "none"]

PAGES = [
  page("look", "Look", "ink", "Night or paper, one accent, your ink. Everything here changes at once.",
    sec("Ground",
      E("look.ground", "Ground", "sun", opts(("night", "Night"), ("paper", "Paper"), ("sun", "Follow the sun")),
        help="Follow the sun: paper during the day, night after dark (the times below)."),
      S("look.sun_from", "Paper from", "07:30", help="When the paper ground starts, as HH:MM.", pattern=r"^([01]?\d|2[0-3]):[0-5]\d$"),
      S("look.sun_to", "Night from", "18:00", help="When the night ground starts, as HH:MM.", pattern=r"^([01]?\d|2[0-3]):[0-5]\d$"),
      E("look.accent", "Accent", "indigo", ACCENTS, help="Used for focus, selection and the ink in the wallpaper.")),
    sec("Size and motion",
      F("look.scale", "Text and interface size", 1.0, 1.0, 2.5, 0.25, unit="×", help="One size for every app, old and new. 1.5 suits a 14\" 2.5K screen."),
      E("look.motion", "Motion", "gentle", opts(("full", "Full"), ("gentle", "Gentle"), ("off", "Off")), help="Gentle: quick and short. Off: everything just appears."),
      E("look.corners", "Window corners", "rounded", opts(("rounded", "Rounded"), ("square", "Square")))),
    sec("Overview (Super)",
      E("look.apps_view", "Apps in the overview", "icons", opts(("icons", "Icons"), ("list", "List")), help="Every installed app, under your open windows. Type to filter."),),
    sec("On the desktop",
      T("look.show_clock", "Time in the corner", True, help="A small pill, top right. It hides while a window covers it."),
      T("look.show_mark", "This machine's mark", True, help="The mark of your one-time ink, bottom left.")),
    sec("Terminal",
      T("term.follow", "Terminal colours follow the ground", True),
      F("term.size", "Terminal text size", 11.0, 7.0, 24.0, 0.5, unit="pt"),
      T("term.fetch", "System summary when a terminal opens", True, help="Samurai's ink helmet and a few lines: system, window manager, X server, memory, Wi-Fi. Once per terminal window."),
      T("term.prompt", "Samurai prompt in bash and zsh", True, help="Folder, git branch and changes, time taken, and the error code only when a command failed."),
      I("term.padding", "Space around the text", 18, 0, 48, 2, unit="px")),
    sec("Fonts",
      S("look.ui_font", "Interface font", "Inter"),
      S("look.mono_font", "Monospace font", "JetBrains Mono")),
    custom="look"),

  page("displays", "Displays", "display", "Screens, their sizes and where they sit. Night light.",
    sec("Screens",
      L("displays.monitors", "Fixed settings per screen", ["output", "mode", "position", "rotate", "primary"], "HDMI-1 · 1920x1080@60 · right-of eDP-1 · normal · no",
        help="Screens without a line use their best mode, placed left to right."),
      T("displays.mirror_new", "Mirror a new screen until it is arranged", False)),
    sec("Night light",
      T("night.enabled", "Night light", False, help="Warmer colours in the evening."),
      I("night.temp", "Evening warmth", 4000, 2500, 6000, 100, unit="K"),
      S("night.from", "From", "20:00", pattern=r"^([01]?\d|2[0-3]):[0-5]\d$"),
      S("night.to", "Until", "07:00", pattern=r"^([01]?\d|2[0-3]):[0-5]\d$")),
    custom="displays"),

  page("network", "Network", "wifi", "Wi-Fi, campus networks, wired and VPN. Changes apply at once.",
    sec("Wi-Fi",
      T("net.repair_after_update", "Repair campus Wi-Fi after system updates", True, help="Checks UC-Secure and eduroam once after each update and fixes the certificate if needed."),
      T("net.powersave", "Wi-Fi power saving", True, help="Off can help a connection that keeps dropping.")),
    custom="network"),

  page("sound", "Sound", "sound", "Where sound goes, where it comes from, and how loud.",
    sec("Volume",
      E("sound.max", "Loudest volume", "100", opts(("100", "100%"), ("150", "150% (may distort)"))),
      I("sound.step", "Volume keys change it by", 5, 1, 20, 1, unit="%")),
    custom="sound"),

  page("keyboard", "Keyboard & shortcuts", "keyboard", "Layout, repeat, and every shortcut. Tap Super alone for the overview.",
    sec("Typing",
      S("kbd.layout", "Layout", "us", help="One or more, comma separated: us,ca"),
      S("kbd.variant", "Variant", "", help="Empty for the usual one; e.g. colemak, dvorak, intl."),
      S("kbd.options", "Extra options", "", help="xkb options, e.g. caps:escape or grp:alt_shift_toggle"),
      I("kbd.delay", "Repeat starts after", 300, 150, 1000, 25, unit="ms"),
      I("kbd.rate", "Repeats per second", 30, 5, 80, 1)),
    sec("Shortcuts",
      E("keys.preset", "Shortcut set", "samurai", opts(("samurai", "Samurai"), ("hyprland", "Hyprland-style")),
        help="Hyprland-style: Super+Q terminal, Super+C close, Super+V float, Super+R apps, Super+arrows move focus. Your own lines below still win."),
      L("keys.custom", "Your shortcuts", ["keys", "action"], "Super+T · spawn alacritty -e htop",
        help="A line here replaces the built-in shortcut on the same keys. Action \"none\" turns a built-in one off.")),
    custom="keyboard"),

  page("mouse", "Mouse & touchpad", "mouse", "Speed, scrolling, tapping, and how windows take focus.",
    sec("Pointer",
      F("mouse.speed", "Speed", 0.0, -1.0, 1.0, 0.1, help="0 is the device's own speed."),
      T("mouse.natural", "Natural scrolling", False, help="Content moves with your fingers."),
      T("mouse.left_handed", "Left-handed buttons", False),
      E("mouse.cursor", "Cursor size", "24", opts(("24", "Normal"), ("32", "Large"), ("48", "Larger")))),
    sec("Touchpad",
      T("touch.tap", "Tap to click", True),
      T("touch.dwt", "Ignore the touchpad while typing", True),
      T("touch.natural", "Natural scrolling on the touchpad", True)),
    sec("Windows",
      E("windows.focus", "A window takes focus", "click", opts(("click", "when clicked"), ("hover", "when the pointer is over it"))),
      T("windows.snap", "Drag to an edge to snap", True, help="Sides make halves, corners make quarters, the top fills the screen."),
      T("windows.snap_assist", "Snap assist", True, help="After you snap a window to one side, the other windows are offered for the rest of the screen."),
      T("windows.tiling", "Tile new windows", True, help="New windows open side by side in columns, like Hyprland or niri. Super+V lets one window float. Off: every window floats."),
      T("windows.shake", "Shake a window to tuck the others away", True),
      I("windows.gap", "Gap around snapped windows", 8, 0, 40, 2, unit="px")),
    sec("Minimized windows (Super+T)",
      T("taskbar.on", "Taskbar for minimized windows", True, help="Hidden until you call it up: Super+T, or the button in the overview. Minimized windows wait there as icons."),
      I("taskbar.columns", "Columns", 6, 1, 12),
      I("taskbar.rows", "Rows", 2, 1, 6),
      I("taskbar.icon", "Icon size", 48, 24, 96, 4, unit="px"),
      E("taskbar.position", "Where it appears", "bottom", opts(("bottom", "Bottom"), ("top", "Top"), ("center", "Middle"), ("bottom-left", "Bottom left"), ("bottom-right", "Bottom right"), ("top-left", "Top left"), ("top-right", "Top right"))),
      I("taskbar.spacing", "Space between icons", 10, 0, 40, 2, unit="px"),
      T("taskbar.labels", "Names under the icons", True),
      E("taskbar.sort", "Order", "recent", opts(("recent", "Most recent first"), ("name", "By name"), ("app", "By app"))),
      T("taskbar.badge", "Count on the corner clock", True, help="A small number while windows are minimized. Nothing else shows until you call the taskbar up.")),
    sec("Transparency",
      F("windows.opacity_active", "Window you are in", 1.0, 0.5, 1.0, 0.05, help="1 is solid. Lower lets the ink show through."),
      F("windows.opacity_inactive", "Other windows", 0.94, 0.5, 1.0, 0.02, help="The windows you are not typing in. 1 is solid."),
      L("windows.opaque_apps", "Always solid", ["app"], "mpv", help="Apps never made see-through (video, images, games). By their window class, e.g. mpv, imv, firefox.")),
  ),

  page("apps", "Apps", "apps", "Which app opens what, and making older apps look and behave like the rest.",
    sec("Opens with",
      S("apps.browser", "Web pages", "firefox.desktop"),
      S("apps.files", "Folders", "thunar.desktop"),
      S("apps.editor", "Text files", "org.xfce.mousepad.desktop"),
      S("apps.images", "Images", "imv.desktop"),
      S("apps.pdf", "PDFs", "org.pwmt.zathura.desktop"),
      S("apps.video", "Video", "mpv.desktop"),
      S("apps.music", "Music", "mpv.desktop"),
      S("apps.mail", "Email links", "org.mozilla.Thunderbird.desktop")),
    sec("Make every app match",
      T("apps.gtk", "GTK apps follow night and paper", True, help="Firefox, Thunar, GIMP and most others."),
      T("apps.qt", "Qt apps follow night and paper", True, help="KDE apps, VLC, OBS."),
      T("apps.fonts", "One font for every app", True),
      T("apps.cursor", "One cursor for every app", True)),
    sec("When apps misbehave",
      T("windows.strip_follows_app", "Title strip takes the app's own colour", True, help="A terminal's strip is the terminal's background."),
      T("windows.unredirect", "No tearing in full-screen video and games", True, help="A full-screen app gets the screen directly."),
      T("windows.remember", "Remember where windows were", True)),
    sec("In the overview",
      L("apps.pinned", "Pinned apps", ["app"], "firefox", help="Shown first among the apps in the overview. Right-click an app there to pin or unpin it.")),
    custom="apps"),

  page("power", "Power", "plug", "Locking, the screen going dark, and sleep.",
    sec("When idle",
      I("idle.lock", "Lock after", 10, 0, 120, 1, unit="min", help="0 never locks by itself."),
      I("idle.screen_off", "Screen off after", 15, 0, 120, 1, unit="min"),
      I("idle.suspend", "Sleep after", 0, 0, 240, 5, unit="min", help="0 never sleeps by itself.")),
    sec("Lock",
      T("lock.on_sleep", "Lock before the machine sleeps", True, help="Also when the lid closes."),
      T("lock.hold_notices", "Hold notifications while locked", True)),
  ),

  page("notices", "Notifications", "bell", "How notifications look, where they appear, and when they wait.",
    sec("Notifications",
      E("notices.corner", "Appear", "top-right", opts(("top-right", "Top right"), ("top-center", "Top middle"), ("bottom-right", "Bottom right"))),
      I("notices.seconds", "Stay for", 6, 2, 30, 1, unit="s", help="Urgent ones stay until you dismiss them."),
      T("notices.history", "Keep a history", True)),
  ),

  page("system", "System & updates", "info", "Updates, the login screen, the X server, and what starts with you.",
    sec("Updates",
      E("updates.check", "Look for updates", "daily", opts(("off", "Never"), ("daily", "Every day"), ("weekly", "Every week"))),
      S("updates.repo", "Get Samurai from", "https://github.com/namidelacruz/samurai.git")),
    sec("Starts with you",
      L("autostart", "Also start", ["command"], "nextcloud --background")),
    custom="system"),
]

SOON = set()

def main():
    out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "settings", "schema.json")
    keys = [i["key"] for p in PAGES for s in p["sections"] for i in s["items"] if i["key"]]
    dup = {k for k in keys if keys.count(k) > 1}
    if dup:
        sys.exit("duplicate keys: %s" % sorted(dup))
    for p in PAGES:
        for s in p["sections"]:
            for i in s["items"]:
                if i["key"] in SOON:
                    i["soon"] = True
                if i["type"] == "enum":
                    i["options"] = [o if isinstance(o, dict) else dict(value=o, label=o) for o in i["options"]]
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "w", encoding="utf-8") as f:
        json.dump(dict(version=2, pages=PAGES, actions=ACTIONS), f, ensure_ascii=False, indent=1)
        f.write("\n")
    # what the overview's search can find
    idx = [dict(label=i["label"], page=p["id"], where="Settings › " + p["title"]) for p in PAGES for s in p["sections"] for i in s["items"] if i["key"]]
    idx += [dict(label=p["title"], page=p["id"], where="Settings") for p in PAGES]
    idx += [dict(label=l, page=pid, where="Settings › " + t) for pid, t, l in [
        ("network", "Network", "UC-Secure"), ("network", "Network", "eduroam"), ("network", "Network", "Wi-Fi networks"),
        ("network", "Network", "Repair this connection"), ("look", "Look", "Wallpaper"), ("system", "System & updates", "Login screen"),
        ("system", "System & updates", "X server (X.Org / XLibre)"), ("sound", "Sound", "Output device"), ("sound", "Sound", "Microphone")]]
    with open(os.path.join(os.path.dirname(out), "settings-index.json"), "w", encoding="utf-8") as f:
        json.dump(idx, f, ensure_ascii=False, indent=1)
    print("%d pages, %d settings -> %s" % (len(PAGES), len(keys), os.path.normpath(out)))

if __name__ == "__main__":
    main()
