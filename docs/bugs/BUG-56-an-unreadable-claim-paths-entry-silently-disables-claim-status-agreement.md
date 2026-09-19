---
Stable-Id: 01M2W328KQGTSR7CGJ2T1CY52T
Status: Fixed
Found-in: 'A cumulative code review of v0.3.0..release -- reproduced with a typo of .changeset'
Regression-test: 'not yet written -- a `claim_paths` entry that is not a readable directory must exit not-run, naming it'
Blocked-on: —
---
# 56 — An unreadable `claim_paths` entry silently disables `claim.status-agreement`

## What was wrong

`read_claim_files` swallowed an unresolvable directory with `let Ok(entries) = read_dir(..) else
{ continue };`. A config saying `claim_paths: [".changesets"]` -- one letter off -- produced
`status: ok`, `blocking: false`, zero findings, and `rules_executed` reporting
`{status: "ran", records_examined: 0}`, against a corpus that genuinely contained a false close claim.

Indistinguishable from a clean run. `BUG-44` added `OptionRequired` precisely to stop this rule going
inert when `claim_paths` is *missing*; a misspelled one went through the same door.

## Fix (shipped)

Validated before any rule runs, because it is a config error rather than a finding about a record:
a `claim_paths` entry that is not a readable directory exits `not-run` and names it.

Verified: the typo'd config now reports
*"claim.status-agreement: claim_paths entry '.changesets' is not a readable directory"*.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed and fixed. **Why:** a rule that cannot reach its input reported success, which is this project's recurring failure in the one rule written to catch a false claim. | **substantive** |
