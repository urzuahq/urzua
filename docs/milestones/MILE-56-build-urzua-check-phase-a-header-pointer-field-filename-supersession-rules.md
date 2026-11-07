---
Status: Done
Stable-Id: 01M1YNN8V8XTS6CSCEEBAEWR9A
Phase: '0'
Track: section-checks
Implements: ADR-23, ADR-8, ADR-9, ADR-14
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/header.rs, code:rust/crates/urzua-core/src/rules/pointer.rs, code:rust/crates/urzua-core/src/rules/field.rs, code:rust/crates/urzua-core/src/rules/filename.rs, code:rust/crates/urzua-core/src/rules/relation.rs, test:rust/crates/urzua-cli/tests/check_integration.rs#check_exits_0_on_a_clean_corpus@sha256:585cb12f3f55fb534592e1802d0448163e7aa3fa06e5a91709bc32ab641fecb9, test:rust/crates/urzua-cli/tests/check_integration.rs#check_exits_1_on_a_missing_required_field@sha256:95b88e795e9a4ffb4fe1983f76bf57a3a3191dfc5c579f4ff207381a3ff0c307, test:rust/crates/urzua-core/src/rules/pointer.rs#a_dangling_parent_pointer_is_an_error@sha256:267187b5e96ba050bf1de1c788b2b95d9de812325e478a6494595fe66bb28b9e, test:rust/crates/urzua-core/src/rules/field.rs#field_quality_flags_placeholder_not_just_absence@sha256:b8f81e22b74605217ded13f751b0892ebeb4aa90fdcde98d1fcfef8ba6b3dd2d, test:rust/crates/urzua-core/src/rules/filename.rs#filename_title_mismatch_is_a_planted_violation_observed_failing@sha256:77d6f0567c5c1172f3702657099a5a05f2ebe6b6cc03f5ea9a92d609448a157b, test:rust/crates/urzua-cli/tests/check_integration.rs#audit_exits_1_and_reports_a_one_directional_supersession_claim_observed_failing@sha256:33724f9c7b6da2a5d560336d17bcdae7a3f558ee2b06e409b6addaf9613b6a6f, code:.urzua/config.yaml, code:.github/workflows/checks.yml, spec:docs/adr/ADR-23-stdout-is-always-json-no-format-flag.md
---
# 56 — Build urzua check Phase A: header, pointer, field, filename, supersession rules

## What

The initial `check` rule set: `header.required-fields`, `pointer.resolution`, `field.quality`, `filename.title-consistency`, `relation.supersession-reciprocity` -- the substrate every other command builds on.

## Why

SPEC-2's own Phase A: a validator whose first run is green has told you nothing about itself, so both seed rules were chosen because they fire on this corpus's real, pre-existing defects.

## Done means

- `urzua check` runs the Phase A rule set (`header.required-fields`, `pointer.resolution`, `field.quality`, `filename.title-consistency`, `relation.supersession-reciprocity`) over a corpus. — evidence: code:rust/crates/urzua-core/src/rules/header.rs, code:rust/crates/urzua-core/src/rules/pointer.rs, code:rust/crates/urzua-core/src/rules/field.rs, code:rust/crates/urzua-core/src/rules/filename.rs, code:rust/crates/urzua-core/src/rules/relation.rs, test:rust/crates/urzua-cli/tests/check_integration.rs#check_exits_0_on_a_clean_corpus@sha256:585cb12f3f55fb534592e1802d0448163e7aa3fa06e5a91709bc32ab641fecb9
- `header.required-fields` fires on a planted defect. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#check_exits_1_on_a_missing_required_field@sha256:95b88e795e9a4ffb4fe1983f76bf57a3a3191dfc5c579f4ff207381a3ff0c307
- `pointer.resolution` fires on a planted defect. — evidence: test:rust/crates/urzua-core/src/rules/pointer.rs#a_dangling_parent_pointer_is_an_error@sha256:267187b5e96ba050bf1de1c788b2b95d9de812325e478a6494595fe66bb28b9e
- `field.quality` fires on a planted defect. — evidence: test:rust/crates/urzua-core/src/rules/field.rs#field_quality_flags_placeholder_not_just_absence@sha256:b8f81e22b74605217ded13f751b0892ebeb4aa90fdcde98d1fcfef8ba6b3dd2d
- `filename.title-consistency` fires on a planted defect. — evidence: test:rust/crates/urzua-core/src/rules/filename.rs#filename_title_mismatch_is_a_planted_violation_observed_failing@sha256:77d6f0567c5c1172f3702657099a5a05f2ebe6b6cc03f5ea9a92d609448a157b
- `relation.supersession-reciprocity` fires on a planted defect. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#audit_exits_1_and_reports_a_one_directional_supersession_claim_observed_failing@sha256:33724f9c7b6da2a5d560336d17bcdae7a3f558ee2b06e409b6addaf9613b6a6f
- Over this repository's own records the rules run and examine them. — evidence: code:.urzua/config.yaml, code:.github/workflows/checks.yml
- `Implements` points at the current `ADR-23`. — evidence: spec:docs/adr/ADR-23-stdout-is-always-json-no-format-flag.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Implements: ADR-7` changed to `ADR-23` (`ADR-23` superseded `ADR-7`). **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
