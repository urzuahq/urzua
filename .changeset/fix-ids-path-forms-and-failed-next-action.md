---
default: patch
closes: [BUG-207, BUG-208]
---

`urzua fix --ids` now takes a record named any way you write its path. A `./`-prefixed path, an absolute path inside the repository and a path written with the operating system's separator all select the same record as `docs/adr/ADR-1-title.md`, in every mode that takes `--ids` (plain, `--rebaseline`, `--pin` and `--conflicts`). Before, such an entry matched nothing and `fix --apply` wrote nothing, which is what happened when the repair the editor hook names was run with a path in another form. An absolute path outside the repository still fails and names the entry. A run that fails because an `--ids` entry matched no pending repair now has a `next_action`: `urzua fix` (`urzua fix --rebaseline` for a re-pin), the dry run that lists the pending repairs by path. Exit codes are unchanged and now stated: a configuration that fails to load exits 2 in every `fix` mode, as `check` does, and a failed entry exits 1.
