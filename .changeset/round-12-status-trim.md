---
default: patch
---

`pointer.target-status` and `claim.status-agreement` compare a target's `Status` with surrounding
whitespace trimmed, matching what `narrative-field.stale` already did. A YAML-quoted value such as
`Status: "Superseded   "` keeps its trailing space, and the untrimmed comparison let a record that
declared a status the rule was configured to report slip past it silently.
