---
default: major
---

A record type's `dir` means **that directory**, not that subtree (`RFC-35`).

Prefix matching let two types claim the same record whenever their directories
nested, and nothing in the schema resolved the overlap. Four heuristics accreted
in `urzua init` guessing what an adopter meant by it, and each re-entered the
previous one's failure -- the last silently dropped every record held directly
in a directory that also had two record-bearing subdirectories, while `check`
reported success over them.

`urzua init` now proposes **one type per directory holding records**: no
folding, no containers, no inference. `docs/adr/archive` is proposed as its own
type, which an adopter keeps, deletes or merges. Every record is accounted for
exactly once.

**Breaking** for a config whose `dir` relied on matching a subtree. Recursion is
not implemented: no corpus examined needs it, and it is added when one asks.

An empty option value is also now rejected rather than accepted:
`closed_statuses: []` made every claim a violation, and `claim_paths: []`
scanned nothing.
