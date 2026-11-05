---
default: minor
---

Adds `urzua roadmap`, which computes the order of work from the records alone: waves, the ready set, the critical path, what each record blocks and a grouping, as JSON. It reads one graph over the sequencing pairs a new `roadmap.pairs` configuration key names, so a loop that crosses two pairs is reported as a cycle. A record that waits on a finished-unsuccessfully record, or on a cycle, is listed under `needs_replan` with a reason instead of appearing ready. A repository that declares no pair gets a note saying there is no roadmap rather than an empty one. `roadmap.group_by` names the header fields to group by, and `export --format json` carries the same `roadmap` object when `roadmap.pairs` is declared.
