---
default: patch
closes: []
---

`urzua fix` computes embodiment drift only when `embodiment.consistency` is enabled, as `check`
does, and both commands run their `git blame` calls in parallel, so a whole-corpus run is
noticeably faster. A shallow clone, or any git call that fails while computing drift, is now
reported as an `embodiment-drift-git` notice instead of reading as no drift.
