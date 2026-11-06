## urzua harness

This section is written by `urzua harness install` and is rewritten between its markers on every install or update. Do not edit it by hand; change `harness` in `.urzua/config.yaml` instead.

The record-edit guardrails are installed in this repository. What an agent must not do:

- Do not accept or reject a record yourself. Moving a decision record to an accepting status is a person's act.
- Do not set a hook override to get a blocked edit through. A named override is recorded for review; it is the maintainer's to take.
- Do not edit the harness itself: the agent's hook settings, the hook files under `.claude/hooks`, `.git-hooks/`, `.urzua/config.yaml` and the CI workflows are protected paths.
- Write records with their drafting commands and `urzua new`, not by hand; run `urzua check` before committing.

Where `authority` is declared, the engine refuses an agent's Write, Edit or MultiEdit that moves a record to a status a gate names or writes a protected path, and no override lifts it; a person edits outside the agent harness. That is the whole claim. Accepted risk, documented by the maintainer on 2026-11-05: a shell command that writes a status or edits a guardrail (reported after the command, not stopped), `git commit --no-verify`, a pull request that changes its own workflow, and an agent acting under the maintainer's token (CI proves who authored the pull request, not who decided) all get past it. Do none of these.

The editor hook judges a Write, Edit or MultiEdit of a record before it lands. It cannot see a shell command that writes a record, so run `urzua check` after one. Only the verdicts of `urzua harness run` bind; this text is advisory.
