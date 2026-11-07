---
default: patch
closes: [MILE-34, MILE-159, MILE-171, MILE-172, MILE-173, MILE-175, MILE-178, MILE-179]
---

`check`'s staged loading is split into named helpers, each short enough to read on its own, with no
change to what `check` prints. The staged functions in `check.rs` are held to a stated size bound
by a test, and the output of every command is pinned by golden tests.
