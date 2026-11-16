---
default: minor
closes: []
---

A change that needs two records to agree (one record superseding another under a declared relation pair, say) can now be made through the edit tools, without the shell.

- `urzua fix` lists the missing other half of a relation that `relation.pair-reciprocity` or `relation.supersession-reciprocity` reports, and `urzua fix --apply --ids <record>` writes it, with a revision-log row. It never changes a `Status`, never overwrites a value that names no record, and writes nothing for a target that does not exist. The reciprocity findings now offer this command (`run-command`) instead of an `edit-field` `append`.
- **The editor hook's default changed.** A new key, `harness.guardrails.editor-hook.pending_pair`, defaults to `allow`: an edit whose only new errors are the missing other half of a pair is now allowed, and the agent is told the record, the field and value, and the `urzua fix` command still required. Before, the edit was refused. To keep the refusal, set `pending_pair: refuse` under `harness.guardrails.editor-hook`; the refusal now names the `urzua fix` command. Any other new error still refuses, the acceptance and protected-path gates are unchanged, and an unmatched pair is still reported after a command and blocked by pre-commit and CI.
