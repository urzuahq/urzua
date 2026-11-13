---
Stable-Id: 01M6SSWHYEH8Y1VBY0X1SRG44Q
Status: Planned
Phase: '1'
Track: accountability-identity
Depends-on: MILE-201
Blocked-on: null
---
# 210 — Say in the fix help that the verified gh login replaces --by under the default attestation policy

## What

Document in `urzua fix --help` that, under the default `attestation.identity: gh-verified`, an authenticated `gh` login replaces `--by` in `fix --apply` and `fix --rebaseline --apply`, and give `--ids`, `--by`, `--tier` and `--actor` help text. Check whether the replacement should be reported more visibly in the one path where it is silent.

## Why

The adopter second pass (M8) found `fix --apply --by some-person` recorded as the verified login. `MILE-201` made the identity source configurable and records it in the revision row, but it did not change the default, which still replaces `--by`, and the `--by` option has no help text. So an adopter cannot learn the rule from the command. `MILE-201` did not fix this: the replacement is deliberate and undocumented in the help.

## Done means

- `urzua fix --help` explains the replacement, the `asserted` alternative and `require_verified`, and every option of `fix` has help text.
- The report names a replaced `--by` in a field in every path that writes a row, and a test covers `--apply` and `--rebaseline --apply`.
- The `fix` help no longer says "Only Tier 1" where more is applied.
- `SPEC-8` states it with a revision row.
- A patch changeset names the change.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Initial record. **Why:** deferred during Phase 0 hardening and tracked by no record. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
