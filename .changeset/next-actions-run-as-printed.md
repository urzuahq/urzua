---
default: minor
closes: [MILE-266, MILE-267]
---

Every command an urzua report prints now runs as printed. `fix --apply` next actions carry `--actor agent`, so they no longer stop with "--actor agent or --actor human is required" under a `gh`-verified login. A new key, `next_action_inputs`, is always an array: it is empty when `next_action` runs as is, and otherwise lists each `<...>` placeholder the command carries, with its flag and what to put there. The re-pin and renewal listings name their apply forms this way.

`check` on a repository whose records are untracked names `git add -- :/<paths>` as its next action, and `urzua new` does the same for the record it wrote. A pinned file that changed leads to the re-pin listing (`urzua fix --rebaseline --ids <record>`, variant `embodiment-drift-pinned`). `fix --apply` no longer writes `Drift detected` over a stale pin, so following next actions cannot turn that drift green without a re-pin reason.

Every built-in template from `urzua init --types` ships a revision log with a dated first row. `fix --apply`, `fix --rebaseline --apply` and `reverify --apply` append to a revision-log table that has no row yet. A mistyped `--rules` or `--rule` id names the nearest one, and an unresolvable `--base` names the ref that could not be resolved, with a `git fetch` next action for a remote-tracking ref.
