---
default: patch
closes: [BUG-213]
---

A record type that declares `fields: {from: prefix-lines}` has its merge-base copy read the way the loader reads it. Before, a `--base` run read that copy as YAML front matter, so the decider gate (`diff.decider-required`) treated a move from Proposed as unreadable, `revision-log.version-bump-required` read the base version as absent, and a deleted record's locators, a changed waiver's old `Scope` and a child record's base status were lost when deciding which unchanged records a change affects. A pure rename of such a record is paired with its base copy by `Stable-Id` rather than judged as a new record by `header.scaffold-mismatch`. `urzua fix` also pairs such a record by `Stable-Id`, so a revision log deleted and the record re-added at another path is refused rather than replaced.
