---
default: patch
closes: []
---

`authority.gates` path entries are read like `published_paths` (`./.agent` gates `.agent`), and an entry
that could match nothing (`.`, a backslash) is a load error, for `diff.decider-required` and the
`protected-paths` guardrail alike. A waiver `Expires` that is not a real calendar date (`2026-99-99`)
expires at once and says so. A re-baseline rewrites only the exact pin it names, `fix` reads a path with
glob characters literally when finding its last commit, and a file-discovery failure while resolving a
locator is reported rather than read as a missing file.
