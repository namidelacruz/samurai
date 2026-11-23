# a test window: python-xlib, a title, a solid colour and some text-like bars
import sys, time
from Xlib import X, display, Xatom
title = sys.argv[1] if len(sys.argv) > 1 else "Test"
col = int(sys.argv[2], 16) if len(sys.argv) > 2 else 0x2a2e36
w, h = (int(sys.argv[3]), int(sys.argv[4])) if len(sys.argv) > 4 else (640, 400)
d = display.Display(); s = d.screen()
win = s.root.create_window(0, 0, w, h, 0, s.root_depth, X.InputOutput, X.CopyFromParent, background_pixel=col,
                           event_mask=X.ExposureMask | X.StructureNotifyMask | X.KeyPressMask)
win.set_wm_name(title); win.set_wm_class(title.lower(), title)
win.change_property(d.intern_atom('_NET_WM_NAME'), d.intern_atom('UTF8_STRING'), 8, title.encode())
win.set_wm_protocols([d.intern_atom('WM_DELETE_WINDOW')])
gc = win.create_gc(foreground=0xdedcd5 if col < 0x808080 else 0x1e1e20)
win.map(); d.flush()
while True:
    e = d.next_event()
    if e.type == X.Expose:
        for i, frac in enumerate([0.6, 0.3, 0.75, 0.5, 0.4, 0.65]):
            win.fill_rectangle(gc, 24, 30 + i * 28, int((w - 48) * frac), 10)
        d.flush()
    elif e.type == X.ClientMessage:
        break
    elif e.type == X.DestroyNotify:
        break
