---
description: Write a milestone for decided-but-unbuilt work from the conversation, scaffolded through urzua new and verified by urzua check
argument-hint: <title>
---

# /milestone

Write one `milestone` record from the current conversation. `ADR-70` governs this command: it
scaffolds through `urzua new`, carries no schema knowledge of its own, and `urzua check` verifies
the result. The title is `$ARGUMENTS`; if it is empty, derive one from the conversation.

Use this for planned work: decided, deferred or unrouted. Point it at the RFC or ADR it realizes only through a reference that resolves.

The template carries a section that states when the milestone is finished. Fill it with concrete,
checkable criteria a reviewer can verify against the repository (the command, rule or file and what it
reports), never a restatement of the What, and include that the capability is exercised in this
repository's own configuration and records where that fits. Step 4 refuses a record without it.

## Steps

1. Scaffold. Run this and keep the printed `path`; every later step names it as `RECORD`.

   ```bash step
   scripts/draft-record.sh new milestone "$TITLE"
   ```

2. Read the scaffolded file and `.urzua/config.yaml` for this type. Fill the body sections and the
   header fields that `urzua new` left `null`, from the conversation only. Take the section
   names, field names and allowed values from the file and the config, never from memory or from
   a nearby record. If `urzua rules` or the config says a field is a pointer, give it a clean
   reference that resolves. Leave a field `null` when the conversation does not supply it.
3. Never decide. Do not change any value `urzua new` wrote (its status, dates, author, ids), do
   not add a field it did not write, and do not set an acceptance, completion or fix state or
   date. Those are the maintainer's decisions. Step 4 refuses a record that does.
4. Finish. This refuses a changed scaffold value, stages the record so `check` governs it,
   regenerates the declared indexes with `urzua index --apply` and stages them, then runs
   `urzua check` on the record and prints how many records it examined.

   ```bash step
   scripts/draft-record.sh finish "$RECORD"
   ```

5. If `check` reports findings, fix the record's content (not the config, not a waiver) and run
   step 4 again. Do not report success on a record `check` did not examine (zero examined is a
   failure) or on one with unwaived findings.
6. Changeset. If the change touches what someone installing `urzua` would care about, add
   `.changeset/<slug>.md` with `default: patch|minor` and `closes: [MILE-N]`, listing only records
   the same PR leaves closed (a milestone the PR fully delivers; AGENTS.md says how it is marked), or `closes: []` for none. A milestone the
   change only mentions is not listed.
7. Report to the user: the path written, the record's status as scaffolded and that it is theirs
   to change, the indexes regenerated, and the `check` result with its examined count.
