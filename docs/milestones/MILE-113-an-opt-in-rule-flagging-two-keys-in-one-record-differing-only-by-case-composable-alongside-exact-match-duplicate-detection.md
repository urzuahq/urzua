---
Stable-Id: 01M36WECY7ZHHCHDCCZTJQ1TAT
Status: Done
Phase: '1'
Track: header-format
Implements: ADR-57
Realized-by: code:rust/crates/urzua-core/src/rules/header.rs#header_case_variant_key@sha256:798898160c39c1a12b1c77eb4662f1a2b057e85052aafec7bc40599e9e9c5941, test:rust/crates/urzua-cli/tests/case_variant_key_integration.rs#a_declared_rule_reports_the_case_variant_and_what_it_examined_observed_failing@sha256:12d7f5e5a690c8a244ddc684c39d1190a63c73e673f7846f0a5243a6fa5012ed, test:rust/crates/urzua-core/src/rules/header.rs#three_spellings_of_one_key_are_one_finding_naming_all_three@sha256:43bf70c14007ef6fe2e1a50d91a4dd9092084af13d4496172c50f71e1d33fbdd, test:rust/crates/urzua-cli/tests/case_variant_key_integration.rs#it_composes_beside_the_exact_match_rules_without_changing_them@sha256:6bb244b603edb3d77107abe3e5ad77c46c96bcfcda40892b56e99a4223384f2c, code:rust/crates/urzua-core/src/rules/catalog.rs@sha256:fa34978bc2fb1c19d46351b26b0d980ae6624d7c2bad28dd271fef4d482d2550, spec:docs/specs/SPEC-2-urzua-check.md
---
# 113 — an opt-in rule flagging two keys in one record differing only by case, composable alongside exact-match duplicate detection

## What

A new, opt-in rule (working name `header.case-variant-key`) that reports when one record's own
header writes two keys differing only by case (`Status`/`status`) -- a distinct signal from
`duplicate_keys()`'s exact-match repetition check, not a replacement for it. `ADR-57`'s exact-match
principle stays exactly as decided: this rule never asserts which spelling is "right," never treats
the two keys as one field, and never affects `duplicate_keys()`, `header.field-set-consistency`, or
any other exact-match comparison. It only surfaces the fact of the case-only difference as its own,
separately-enable-able finding, for a repository that wants that extra scrutiny.

## Why

Came out of investigating `BUG-130`: a real, live gap exists (a type declaring no vocabulary has
nothing that flags a `Status`/`status` pair in one header as suspicious), but the fix isn't to make
`duplicate_keys()` case-insensitive -- `ADR-57` already, explicitly, decided that comparison stays
exact, for the same reason it stays exact everywhere else (measured: zero legitimate case-only field
pairs in this corpus's own history; `Maße`/`Masse` is exactly the shape a loose comparison gets wrong
in the other direction). The right shape is a new, additive, opt-in signal composed alongside the
existing exact-match rules, not a change to any of them -- the same pattern `header.field-case-
mismatch` (`RFC-40`/`ADR-58`) already uses for a declared-field case mismatch, generalized to a
same-record, no-declaration-required case.

## Done means

- The opt-in rule `header.case-variant-key` reports a record whose own header writes two keys differing only by case (Unicode `to_lowercase` equality), once per group of spellings. — evidence: code:rust/crates/urzua-core/src/rules/header.rs#header_case_variant_key@sha256:798898160c39c1a12b1c77eb4662f1a2b057e85052aafec7bc40599e9e9c5941, test:rust/crates/urzua-cli/tests/case_variant_key_integration.rs#a_declared_rule_reports_the_case_variant_and_what_it_examined_observed_failing@sha256:12d7f5e5a690c8a244ddc684c39d1190a63c73e673f7846f0a5243a6fa5012ed, test:rust/crates/urzua-core/src/rules/header.rs#three_spellings_of_one_key_are_one_finding_naming_all_three@sha256:43bf70c14007ef6fe2e1a50d91a4dd9092084af13d4496172c50f71e1d33fbdd
- It runs beside `duplicate_keys()`'s exact-match check, which is unchanged. — evidence: test:rust/crates/urzua-cli/tests/case_variant_key_integration.rs#it_composes_beside_the_exact_match_rules_without_changing_them@sha256:6bb244b603edb3d77107abe3e5ad77c46c96bcfcda40892b56e99a4223384f2c
- It takes no options and is registered in `rules/catalog.rs` and `SPEC-2`'s generated rule table. — evidence: code:rust/crates/urzua-core/src/rules/catalog.rs@sha256:fa34978bc2fb1c19d46351b26b0d980ae6624d7c2bad28dd271fef4d482d2550, code:rust/crates/urzua-core/src/rules/header.rs#header_case_variant_key@sha256:798898160c39c1a12b1c77eb4662f1a2b057e85052aafec7bc40599e9e9c5941, spec:docs/specs/SPEC-2-urzua-check.md
- `case_variant_key_integration.rs` covers a case-variant record and a clean one. — evidence: test:rust/crates/urzua-cli/tests/case_variant_key_integration.rs#a_declared_rule_reports_the_case_variant_and_what_it_examined_observed_failing@sha256:12d7f5e5a690c8a244ddc684c39d1190a63c73e673f7846f0a5243a6fa5012ed

## References

- `BUG-130` -- the investigation that surfaced this, corrected to `Not a bug` rather than folded
  into this milestone's own scope.
- `ADR-57` -- the exact-match principle this milestone does not reopen or weaken.
- `header.field-case-mismatch` (`RFC-40`/`ADR-58`) -- the closest existing precedent for a
  case-focused, additive signal that doesn't touch exact-match verdicts.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** the user proposed this as a genuinely separate, composable rule while working through `BUG-130` with me -- filed as its own milestone rather than bundled into that bug's correction, since it's new work, not a fix to a misdiagnosed one. | **substantive** |
> | 2026-11-03 | Built: `header.case-variant-key` ships in `rules/header.rs`, registered in `rules/catalog.rs` and `check`, with `SPEC-2`'s generated rule table updated. Narrowest readings taken: the rule takes no options; "differing only by case" is Unicode `to_lowercase` equality over the keys a record wrote (so `CAFÉ`/`café` group, `Maße`/`Masse` do not); one finding per group of distinct spellings, never for an exact repeat of one spelling (that stays `header.required-fields`'s); the population is every loaded record (`header_shape: none` out of scope, unreadable headers unreadable). **Why:** this milestone's design left these open and `ADR-57` allows only an additive, non-merging signal. `Status` is untouched. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #157, #117: opt-in rule `header.case-variant-key` reports two keys differing only by case, beside the exact-match duplicate check. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/rules/catalog.rs@sha256:dff97ad9a0a8 Reason: Pinned the files as they are now after checking each against the record it evidences: report.rs still discloses every non-absent exclusion in a rule's population (MILE-111), catalog.rs still registers header.case-variant-key (MILE-113), export.rs still emits each record as plain JSON and generate-dashboard.py still reads it (MILE-115), the Makefile and checks.yml still run the zero-warning gate (MILE-116), AGENTS.md and config.rs still state and read the declared-adopter premise (MILE-118), diff_scope.rs still selects the records a --base run examines (MILE-123), boundary.rs, back_pointer.rs and the corpus-h expectation still implement and prove the declared published-path boundary (MILE-125), the claim_expiry.rs, reverify.rs and commands/reverify.rs files and the changeset still implement ADR-97 beside embodiment.rs, discovery.rs and fix.rs (MILE-129), discovery.rs, new.rs and the corpus-i files still give one identity namespace across directories (MILE-130), and the rules.rs module index and the per-family rule modules still exist as the split MILE-131 describes. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/header.rs#header_case_variant_key` (lines 306-380) 4a282671e6e5 -> 798898160c39. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: header.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/catalog.rs` dff97ad9a0a8 -> fa34978bc2fb. Reason: Read the diff of each pinned file since the previous pin. catalog.rs gained the RULE_SECTION_DUPLICATE, RULE_SECTION_LEVEL_SKIPPED and RULE_SECTION_ORDER id constants; no existing constant changed; the claim holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
