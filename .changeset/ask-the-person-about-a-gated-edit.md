---
default: minor
closes: []
---

Opt-in: ask the person about a gated edit in the agent session. `harness.guardrails.acceptance` takes `in_session: ask` (default `refuse`, which behaves exactly as before). With it, an agent's Write, Edit or MultiEdit that moves a record into a status the `authority` table reserves goes to the person through the agent host's permission prompt instead of being refused. The hook writes the prompt's reason (the record, the status it leaves and enters, the decider group) and adds the receipt to the edit: the entered status's required date fields and a revision-log row reading that the move was approved in an agent session by the named person.

The edit stays refused where the person cannot be asked: a subagent, and every Claude Code permission mode but Manual (`default`). A confirmed approval, kept under `.git/urzua/approvals` and bound to the record, the status, the file's bytes, the session and an expiry (`approval_minutes`, default 60), lets the pre-commit check accept the move without an actors file. CI is unchanged: `diff.decider-required` still requires the actors file to name a decider.

The Claude Code bundle gains a `PostToolUse` hook on Write, Edit and MultiEdit (`urzua harness hook claude-code post-edit`); run `urzua harness update` after upgrading.
