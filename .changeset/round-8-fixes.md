---
default: patch
---

Fixes BUG-77: `check` and `audit` report `not-run` rather than `ok` when no rule examined anything.

Fixes BUG-76: a tracked record the loader cannot read stops the run instead of silently shrinking the
corpus. A staged deletion is still legitimately absent.

Fixes BUG-75: a symlinked claim file is read rather than skipped, and a symlinked directory inside
`claim_paths` aborts with a message saying so.

Fixes BUG-80: a `claim_paths` root symlinked to a directory inside the repository is usable; the
target is resolved and required to stay under the repository root.

Fixes BUG-79: adds the opt-in `identity.collision` rule, so two records claiming one identifier are
reported rather than one of them silently disappearing from the reference index.

Fixes BUG-78: `header.field-set-consistency` no longer counts a record whose header never parsed as
examined.

Fixes BUG-81: `RuleExecution` carries a `scope`, so a rule that counted configuration entries can no
longer satisfy the guard that a corpus record was examined.
