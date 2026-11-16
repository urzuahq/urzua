---
default: minor
closes: []
---

The agent harness is quieter. The editor hook no longer prints "urzua did NOT judge this edit ... is
outside the repository" for a write it allows outside the repository (a note or a scratch file): this is a
new default, chosen by the maintainer, not today's behaviour. To get the message back, set
`report_outside_writes: true` under `harness` in `.urzua/config.yaml`. A refused write and an edit inside
the repository that urzua could not judge are still reported.

The post-command check prints nothing for a command that changed nothing governed. A gated move or a
protected-path change is reported once, to you, however it arrived; one that came in by `git pull`,
`checkout`, `merge`, `reset` or `stash pop` is worded as such and is not repeated after later commands.
Ordinary findings in records a pull or checkout brought in are not reported. The agent gets a short line
and you get a message only for a gated move, a protected-path change or a check that could not run; the two
never repeat each other. Run `urzua harness update` to install the new translator and hook commands.
