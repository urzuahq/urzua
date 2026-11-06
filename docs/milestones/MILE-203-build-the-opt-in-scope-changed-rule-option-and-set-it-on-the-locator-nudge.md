---
Stable-Id: 01M6PYDB5CJ8TYKF2ZF9K004JW
Status: Done
Phase: '0'
Track: governance-process
Implements: ADR-90
Blocked-on: null
Realized-by: code:rust/crates/urzua-core/src/rules/shared.rs, code:rust/crates/urzua-core/src/rules/locator_mechanism.rs, code:rust/crates/urzua-core/src/rules/table.rs, code:rust/crates/urzua-core/src/rules/waiver.rs, code:rust/crates/urzua-core/src/config.rs, code:rust/crates/urzua-core/src/report.rs, code:rust/crates/urzua-cli/src/commands/check.rs, code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/scope_changed_integration.rs, spec:docs/specs/SPEC-2-urzua-check.md, spec:docs/specs/SPEC-3-configuration.md
---
# 203 — Build the opt-in scope: changed rule option and set it on the locator nudge

## What

Build the general, opt-in `scope: changed` rule option of `ADR-90` and set it on `embodiment.locator-on-back-pointer-path` in this repository's `.urzua/config.yaml`. A rule that declares it judges only the records a `--base` diff adds or changes, none on a plain `check`, and the population says why.

## Order

Sequence this after PR #357, which also edits `rules/table.rs` and `locator_mechanism.rs`, and after `ADR-90` is Accepted.

## Why

`MILE-189`'s nudge reports 398 findings on 158 closed records here, a backlog that can never be burned down (373 after #357 removes the 25 findings on `Fixed` records; re-state after it merges); a nudge is only useful when someone touches the record. `ADR-90` (Proposed) records the maintainer's decision of 2026-11-05 to add the option rather than filter findings or waive them.

## Done means

- `scope: changed` loads as a rule option whose only value is `changed`; any other value is a load error naming it. A rule without it behaves byte for byte as before.
- Under `check --base`, a rule with the option judges only records in the diff's changed set (not dependents); the others are eligible and counted in a new additive `deferred_by_scope` counter on the population (an additive `SPEC-2` output-contract change), not in `out_of_scope`. The unit is the `field` slot (494 eligible now), not the record. On a plain `check` it judges none, is `ran`, and one `info` notice states the slot count and the reason. `check --stdin-path` judges the proposed record.
- `scope` is a scalar, which `option_field` does not read, so a dedicated validation is built: a rule that does not list `scope` fails to load, and a value other than `changed` fails naming `changed`.
- `config.scope-matches-nothing` does not read `deferred_by_scope`: a test shows a plain `check` with the option declared raises no such warning, while a real `out_of_scope` still does.
- Only per-record rules take the option; cross-record rules (id collision, reference resolution and similar) do not list `scope`. A table-consistency test and a cross-check against `rule_accepts_option` enforce it.
- A rule that is not record-level (`Scope::Config`, `Files`, `Diff`, `Run`, or Directory-anchored), or that already judges only the changed set (`status.transition-allowed`, `revision-log.version-bump-required`), refuses the option at load with "does not take the option 'scope'".
- `waiver.matches-nothing` leaves a waiver on such a rule eligible and unexamined, and the `ADR-73` actions of the rule are unchanged.
- Tests fail before and pass after: each of the above, plus a planted unchanged record that is not nudged and a changed one that is.
- `.urzua/config.yaml` declares `scope: changed` on `embodiment.locator-on-back-pointer-path`; `urzua check` shows it eligible with 0 examined and 0 findings from it, `check --base` nudges only a changed record, and both exit 0.
- `SPEC-2` (including `deferred_by_scope`), `SPEC-3`, the counter's documentation and the generated rule table describe it; `make rule-table-check` passes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** Maintainer decision, 2026-11-05, recorded in `ADR-90` (Proposed). | **substantive** |
> | 2026-11-05 | Amended after independent review of PR #360: separate `deferred_by_scope` counter, scalar validation, slot units, cross-record rules, sequencing after #357. **Why:** Reusing `out_of_scope` would make `config.scope-matches-nothing` warn on every plain `check`. | **substantive** |
> | 2026-11-05 | Built and marked Done. `scope: changed` loads as a rule option (`RuleScope`, one value; a rule that does not list it, and any other value, fail to load); the rule table marks each rule `one-record`, `cross-record`, `changed-set` or `not-record-level` and only `embodiment.locator-on-back-pointer-path` lists `scope`; `deferred_by_scope` is a separate population counter (present only when non-zero) that `config.scope-matches-nothing` does not read; one `info` notice per rule says how many slots were not judged and why; dependents are not changed; `--stdin-path` treats the proposed record as changed; `waiver.matches-nothing` skips a waiver on such a rule. `.urzua/config.yaml` sets it on the locator nudge. `SPEC-2` is `0.105` and `SPEC-3` `0.93`. **Why:** the delivering change for `ADR-90`; the `Done means` above are each a test in `scope_changed_integration.rs`, `config.rs`, `report.rs`, `table.rs` and `waiver.rs`. | **substantive** |
