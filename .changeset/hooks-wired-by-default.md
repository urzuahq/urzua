---
default: minor
---

`urzua doctor` reports whether the `pre-commit` and `pre-push` git hooks are wired. Each layer
appears as a `hook-wired` check with its `layer` and a `state`: `not-installed`,
`installed-running-urzua`, `installed-not-running-urzua` or `installed-not-executable`. It follows
`core.hooksPath` when one is set.
