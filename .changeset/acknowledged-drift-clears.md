---
default: patch
closes: []
---

`reverify` under `drift: acknowledge` renews a record whose drift has outlived `drift_grace_days`
without a prior `fix`, and the acknowledged renewal clears that drift for `embodiment.consistency`
(an unpinned locator is judged from the date of the acknowledging `Re-verified` row that names it; a
pinned one by its re-pin). Only a row in the exact shape `reverify` writes acknowledges or attests
anything, and `embodiment.claim-renewal-unattested` reports an added acknowledging row that names a
locator which had not drifted at the merge base or lists evidence that is not the record's current
locators. `check` prints the `citation.issue-exists` snapshot notice only when that rule is part
of the run.
