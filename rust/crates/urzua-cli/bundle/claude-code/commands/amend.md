---
description: Add a dated amendment and a revision-log row to a decided record, leaving what it decided unchanged
argument-hint: <record path> <what changed>
---

# /amend

Correct or extend a record that has already been decided without rewriting it. `$ARGUMENTS` is the record's
path and what changed. If the path is missing, ask for it.

A decided record's content is not edited to match a changed understanding. Instead, after the section
that states the decision, add a section headed `## Amendment (YYYY-MM-DD): <what changed>` (today's date)
that says what is now understood and why, and leave the decision text exactly as it was. Add one row to
the record's revision log saying an amendment was added and why. Do not change any header value.

## Steps

1. Read the record and the section that states its decision. Add the amendment section after it and the
   revision-log row at the end of the log, using the row format the log already uses. Change nothing else.
2. Finish. This refuses any change that is not an added line, an added amendment section or an added
   revision-log row, stages the record, and runs `urzua check` on it with its examined count.

   ```bash step
   .claude/hooks/urzua-record.sh amend-finish "$RECORD"
   ```

3. If `check` reports findings, fix the added text and run step 2 again. Report the path, the amendment
   heading and the `check` result with its examined count.
