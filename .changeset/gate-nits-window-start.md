---
default: patch
closes: []
---

A drift-acknowledging `Re-verified` row written just before UTC midnight on a branch whose first
commit is authored after midnight is no longer reported by `embodiment.claim-renewal-unattested`:
the date window starts one UTC day before the first commit. On a branch with no commit yet, the
window starts at the merge base, so a row written yesterday and not yet committed passes a local
`--base` run. The harness `post-command` hook reports a deleted gated record once, not on every
second command, and `check --base` warns when it cannot read a deleted record's base copy.
