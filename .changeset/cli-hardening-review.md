---
default: patch
closes: []
---

Hardening from a review of the CLI. `check --base` pulls in an unchanged record whose `Realized-by`
glob covers a changed file, so it no longer exits 0 where a full check exits 1. A shallow clone
without the base's history is reported as a shallow clone, with the fix (fetch full history), not as
unrelated histories. `status.transition-allowed` counts a merge-base copy it cannot read as
unreadable, with a notice, instead of as a new record. Records, the config and index artifacts are
written through a temporary file and a rename, so a failed write cannot leave a truncated file.
`fix --apply` and `--rebaseline` refuse an identity containing `|` or a line break. `migrate ids
--apply` reports `partial-failure` when a write fails, takes the `fix` lock and re-reads before
writing. `doctor` reads the global `--config`. The suggested edit for an uncovered back-pointer path
targets the configuration file.
