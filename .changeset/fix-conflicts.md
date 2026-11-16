---
default: minor
closes: []
---

`urzua fix --conflicts` resolves the records a merge or rebase left conflicted when two branches each
re-pinned them. Without `--apply` it lists every unmerged record by kind of conflict, resolvable or
refused with the reason; with `--apply` it keeps both branches' revision-log rows in date order, takes
the higher `Version`, recomputes each pin both sides moved from the merged file and appends a row saying
the pin was recomputed on merge. A record that conflicts in anything else, or whose pinned file is
itself conflicted, is refused and left untouched. It never stages. While records are unmerged, `urzua
check` and `urzua fix --rebaseline` suggest it as the next action.
