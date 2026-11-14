---
default: minor
closes: []
---

Blanking or un-accepting an Accepted record is now gated like deleting it. `diff.decider-required` reports a record that held a gated status at the merge base and is now empty, has no header, or has a status the gate does not name (Accepted to Proposed), as the finding `gated-departure-without-decider`, and the agent harness's editor hook refuses the same edit from an agent. A move between statuses the same gate names, and a decider's edit, are unchanged.

`urzua fix --apply`, `urzua fix --rebaseline --apply` and `urzua reverify --apply` now require `--actor agent` or `--actor human` when the attestation identity is `gh-verified` (the default). Without it the command is `not-run` (exit 2), says which flag is missing and writes nothing, so a revision row can no longer record `actor: unspecified` under a verified login and read as a person's own attestation of what an agent ran. To adapt, add `--actor agent` to scripts and agent commands, or `--actor human` when a person runs it. The harness commands `/repin`, `/reverify` and `/done` already pass `--actor agent`. An `asserted` or `git-config` identity is unchanged.
