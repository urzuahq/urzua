---
description: Re-pin a record's stale evidence pins with an honest, required reason, listed for the PR body
argument-hint: <record path> <what actually changed in the pinned code>
---

# /repin

Re-pin the stale `@sha256:` pins of one record with `urzua fix --rebaseline`. `$ARGUMENTS` is the record's
path and the reason.

The reason must say what actually changed in the pinned code and why the claim still holds, not that the
pin was stale. Before re-pinning, look at the diff of every pinned file the listing below names. Never
re-pin to quiet drift you have not read. The command passes `--actor agent`, so the revision row records
that an agent ran it, and it refuses an empty reason.

## Steps

1. Run it with `RECORD` and `REASON`. It lists the stale pins first, applies the re-pin only with a
   non-empty reason, and prints a `Re-pin:` line.

   ```bash step
   bash .claude/hooks/urzua-record.sh repin "$RECORD" "$REASON"
   ```

2. Put every `Re-pin:` line this command printed in the pull request body, so a reviewer can check that
   each reason matches the code change. Run `urzua check` and report its result.

If a merge or rebase conflicts in a record's pin, `Version` and re-pin rows, do not re-pin again by hand.
Run `urzua fix --conflicts` and read the listing. Resolve the records it lists as resolvable with
`urzua fix --conflicts --apply --ids <record,...> --actor agent`, `git add` them and continue the merge.
A refused record (it conflicts elsewhere, or its pinned file is not git's clean merge of two re-pinned
commits) is resolved by hand and then re-pinned here with a reason.
