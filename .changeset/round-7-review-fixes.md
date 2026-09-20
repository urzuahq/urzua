---
default: patch
---

Fixes BUG-69 more completely: a `claim_paths` root that is itself a symlink is rejected rather than
traversed, and a traversal or read failure aborts the run instead of yielding a clean result over an
incomplete set of claims.
