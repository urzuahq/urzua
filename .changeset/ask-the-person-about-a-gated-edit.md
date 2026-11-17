---
default: minor
closes: []
---

Opt-in: ask the person about a gated edit in the agent session. `harness.guardrails.acceptance` takes `in_session: ask` (default `refuse`). With it, an agent's Write, Edit or MultiEdit that moves a record into a status the `authority` table reserves goes to the person through the agent host's permission prompt instead of being refused, in the Claude Code permission mode `default`, from the main session, when the session's name is a login in the gate's group. The hook writes the prompt's reason and adds a receipt row to the edit, which begins "Recorded as approved in an agent session" and ends "not verified; the pull request check does not rely on it". Approving writes the file; declining writes nothing. Nothing is stored and no hook is added.

The local commit check is unchanged except that, under `require_evidence` and with no actors file, a gated move whose record carries the receipt row is reported at info as unverified. The row is text anything able to edit the record can write, so a repository that relies on local `require_evidence` should not turn the mode on; the pull request check with the actors file is the control. On a repository with `require_evidence` off (the default), a gated status change made by shell and committed is not refused locally; the local check reports it at info and the pull request check decides.

With the mode off, one change applies to every repository with `protected-paths` on: the edit tools are refused writes under any git directory and to `.git` pointer files, including a git directory outside the working tree (a separate git directory, a symlinked `.git`, a submodule's, a bare repository's), because its hooks and config are there.
