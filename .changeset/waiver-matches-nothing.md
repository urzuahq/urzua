---
default: minor
closes: []
---

Add `waiver.matches-nothing`, an opt-in rule that reports a waiver that does nothing: one that is expired, one naming a rule this build does not have, and one whose rule ran over the whole corpus and found nothing in its scope. Each finding is on the waiver record and names the rule and scope it claimed. A path-scoped or `--base` run, or a waiver whose rule is off, does not judge a live waiver, so a partial run produces no false stale report.
