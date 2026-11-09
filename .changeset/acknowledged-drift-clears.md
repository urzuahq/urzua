---
default: patch
closes: []
---

`reverify` under `drift: acknowledge` renews a record whose drift has outlived `drift_grace_days`
without a prior `fix`, and the acknowledged renewal clears that drift for `embodiment.consistency`
(an unpinned locator is judged from the date of the acknowledging `Re-verified` row; a pinned one by
its re-pin). `check` prints the `citation.issue-exists` snapshot notice only when that rule is part
of the run.
