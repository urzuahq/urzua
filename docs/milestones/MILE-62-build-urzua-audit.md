---
Status: Done
Stable-Id: 01M1YNNCJQKWVEVQWPQ3ZCHJMV
Phase: '0'
Track: governance-process
Implements: ADR-30
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#audit_honours_the_declared_level_of_a_rule_instead_of_running_it_unconditionally@sha256:60fd2f4ae8378d956ed2d8a41ae62bfb70201e8a496c2d508bf0c8b66956f217, test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#audit_is_check_over_the_configured_rules_plus_one_deprecation_notice@sha256:e190a2681bbc21edee5066f08c4a3ebc6e1d33fb3e089fa68b551839c37b4dc5, code:rust/crates/urzua-core/src/rules/pointer.rs, code:rust/crates/urzua-cli/src/commands/audit.rs, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-b/expectations/audit-reports-one-sided-supersession.yaml, spec:docs/specs/SPEC-11-urzua-audit.md
---
# 62 — Build urzua audit

## What

`urzua audit`: cross-record reconciliation -- supersession reciprocity and dangling cross-references -- reusing `check`'s own rule functions rather than a second implementation. Read-only; never writes.

## Why

A bulk cross-reference rewrite is a real data-loss risk without a review step, so `audit` reports what's wrong across records without ever attempting to fix it itself.

## Done means

- `urzua audit` reports supersession reciprocity across records. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#audit_honours_the_declared_level_of_a_rule_instead_of_running_it_unconditionally@sha256:60fd2f4ae8378d956ed2d8a41ae62bfb70201e8a496c2d508bf0c8b66956f217
- `urzua audit` reports dangling cross-references across records. — evidence: test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#audit_is_check_over_the_configured_rules_plus_one_deprecation_notice@sha256:e190a2681bbc21edee5066f08c4a3ebc6e1d33fb3e089fa68b551839c37b4dc5, code:rust/crates/urzua-core/src/rules/pointer.rs
- It does so by calling `check`'s own rule functions rather than a second implementation. — evidence: code:rust/crates/urzua-cli/src/commands/audit.rs, test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#audit_is_check_over_the_configured_rules_plus_one_deprecation_notice@sha256:e190a2681bbc21edee5066f08c4a3ebc6e1d33fb3e089fa68b551839c37b4dc5
- `urzua audit` is read-only and writes nothing. — evidence: code:rust/crates/urzua-cli/src/commands/audit.rs
- The acceptance suite runs it (`SPEC-11`). — evidence: test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-b/expectations/audit-reports-one-sided-supersession.yaml, spec:docs/specs/SPEC-11-urzua-audit.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
