---
default: patch
closes: []
---

The one-line description in `urzua --help` and the README no longer says the engine "never judges the change". It now says urzua checks every change against the rules you declared for the records it traces back to, gives the same answer every time, orders work from their declared dependencies, and has no opinion on the code itself.
