---
Stable-Id: 01M6HZ0MAXAFHSYZSBZ4D6KBAP
Status: Done
Phase: '0'
Track: schema-governance
Implements: RFC-33
Realized-by: code:rust/crates/urzua-core/src/rules/dates.rs#date_field_valid@sha256:6281e19f318b67fe1db9b1f28e20ede438f3f63f5c5c69b31d915970c9782182, test:rust/crates/urzua-core/src/rules/dates.rs#a_malformed_date_is_reported_observed_failing@sha256:c6df325a5c59883aef2ee0d89f7a756d42a96290ea4258465c53f64a021e4100, test:rust/crates/urzua-core/src/rules/dates.rs#an_impossible_calendar_date_is_reported_and_leap_days_pass@sha256:149cd96df35af78c8172e1c06f2d014a795174c0e0f6463b11dfd513b91c0c8b, test:rust/crates/urzua-cli/tests/check_integration.rs#the_acceptance_date_by_status_is_expressed_with_statuses_require_forbid_and_date_field_valid@sha256:7960528192b7e754ae83eea287713de7fe2e248804aa5650f2c553bc348ef22c, test:rust/crates/urzua-cli/tests/check_integration.rs#declared_statuses_are_enforced_end_to_end@sha256:a907e0b1e48e96373c41f5ff7abfe722c66924e2d53731b825d18d35f491deb2, code:rust/crates/urzua-core/src/rules/status.rs#status_forbidden_field_present@sha256:95bfdccfc441b0166ae7bdecfeb8146e5fee70caa0a341a55766eb76ddbe95d1, test:rust/crates/urzua-core/src/rules/dates.rs#the_rule_counts_the_date_values_it_examined@sha256:79e1f5f748f216b98cde6cd8b23e89c0afc8e290f09c64975511bae5e9a6546b
---
# 137 — Validate date fields and require the acceptance date exactly on accepted-type statuses

## What

Date-valued fields are validated as dates: well-formed (`YYYY-MM-DD`) and a real calendar date (no `2026-02-30`). A type also declares which statuses are accepted-type; the acceptance date field is required on those and forbidden (other than as a placeholder) on every other status. All violations are errors. The status-dependent requirement is expressed through the status-dependent required and forbidden fields in `MILE-132`; this milestone owns the date shape and the calendar check.

## Why

urzua checks that a date field is present but never that it is a date, so `2026-13-45` or free text passes. The adopter's linter requires its record date to be well-formed, requires its acceptance date to be a real calendar date, requires that date exactly when the record is accepted, superseded or deprecated, and rejects it on a record that is not. A replacement without these lets an unaccepted record carry an acceptance date, which reads as a decision nobody made.

## Done means

- A date field declared as a date type rejects a malformed value and an impossible calendar date, each with an error. — evidence: code:rust/crates/urzua-core/src/rules/dates.rs#date_field_valid@sha256:6281e19f318b67fe1db9b1f28e20ede438f3f63f5c5c69b31d915970c9782182, test:rust/crates/urzua-core/src/rules/dates.rs#a_malformed_date_is_reported_observed_failing@sha256:c6df325a5c59883aef2ee0d89f7a756d42a96290ea4258465c53f64a021e4100, test:rust/crates/urzua-core/src/rules/dates.rs#an_impossible_calendar_date_is_reported_and_leap_days_pass@sha256:149cd96df35af78c8172e1c06f2d014a795174c0e0f6463b11dfd513b91c0c8b
- An accepted-type record without the acceptance date, and any other record carrying one, each produce an error. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#the_acceptance_date_by_status_is_expressed_with_statuses_require_forbid_and_date_field_valid@sha256:7960528192b7e754ae83eea287713de7fe2e248804aa5650f2c553bc348ef22c, test:rust/crates/urzua-cli/tests/check_integration.rs#declared_statuses_are_enforced_end_to_end@sha256:a907e0b1e48e96373c41f5ff7abfe722c66924e2d53731b825d18d35f491deb2
- A blank or declared placeholder acceptance date is tolerated on the forbidden side. — evidence: code:rust/crates/urzua-core/src/rules/status.rs#status_forbidden_field_present@sha256:95bfdccfc441b0166ae7bdecfeb8146e5fee70caa0a341a55766eb76ddbe95d1
- The rule reports how many date values it examined. — evidence: test:rust/crates/urzua-core/src/rules/dates.rs#the_rule_counts_the_date_values_it_examined@sha256:79e1f5f748f216b98cde6cd8b23e89c0afc8e290f09c64975511bae5e9a6546b

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter validates date shape, calendar validity and the acceptance date's presence and absence by status; urzua validates none of it. The check that the acceptance date is not earlier than the record date is consciously not carried over: the adopter itself treats it as usually legitimate. | **substantive** |
> | 2026-11-04 | Built as two opt-in rules, `date.field-valid` (`date_fields`) and `date.acceptance-by-status` (`acceptance_field`; accepted-type statuses from the declared `statuses.success` or an `accepted_statuses` option). **Why:** the `fields` declaration names no field types, so the narrowest reading puts the date and acceptance-field names in rule options; the acceptance-date-not-earlier check stays out, as filed. | **substantive** |
> | 2026-11-04 | Dropped `date.acceptance-by-status` and its `acceptance_field` option before release; the acceptance date by status is the `statuses.require` / `statuses.forbid` declaration of `MILE-132` plus `date.field-valid`, documented in `SPEC-3` and proven by an integration test. `accepted_statuses` stays with `embodiment.back-pointer-decision-accepted`. **Why:** maintainer decision of 2026-11-04: one mechanism, not two that could disagree. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #183, #179: `date.field-valid` plus the acceptance date by status through `statuses.require` and `statuses.forbid`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/dates.rs#date_field_valid` (lines 48-102) 434f99299a48 -> 6281e19f318b. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: dates.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
