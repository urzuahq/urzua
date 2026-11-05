---
default: patch
closes: [BUG-13]
---

`urzua check` and `urzua audit` now tell a reference to a record that exists on disk but is not tracked by
git apart from one that exists nowhere. The finding names the file and says it is not tracked, so a record
drafted beside the one citing it no longer reads like a typo. A `Realized-by` locator naming an untracked
file on disk is reported the same way. Tracking the file clears the finding.
