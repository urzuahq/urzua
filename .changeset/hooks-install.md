---
default: minor
closes: []
---

`urzua hooks install`, `status`, `uninstall` and `run` wire the pre-commit and pre-push git hooks that `doctor` reports, in `.git/hooks`, a `core.hooksPath` inside the repository, husky's `.husky/` or the pre-commit framework. An existing hook gets a marked block at its top, so urzua runs first and the rest of the hook is unchanged; `uninstall` restores it byte for byte. A non-shell hook, a symlinked hook or a hooks directory outside the repository is refused with the line to add. `doctor`, `harness status` and the session-start check read the same layouts, read only urzua's own block as wired (any other mention of urzua is reported as an unmanaged call), and name the command that wires a missing hook. `hooks:` in the config names the `check` each hook runs and `on_not_run` (default `warn`: the hook says the records were not checked and lets the commit through; a blocking finding always blocks). A hook the repository maintains itself is declared (`hooks.<hook>.managed: declared`) and proven by `urzua hooks verify --run`, which runs it once, for real, and reports whether it reached urzua.
