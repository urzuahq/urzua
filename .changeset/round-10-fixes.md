---
default: patch
---

Fixes BUG-87: a record whose filename contains a non-ASCII character is part of the corpus. git
C-quotes such paths under its default `core.quotepath`, so the name arrived wrapped in quotes, its
parent never matched the declared `dir`, and the record was dropped — a planted violation was
invisible and the run exited 0. Discovery reads git's output NUL-separated.

Fixes BUG-89: a `claim_paths` symlink pointing at an ancestor of the declared prefix is refused by
name rather than followed, so the walk cannot sweep the whole repository and report blocking findings
about files never declared as claims.

Fixes BUG-90: `claim.status-agreement` counts the records a claim resolved against, not the claim
files it read, so its number can no longer exceed the size of the corpus.
