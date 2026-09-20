---
default: patch
---

Fixes BUG-82: only a staged deletion is treated as a legitimate absence, so a tracked record deleted
from the worktree stops the run instead of shrinking the corpus in silence.

Fixes BUG-83: `RuleScope` gains `Paths` for a rule that examines tracked path names without opening a
file, so it can no longer certify a corpus on its own.

Fixes BUG-84: `audit` reports `ok` on a clean corpus whose records carry no relationships. The guard
requires that a record-scoped rule ran, not that it examined a non-zero count.

Fixes BUG-85: a directory symlink inside `claim_paths` is resolved and followed when it stays inside
the repository, matching how the declared root is already treated; a visited set bounds the walk.
