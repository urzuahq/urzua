---
Stable-Id: 01M6D5VMCJKG8X6YSHC5QZE8D4
Status: Fixed
Found-in: "A /code-review pass, round 30"
Realized-by: test:rust/crates/urzua-core/src/waiver.rs#a_suppression_lacking_a_required_field_does_not_suppress_observed_failing@sha256:bffbd8916d7b06e259842924d6b9f8d9e31184c3e9794c05a1a142ed70f85526, test:rust/crates/urzua-cli/tests/suppression_type_integration.rs#a_suppression_missing_a_required_field_does_not_suppress_and_is_reported_observed_failing@sha256:a5809f8dda440e8d090bc779577c05b426fc498e321347a8eef6156bb1e70a7c
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
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/waiver.rs#a_suppression_lacking_a_required_field_does_not_suppress_observed_failing` (lines 597-628) 474b888228fa -> bffbd8916d7b, `rust/crates/urzua-cli/tests/suppression_type_integration.rs#a_suppression_missing_a_required_field_does_not_suppress_and_is_reported_observed_failing` (lines 191-233) a48c5a777e97 -> a5809f8dda44. Reason: Read the diff of each pinned span since the old pin and the claim still holds. waiver.rs#a_suppression_lacking_a_required_field_does_not_suppress_observed_failing: the test now builds the suppression kind with the template's placeholder text and a declared pending vocabulary and checks the absent, blank, template-text and pending Reason cases, and it still asserts that a suppression missing a required field is loaded and does not suppress; suppression_type_integration.rs#a_suppression_missing_a_required_field_does_not_suppress_and_is_reported_observed_failing: the test now writes the waiver type's template so the placeholder text is declared and checks absent, blank, template-text and pending Reason values, each reported as lacking Reason and not suppressing. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
