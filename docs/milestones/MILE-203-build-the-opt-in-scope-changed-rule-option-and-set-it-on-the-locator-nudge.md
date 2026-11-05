---
Stable-Id: 01M6PYDB5CJ8TYKF2ZF9K004JW
Status: Planned
Phase: '0'
Track: governance-process
Implements: ADR-90
Blocked-on: null
---
# 203 — Build the opt-in scope: changed rule option and set it on the locator nudge

## What

Build the general, opt-in `scope: changed` rule option of `ADR-90` and set it on `embodiment.locator-on-back-pointer-path` in this repository's `.urzua/config.yaml`. A rule that declares it judges only the records a `--base` diff adds or changes, none on a plain `check`, and the population says why.

## Why

`MILE-189`'s nudge reports 398 findings on 158 closed records here, a backlog that can never be burned down; a nudge is only useful when someone touches the record. `ADR-90` (Proposed) records the maintainer's decision of 2026-11-05 to add the option rather than filter findings or waive them.

## Done means

- `scope: changed` loads as a rule option whose only value is `changed`; any other value is a load error naming it. A rule without it behaves byte for byte as before.
- Under `check --base`, a rule with the option judges only records in the diff's changed set (not dependents); the others are eligible and counted in the population's `out_of_scope`. On a plain `check` it judges none, is `ran`, and a notice states the count and the reason. `check --stdin-path` judges the proposed record.
- A rule that is not record-level (`Scope::Config`, `Files`, `Diff`, `Run`, or Directory-anchored), or that already judges only the changed set (`status.transition-allowed`, `revision-log.version-bump-required`), refuses the option at load with "does not take the option 'scope'".
- `waiver.matches-nothing` leaves a waiver on such a rule eligible and unexamined, and the `ADR-73` actions of the rule are unchanged.
- Tests fail before and pass after: each of the above, plus a planted unchanged record that is not nudged and a changed one that is.
- `.urzua/config.yaml` declares `scope: changed` on `embodiment.locator-on-back-pointer-path`; `urzua check` shows it eligible with 0 examined and 0 findings from it, `check --base` nudges only a changed record, and both exit 0.
- `SPEC-2`, `SPEC-3`, the `Outcome::OutOfScope` documentation and the generated rule table describe it; `make rule-table-check` passes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** Maintainer decision, 2026-11-05, recorded in `ADR-90` (Proposed). | **substantive** |
