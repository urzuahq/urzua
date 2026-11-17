---
description: Renew an expired claim with urzua reverify and an honest, required reason, listed for the PR body
argument-hint: <record path> <what you checked>
---

# /reverify

Renew the claim of one record with `urzua reverify`. `$ARGUMENTS` is the record's path and the reason.

The reason must say what you read and why the claim still holds, not that the claim was old. Before
renewing, read every locator and span the review below lists, and the code each one points at. Never
renew a claim you have not read. The command passes `--actor agent`, so the revision row records that an
agent ran it, and it refuses an empty reason.

`reverify` renews only over evidence that has not changed. If it refuses a changed pin, run `/repin`
first, with its own reason; if it refuses a drifted locator that has no pin, pin it with
`urzua reverify --pin-only` (a separate act with its own reason), then run this command again.

## Steps

1. Run it with `RECORD` and `REASON`. It lists the review first, applies the renewal only with a
   non-empty reason, and prints a `Re-verified:` line (and a `Pinned:` line when it wrote a first pin).

   ```bash step
   bash .claude/hooks/urzua-record.sh reverify "$RECORD" "$REASON"
   ```

2. Put every `Pinned:` and `Re-verified:` line this command printed in the pull request body, so a
   reviewer can check that each reason matches what was read. Run `urzua check` and report its result.
