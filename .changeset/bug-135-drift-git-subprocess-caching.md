---
default: patch
---

`compute_drifted_records` (the git-history check behind `embodiment.consistency`'s drift detection)
spawned a `git log`/`git merge-base` subprocess per `Realized-by` locator per record, with no caching
across records or locators that share the same path or resolve to the same commit -- on a corpus
where several records cite the same shared file, the same commit history was looked up repeatedly.
Locator lookups and drift comparisons are now memoized per locator path and per commit pair for the
duration of a single `check` run. No behavior change; findings are identical.
