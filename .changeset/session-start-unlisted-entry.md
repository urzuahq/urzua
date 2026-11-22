---
default: patch
closes: [MILE-354]
---

The session-start check names a hook entry the current bundle wants that the lock does not list (such as the two AskUserQuestion entries a release added after the install) as not wired, and says to run `urzua harness install`. Before, it walked only the lock's entries, so a repository installed before such a release heard nothing at session start.
