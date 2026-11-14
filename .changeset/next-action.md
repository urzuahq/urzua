---
default: minor
closes: []
---

Every command's JSON now carries a top-level `next_action` and `next_action_reason`, always present.
`next_action` is one runnable command, such as `urzua fix --apply`, or `null` when nothing is better
than reading the findings; `next_action_reason` is a short count-based reason such as
`repairs 3 of 5 findings`, and is `null` exactly when `next_action` is. `check` picks the command
from the unwaived findings' `run-command` actions (`urzua fix --apply`, then `urzua index --apply`,
then `urzua refs refresh`), `fix` points at `urzua fix --apply` after a detect run and `urzua check`
after an apply, and every other command emits both as `null`. The `urzua mcp` tools carry them too.
