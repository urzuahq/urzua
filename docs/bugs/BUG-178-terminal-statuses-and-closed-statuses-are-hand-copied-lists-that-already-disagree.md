---
Stable-Id: 01M6DHE7FTV278Q4GY103WZJ4P
Status: Open
Found-in: 'an inventory of implicit state machines across config, engine and specs (2026-11-02), confirmed by reading .urzua/config.yaml'
Regression-test: 'rust/crates/urzua-core/src/rules/claim.rs::a_claim_against_a_declared_failure_terminal_is_not_an_error_observed_failing, rust/crates/urzua-core/src/rules/narrative.rs::narrative_field_stale_reads_the_declared_terminals_observed_failing'
---
# 178 — terminal_statuses and closed_statuses are hand-copied lists that already disagree

## What was wrong

The set of finished statuses is written out by hand in several places, and the copies disagree:

- `narrative-field.stale` `terminal_statuses`: Fixed, WontFix, Accepted, Rejected, Superseded, Done,
  WontDo (`.urzua/config.yaml`).
- `claim.status-agreement` `closed_statuses`: Fixed, Done, Accepted, Rejected, Superseded, Withdrawn.
- The test helper `terminal_for_tests` in `rules.rs`, a fourth copy of the first list.

`closed_statuses` lacks WontFix and WontDo, so a changeset that claims to close a bug whose status is
WontFix is reported as an error, and it carries Withdrawn, which no record uses. Neither list has
`Not a bug`, which two bug records use (BUG-127, BUG-130), so a `Blocked-on` citing either never goes
stale.

## Why nothing caught it

No type declares its status vocabulary, so nothing can check that a status named in a rule option
exists, or that two rules mean the same set. Each list was added for its own rule, which is the
duplication `ADR-64` names.

## References

- `MILE-132` -- declared state machines, which replace these lists with one declaration per type.
- `ADR-64` -- structured in, structured out.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** Found by the implicit state machine inventory. | **substantive** |
> | 2026-11-02 | Dropped `pointer.target-status` `not_in` from the copies. **Why:** It is a separate, narrower policy (pointers must not target a Superseded record), not a list of finished statuses. | **substantive** |
> | 2026-11-04 | `narrative-field.stale` and `claim.status-agreement` read the declared `success` and `failure` statuses of the target's type when it declares `statuses`, and keep their list for a type that declares none; this repository declares all five types, so `WontFix` and `Not a bug` now finish a bug for both rules. The lists stay in `.urzua/config.yaml` as the fallback. `Status` unchanged. **Why:** `MILE-132` Phase 0 (`SPEC-3`). | **substantive** |
