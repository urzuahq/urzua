---
Stable-Id: 01M2WG83X4FDF5ADVSCPF7P434
Status: Fixed
Found-in: "Round 7 of the 0.4.0 review"
Regression-test: 'rust/crates/urzua-cli/src/commands/init.rs::a_rule_reading_a_declaration_init_does_not_write_is_not_proposed_observed_failing, rust/crates/urzua-cli/tests/check_integration.rs::init_does_not_propose_rules_that_would_examine_nothing_on_the_corpus_it_adopted'
---
# 73 — init proposes rules whose declarations it did not write so they examine zero records

## What was wrong

`init` writes `required_fields: []` and no `known_fields`, `header_layout` or `pointer_fields`, then
proposes `field.quality`, `field.pending`, `header.field-set-consistency`,
`header.layout-consistency` and `header.pointer-field-clean` at `warn`.

All five read a declaration `init` did not write. On an adopted corpus each reports
`status: ran, records_examined: 0` -- structurally incapable of firing given the configuration `init`
itself produced.

`init` already declines to propose a rule whose required *options* it cannot supply, and (after
`BUG-70`) a rule whose identity assumption the corpus does not satisfy. The same judgement extends to
a rule whose required *declaration* `init` left empty: either propose the rule and the declaration it
needs, or do not propose the rule.

Filed rather than fixed with the rest of round 7: which declarations `init` should infer from a
corpus, versus leave to the adopter, is a design question and `ADR-33` already decided that adopt
mode proposes where to grow rather than preserving what it found.

## Why nothing caught it

Adopt mode's tests assert on the configuration `init` writes. None runs `check` with that
configuration and asserts the proposed rules reached a verdict. A config that is well-formed and
inert passes every test there is.

## Decision (2026-11-04)

`init` stops proposing the five declaration-dependent rules (`field.quality`, `field.pending`, `header.field-set-consistency`, `header.layout-consistency` and `header.pointer-field-clean`) until their declarations exist, and tells the adopter what to declare first. It infers nothing from the corpus (`ADR-33`).

## References

- `BUG-61` and `BUG-70`, the same judgement applied to options and to identity.
- `ADR-33`, on what adopt mode proposes.
- `MILE-51`, the adopt-a-foreign-corpus validation.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Recorded the chosen fix: `init` stops proposing the five declaration-dependent rules until their declarations exist and tells the adopter what to declare first; it infers nothing from the corpus (`ADR-33`). `Status` stays `Open`. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | Triage: still reproduces on main; `Status` stays `Open`. `urzua init` on a two-ADR corpus, then `urzua check`, runs `field.quality`, `field.pending`, `header.field-set-consistency` and `header.pointer-field-clean` with `eligible: 0, examined: 0`. The decision of 2026-11-04 is recorded and the fix follows as its own pull request. **Why:** confirming against main before building. | **substantive** |
> | 2026-11-04 | Fixed. `init` skips a rule whose table entry declares `reads_declaration` (`field.quality`, `field.pending`, `header.field-set-consistency`, `header.pointer-field-clean`) and reports them in a `rule-applicability` notice with the declaration each needs; `header.layout-consistency` no longer exists (`header_layout` was removed). Infers nothing (`ADR-33`). Documented in `SPEC-5` (`## Proposed rules`, `0.8`). **Why:** the maintainer decision of 2026-11-04 above. | **substantive** |
