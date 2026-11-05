---
default: minor
closes: []
---

`urzua fix` takes record paths: `urzua fix docs/adr/ADR-1-x.md` examines, computes drift for and
repairs only that record, and a directory selects the records under it. Drift and `git blame` work
only for the selected records, so a single-record run is several times faster than a whole-corpus
run. A path that matches no record is reported as a failure naming it instead of being skipped, and
`--ids` and `--force` choose among the scoped repairs only.
