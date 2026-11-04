---
Stable-Id: 01M6JW47RYART844WMEJMRVWTD
Status: Done
Phase: '0'
Track: governance-process
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/waiver.rs, test:rust/crates/urzua-cli/tests/waiver_stale_integration.rs
---
# 168 — A rule reports a waiver that matches no finding

## What

Add a rule that reports a waiver which matches no current finding.

## Why

A stale waiver is a silent no-op today: the finding it covered is gone but the exception stays on file, hiding that the exception is no longer needed and leaving room to cover a future finding by accident. The no-silent-no-op principle applies to the waiver record as much as to a rule.

## Done means

The rule names each waiver that matches nothing and the rule and target it claimed; a waiver that matches a finding is not reported; the rule is enabled in urzua's own `.urzua/config.yaml` and `urzua check` is clean over this repository's waivers, not only in unit tests

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Built as `waiver.matches-nothing` (opt-in, enabled in this repository's config). Reports an expired waiver, a waiver naming an unknown rule, and an active waiver whose rule ran over the whole corpus and found nothing in its scope; a partial run does not judge a live waiver. **Why:** the Done-when contract; scope semantics chosen so `--base` and path-scoped runs produce no false stale report. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged on `main`: opt-in rule `waiver.matches-nothing`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-177`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
