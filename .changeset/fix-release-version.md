---
default: patch
---

Fixes BUG-92: resets the in-tree version to `0.3.0` and removes a generated CHANGELOG section that
reached `main` from a release-prep branch. No `0.4.0` was ever published, so the next release computes
`0.4.0` from the pending changesets rather than `0.4.1`.
