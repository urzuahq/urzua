---
Stable-Id: 01M6D5VMCJKG8X6YSHC5QZE8D4
Status: Fixed
Found-in: "A /code-review pass, round 30"
Realized-by: test:rust/crates/urzua-core/src/waiver.rs#a_suppression_lacking_a_required_field_does_not_suppress_observed_failing@sha256:474b888228fa0bf38fb3d215d760fd5f2c430325bf3f30e51c2f307d66fc802a, test:rust/crates/urzua-cli/tests/suppression_type_integration.rs#a_suppression_missing_a_required_field_does_not_suppress_and_is_reported_observed_failing@sha256:a48c5a777e97ef1b27861938654b7d8c80670f8f06802881faa5603c8febe39a
---
# 177 — a waiver with no Reason still waives because load_waivers requires only Rule and Scope

## What was wrong

`load_waivers` skips a waiver only when `Rule` or `Scope` is missing. A waiver with no `Reason` still waives;
the omission is visible only when `header.required-fields` is enabled for the waiver type. `SPEC-10` lists
`Reason` as "free text, required", "the whole reviewability point of not using an ignore list".

## Why nothing caught it

`SPEC-10` describes the skip rule for `Rule`/`Scope` only; `Reason` is enforced only through the generic
required-fields rule, which is opt-in.

## Why this needs a decision, not a mechanical fix

Needs a decision: refuse to honor a reasonless waiver (fails toward more findings, like a missing `Rule`),
or keep enforcement in `header.required-fields` and say so in `SPEC-10`. Note, checked and not filed:
`Scope` matching exactly (so `./x`, a directory or a glob waives nothing) is explicit in `SPEC-10`'s
Matching semantics, and a typo'd `Rule` is listed there under "not built".

## References

`SPEC-10`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed, left `Status: Open`. **Why:** found by a round 30 review and reproduced against a disposable fixture. | **substantive** |
> | 2026-11-04 | Triage: still reproduces on main; `Status` stays `Open`. A waiver record with `Rule` and `Scope` and no `Reason` still waives: `urzua check` marks the targeted `embodiment.locator-exists` finding `waived` by it. **Why:** refuse a reasonless waiver, or keep enforcement in `header.required-fields` and say so in `SPEC-10`, is the decision the record names. | **substantive** |
> | 2026-11-05 | Fixed, opt-in per the maintainer decision of 2026-11-05: the suppression type declares `effective_requires: [Reason]` (`SPEC-3`, `SPEC-10`); a waiver lacking a listed field, or holding a blank, placeholder or pending value, suppresses nothing, its target finding is reported again, and `waiver.matches-nothing` reports it as `waiver-ineffective` (`SPEC-2`, with its `edit-field` `set` action). The default keeps today's behaviour; this repository requires `Reason`. **Why:** the decision this record named, refuse a reasonless waiver or leave enforcement to `header.required-fields`, was taken as an opt-in declaration. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
