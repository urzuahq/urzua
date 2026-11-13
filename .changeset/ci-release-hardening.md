---
default: patch
closes: []
---

The `urzua-gate` action now verifies the downloaded binary's build provenance against the release
workflow that published it, not just the repository, and fails with a could-not-run result when its
`working-directory` cannot be entered instead of checking the wrong tree.
