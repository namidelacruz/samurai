# drive the X server like a person: key taps and typing through XTEST
import sys, time
from Xlib import X, XK, display
from Xlib.ext import xtest
d = display.Display()
def code(name):
    ks = XK.string_to_keysym(name)
    return d.keysym_to_keycode(ks)
def tap(*names):
    codes = [code(n) for n in names]
    for c in codes: xtest.fake_input(d, X.KeyPress, c)
    d.sync(); time.sleep(0.03)
    for c in reversed(codes): xtest.fake_input(d, X.KeyRelease, c)
    d.sync(); time.sleep(0.05)
def typ(text):
    for ch in text:
        name = {' ': 'space', '=': 'equal', '+': 'plus', '*': 'asterisk', '>': 'greater', '.': 'period', '-': 'minus'}.get(ch, ch)
        if ch in '+*>':
            tap('Shift_L', name)
        else:
            tap(name)
def click(x, y, b=1):
    xtest.fake_input(d, X.MotionNotify, x=x, y=y); d.sync(); time.sleep(0.05)
    xtest.fake_input(d, X.ButtonPress, b); d.sync(); time.sleep(0.05)
    xtest.fake_input(d, X.ButtonRelease, b); d.sync(); time.sleep(0.05)
def drag(x0, y0, x1, y1, steps=20, mods=()):
    for m in mods: xtest.fake_input(d, X.KeyPress, code(m))
    xtest.fake_input(d, X.MotionNotify, x=x0, y=y0); d.sync(); time.sleep(0.05)
    xtest.fake_input(d, X.ButtonPress, 1); d.sync(); time.sleep(0.05)
    for i in range(1, steps + 1):
        xtest.fake_input(d, X.MotionNotify, x=x0 + (x1 - x0) * i // steps, y=y0 + (y1 - y0) * i // steps); d.sync(); time.sleep(0.02)
    time.sleep(0.15)
    xtest.fake_input(d, X.ButtonRelease, 1); d.sync()
    for m in mods: xtest.fake_input(d, X.KeyRelease, code(m))
    d.sync(); time.sleep(0.1)
cmd = sys.argv[1]
if cmd == 'tap': tap(*sys.argv[2:])
elif cmd == 'type': typ(sys.argv[2])
elif cmd == 'click': click(int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]) if len(sys.argv) > 4 else 1)
elif cmd == 'drag': drag(*map(int, sys.argv[2:6]), mods=sys.argv[6:])
