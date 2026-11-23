import os
from Xlib import display
try: display.Display(); print("connected with cookie")
except Exception as e: print("no:", type(e).__name__)
os.environ["XAUTHORITY"]="/dev/null"
try: display.Display(); print("OPEN WITHOUT COOKIE")
except Exception as e: print("refused without cookie")
