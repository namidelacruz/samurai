#!/usr/bin/env python3
"""Drive the login screen against greetd's own test server (fakegreet): wrong password, then the
right one with its extra question. Run as: fakegreet python3 tests/greeter_fake.py
fakegreet's rules: user "user", password "password", then it asks "7 + 2:" (answer 9)."""
import os, sys
from pathlib import Path
os.environ.setdefault("QT_QPA_PLATFORM", "offscreen")
os.environ["SAMURAI_GREETER_CONF"] = os.environ.get("SAMURAI_GREETER_CONF", "/dev/null")
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "greeter"))
import greeter
from PySide6.QtGui import QGuiApplication
app = QGuiApplication(sys.argv)
quits = []
QGuiApplication.quit = staticmethod(lambda: quits.append(1))
c = greeter.Core()
said, asked = [], []
c.said.connect(lambda t, b: said.append((t, b)))
c.asked.connect(lambda t, s: asked.append((t, s)))
def run(fn):
    fn(); [app.processEvents() for _ in range(3)]
    import time; deadline = time.time() + 8
    while c.busy and time.time() < deadline:
        app.processEvents(); time.sleep(0.02)
    for _ in range(50): app.processEvents()
checks = []
def check(name, ok): checks.append(ok); print(("  ok   " if ok else "  FAIL ") + name)
run(lambda: c.login("user", "wrong", 0))
# fakegreet asks its extra question before judging; we answer it wrong too
check("the extra question is shown", asked and "7 + 2" in asked[-1][0])
run(lambda: c.answer("1"))
check("a wrong password is refused", said and said[-1] == ("Wrong user or password.", True) and not quits)
run(lambda: c.login("user", "password", 0))
check("right password: the extra question comes", len(asked) == 2)
run(lambda: c.answer("9"))
check("then the session starts and the greeter leaves", quits == [1])
print(f"{len(checks)} checks, {checks.count(False)} failed")
sys.exit(0 if all(checks) else 1)
