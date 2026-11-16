---
default: minor
closes: []
---

`urzua hooks install`, `status`, `uninstall` and `run` wire the pre-commit and pre-push git hooks that `doctor` reports, in `.git/hooks`, a `core.hooksPath` inside the repository, husky's `.husky/` or the pre-commit framework, without replacing an existing hook. `doctor`, `harness status` and the session-start check read the same layouts and name the command that wires a missing hook; `hooks:` in the config names what each hook runs.
