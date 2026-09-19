---
default: patch
---

Fixes BUG-60: `check`'s path argument narrows what is reported on, not what a pointer resolves
against.

Fixes BUG-61: `init` writes the type prefix its corpus actually uses, and declines to propose rules
that corpus gives no identity to read.

Fixes BUG-62: a record-shaped file below a declared type dir but not in it is reported by the new
opt-in `type.record-outside-declared-dir` rule rather than dropped in silence.

Fixes BUG-63: `claim_paths` is read to any depth, as its own description says.

Fixes BUG-64: an empty `#` line no longer abandons the H1 scan.
