---
default: patch
closes: []
---

Hardening, four decided items. `identity.open-pr-collision` now tells the reader to rename the file
and its H1 to the next free number, or to waive the finding when the other pull request is abandoned,
instead of pointing at `urzua migrate ids`, which cannot renumber. `fix` without `--apply` honours
`--ids` (an entry that matches nothing is a failure, as under `--apply`) and rejects `--force`, `--by`
and `--actor` as usage errors instead of ignoring them. A waiver's required field counts as a
placeholder only when its whole value is a declared pending value or the template's text, so a
`Reason` such as "pending the upstream fix" takes effect, and a placeholder is reported as a
placeholder, not as missing. A name under `harness.guardrails` that this build does not ship is a
configuration load error for every command, with the nearest valid name suggested, and `doctor` no
longer reports "no unrecognized keys" beside an unrecognized harness entry.
