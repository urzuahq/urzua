---
default: minor
closes: [MILE-202]
---

Add `urzua harness run post-command`, which runs after any shell command an agent runs and checks the records git shows as changed, untracked ones included, in one `check` call, then reports the unwaived findings back to the agent. It never reads the command and never blocks: a record that already failed at `HEAD` is listed as pre-existing, a record whose content has already been reported is not checked again, a deleted record is not judged, and a missing `git`, an unreadable configuration or a `check` that cannot run is stated as not judged. Turn it on with `post-command` under `harness.guardrails`; the Claude Code adapter wires it as a hook on its Bash tool.
