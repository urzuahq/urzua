---
default: minor
closes: []
---

The agent harness is quieter. The editor hook no longer prints "urzua did NOT judge this edit ... is
outside the repository" for a write it allows outside the repository (a note or a scratch file): this is a
new default, chosen by the maintainer, not today's behaviour. To get the message back, set
`report_outside_writes: true` under `harness` in `.urzua/config.yaml`. A refused write and an edit inside
the repository that urzua could not judge are still reported.

The post-command check prints nothing for a command that changed nothing governed. A change that arrived
by `git pull`, `checkout`, `merge`, `rebase`, `reset` or `stash pop` is not reported as the agent's edit and
is not repeated after later commands. What the command itself changed is reported once, with a short line
to the agent and, only for a gated move, a protected-path change or a check that could not run, a message
to you; the two never repeat each other. When Claude Code reports that a shell command changed no file, the
check is not started. Run `urzua harness update` to install the new translator.
