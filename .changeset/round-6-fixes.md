---
default: patch
---

`check`'s path argument narrows what is reported on, not what a pointer resolves against (BUG-60).

`init` writes the type prefix its corpus actually uses, and declines to propose rules that corpus
gives no identity to read (BUG-61).

A record-shaped file below a declared type dir but not in it is reported by the new opt-in
`type.record-outside-declared-dir` rule rather than dropped in silence (BUG-62).

`claim_paths` is read to any depth, as its own description says (BUG-63).

An empty `#` line no longer abandons the H1 scan (BUG-64).
