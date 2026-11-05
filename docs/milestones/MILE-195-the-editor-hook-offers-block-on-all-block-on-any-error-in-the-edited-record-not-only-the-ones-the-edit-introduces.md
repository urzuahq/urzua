---
Stable-Id: 01M6NH5SQFSPY5GD8660QVB3X2
Status: Planned
Phase: '0'
Track: governance-process
Implements: ADR-78
Blocked-on: null
---
# 195 — The editor hook offers block_on all: block on any error in the edited record, not only the ones the edit introduces

## What

A hook option `block_on: introduced | all` (`ADR-78`'s amendment of 2026-11-05, proposed). `introduced` is the default and today's behaviour: the editor hook blocks only on an unwaived error the edit introduces. `all` blocks on any unwaived error in the proposed content of the edited record, including one the record already has on disk.

## Why

`MILE-191` built the hook as introduced-only and its Done means states that as the default. A repository with no zero-findings gate in CI, or one that wants a stricter editor layer, has no way to refuse an edit to a record that is already failing. The amendment records the option and its trade-offs; this milestone builds it. It is blocked on the maintainer accepting the amendment.

## Done means

- The hook reads the option from the configuration the way it reads `override_log`; absent or `introduced` leaves the verdicts and messages of `scripts/hooks/record-edit-hook.sh` byte-identical to today's; any other value is a loud not-judged or refusal naming the value, never a silent default.
- Under `all`, a recorded Write payload on a record that already has an error is blocked with that error listed, the second baseline `check` run is skipped, and the named override still lets it through and is recorded in the override log. A hook test fails before and passes after, and covers both values.
- `SPEC-3` (the key) and `SPEC-8` (the hook contract) describe the option with revision rows, and the Claude Code adapter passes it through unchanged.
- This repository's `.urzua/config.yaml` leaves the default (`introduced`), or declares it explicitly, and `urzua check` reports 0 unwaived findings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-78`'s amendment of 2026-11-05 (proposed, pending maintainer acceptance) adds `block_on` to the editor hook; nothing builds it. | **substantive** |
