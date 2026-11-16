---
default: minor
closes: []
---

`urzua fix --conflicts` resolves the records a merge or rebase left conflicted when two branches each
re-pinned them. Without `--apply` it lists every unmerged record by kind of conflict, resolvable or
refused with the reason; with `--apply` it keeps both branches' revision-log rows in date order, takes
the higher `Version`, recomputes each pin both sides moved and appends a row saying the pin was
recomputed on merge. A pin is recomputed only when the merged file is git's automatic merge of two
commits whose files match their branches' pins; a hand edit during the merge, a change on a branch
without a re-pin, or a conflict anywhere else is refused and left untouched. It never stages. While
records are unmerged, `urzua check` and `urzua fix --rebaseline` suggest it; `urzua check --base` lists
every pin a pull request recomputed on merge. A type may declare `revision_log.evidence_class`, the
class of re-pin rows; left out of `version_bump.classes`, a re-pin no longer bumps the version.
