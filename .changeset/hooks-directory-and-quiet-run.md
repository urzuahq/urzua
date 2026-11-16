---
default: minor
closes: []
---

A configuration can name the directory its git hooks are committed in (`hooks.path`). `urzua hooks install` points this clone's `core.hooksPath` at it, refusing and naming the current value when another is set, and `urzua hooks uninstall` unsets it only when it is that directory; while a clone is not pointed there, `urzua hooks status`, `urzua doctor` and the session-start report say so, name the directory and give `urzua hooks install` as the next action, instead of asking for a hook under the clone's own hooks directory. The session-start report adds its `on_unjudged: deny` paragraph only when a problem other than a git hook is listed. `urzua hooks run` now prints nothing when no finding blocks; `hooks.print_success: true` restores the `no blocking findings` line, and a refusal is unchanged.
