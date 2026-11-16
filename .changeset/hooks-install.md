---
default: minor
closes: []
---

`urzua hooks install`, `status`, `uninstall` and `run` wire the pre-commit and pre-push git hooks that `doctor` reports, in `.git/hooks`, a `core.hooksPath` inside the repository, husky's `.husky/` or the pre-commit framework. An existing hook is extended, never replaced, and only where the block will run; a hook that could end first, a non-shell hook, a symlinked hook or a hooks directory outside the repository is refused with the line to add. `doctor`, `harness status` and the session-start check read the same layouts, report a call that can never run as unreachable, and name the command that wires a missing hook. `hooks:` in the config names the `check` each hook runs and `on_not_run` (default `warn`: the hook says the records were not checked and lets the commit through; a blocking finding always blocks).
