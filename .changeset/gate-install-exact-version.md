---
default: patch
closes: []
---

The `urzua-gate` action's install step now accepts a downloaded binary only when the version it
reports is exactly the pinned release, so a binary reporting `1.2.35` no longer passes for a pin of
`1.2.3`.
