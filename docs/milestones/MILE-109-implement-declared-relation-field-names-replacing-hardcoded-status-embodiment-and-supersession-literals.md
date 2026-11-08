---
Stable-Id: 01M35QF98A6W7VYP7JHR3KTCDZ
Status: Done
Phase: '1'
Track: schema-governance
Implements: RFC-42
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/config.rs#RelationFields@sha256:80f755ae4036b0c9f2ae8f3719de30237a80f691f059378eb22ba1a8d85cec84, code:rust/crates/urzua-core/src/rules/embodiment.rs#relation_field_name@sha256:1fad8037a10bc178cd7f95acd3d41fb9ab7fd598abcc82a822828c4edbf3fcec, code:rust/crates/urzua-core/src/rules/pointer.rs@sha256:e4d8db46f2e32fd5b80d2a62fe9938bf7ef057e18de0e92d2f22c66987b057d9, code:rust/crates/urzua-core/src/rules/narrative.rs@sha256:1b08f9fad547e6d6c09ae233eaec56f5172fe65509859b68b2951e65bdf0dd35, code:rust/crates/urzua-core/src/fix.rs@sha256:aff4037c8a9a41fb3de7a110e28a3534ddd2f41e7568a7326b382f6aba59128a, test:rust/crates/urzua-cli/tests/relation_field_literals.rs#no_relation_field_literal_escapes_config_rs@sha256:ecbcb243a3aa835749ec347d6fe2de73f75b40c2b00f95c31cb18e3cf1c44856, test:rust/crates/urzua-core/src/rules/embodiment.rs#a_type_declaring_custom_embodiment_field_names_is_read_by_those_names_observed_failing@sha256:986cd228ed1bebab023b1166a234cd4fbefaccd0a92980fcc5d13d1445582778, test:rust/crates/urzua-core/src/rules/pointer.rs#a_type_declaring_a_custom_status_field_name_is_read_by_that_name_observed_failing@sha256:6d06c500ec1f6e147218f3b24423adc38291c17f26e4feb2b37eace2723f0505, test:rust/crates/urzua-core/src/rules/narrative.rs#narrative_field_stale_reads_a_custom_status_field_name_observed_failing@sha256:473d192beed6c03642571c3fc5486c501e88d2f364b3c4d2782298843df1bc15
---
# 109 — Implement declared relation field names replacing hardcoded status embodiment and supersession literals

## What

`RFC-42`/`ADR-61`'s implementation: a per-type `relation_fields` config map (`status`,
`embodiment_state`, `embodiment_locator`, `supersession`), each defaulting to its pre-`RFC-42` literal
(`Status`, `Embodiment`, `Realized-by`, `Supersedes / Superseded-by`). Seven call sites across
`claim_status_agreement`, `pointer_target_status`, `narrative_field_stale`, `embodiment_consistency`,
`embodiment_locator_exists`, `embodiment_locator_promotion_candidate`, and `supersession_reciprocity`
resolve their field name through `RecordTypeConfig::relation_field(role)` instead of a literal. A new
rule, `config.relation-field-not-known`, reports a declared override not also present in that type's
`required_fields`/`known_fields`.

## Why

`BUG-110` found `Status` hardcoded in three rules; a round-18 code review found the identical shape
recurring in four more rules for `Embodiment`, `Realized-by`, and `Supersedes / Superseded-by`. Seven
rules, one defect shape, so `RFC-42` consolidated them into one config mechanism rather than four
separate `status_field`-shaped patches, matching the precedent `MILE-90` already set for
`pointer_fields`/`narrative_fields`.

## Verification

Full test suite (219 tests, 10 new), `cargo fmt`, `clippy -D warnings`, `make ci`, and a real-corpus
`check` run reporting the same 70 findings before and after — this repository's own config declares no
`relation_fields` override, so the change is a no-op on it by design. Each of the four resolved roles
(`status`, `embodiment_state`+`embodiment_locator`, `supersession`) has its own planted-violation test,
observed failing against the pre-fix literal and passing after, plus a CodeRabbit-review-prompted fix
making every finding message report the resolved field name instead of the pre-`RFC-42` literal.

## Done means

- A per-type `relation_fields` map declares the `status`, `embodiment_state`, `embodiment_locator` and `supersession` field names, each defaulting to its pre-`RFC-42` literal (`Status`, `Embodiment`, `Realized-by`, `Supersedes / Superseded-by`). — evidence: code:rust/crates/urzua-core/src/config.rs#RelationFields@sha256:80f755ae4036b0c9f2ae8f3719de30237a80f691f059378eb22ba1a8d85cec84
- The seven call sites that hardcoded those names read the map. — evidence: code:rust/crates/urzua-core/src/rules/embodiment.rs#relation_field_name@sha256:1fad8037a10bc178cd7f95acd3d41fb9ab7fd598abcc82a822828c4edbf3fcec, code:rust/crates/urzua-core/src/rules/pointer.rs@sha256:e4d8db46f2e32fd5b80d2a62fe9938bf7ef057e18de0e92d2f22c66987b057d9, code:rust/crates/urzua-core/src/rules/narrative.rs@sha256:1b08f9fad547e6d6c09ae233eaec56f5172fe65509859b68b2951e65bdf0dd35, code:rust/crates/urzua-core/src/fix.rs@sha256:aff4037c8a9a41fb3de7a110e28a3534ddd2f41e7568a7326b382f6aba59128a
- `relation_field_literals.rs` fails when a rule restates one. — evidence: test:rust/crates/urzua-cli/tests/relation_field_literals.rs#no_relation_field_literal_escapes_config_rs@sha256:ecbcb243a3aa835749ec347d6fe2de73f75b40c2b00f95c31cb18e3cf1c44856
- A fixture with renamed fields passes the same rules. — evidence: test:rust/crates/urzua-core/src/rules/embodiment.rs#a_type_declaring_custom_embodiment_field_names_is_read_by_those_names_observed_failing@sha256:986cd228ed1bebab023b1166a234cd4fbefaccd0a92980fcc5d13d1445582778, test:rust/crates/urzua-core/src/rules/pointer.rs#a_type_declaring_a_custom_status_field_name_is_read_by_that_name_observed_failing@sha256:6d06c500ec1f6e147218f3b24423adc38291c17f26e4feb2b37eace2723f0505, test:rust/crates/urzua-core/src/rules/narrative.rs#narrative_field_stale_reads_a_custom_status_field_name_observed_failing@sha256:473d192beed6c03642571c3fc5486c501e88d2f364b3c4d2782298843df1bc15

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and implemented in one round. **Why:** `RFC-42`/`ADR-61` decided the design the same day a round-18 review surfaced the recurring defect shape; scheduled and built together so the gap didn't sit open for a later release. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/config.rs#RelationFields` (lines 1370-1386) 6d687d622ad1 -> 80f755ae4036. Reason: RelationFields gained an optional title field, the field urzua new fills with the record's title (MILE-74). The status, embodiment, supersession, date and author fields the record cites are unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/rules/pointer.rs@sha256:e4d8db46f2e3, rust/crates/urzua-core/src/rules/narrative.rs@sha256:1b08f9fad547, rust/crates/urzua-core/src/fix.rs@sha256:c9cd87e7b00e Reason: Pinned the files as they are now after checking each against the record it evidences: claim.rs and claim_field.rs still read a changeset's declared closes field as a claim (BUG-66), release-invariants.test.sh still plants a release-prep version bump and an unmatched level and asserts the guards fail, and checks.yml now runs it on the unattended path (BUG-92, BUG-93, BUG-96), new.rs and new_record.rs still take the configured header shape over a template (MILE-1), filename.rs still compares the filename's type prefix and padded number with the H1 (MILE-10), config.yaml still declares config.scope-matches-nothing and checks.yml runs the gate (MILE-106, MILE-110), and header.rs, narrative.rs, pointer.rs, field.rs, fix.rs, the Makefile and checks.yml still read the declared relation field names with relation_field_literals.rs failing on a restated literal (MILE-107, MILE-109). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` c9cd87e7b00e -> 88c815c0639a. Reason: Read the diff of each pinned file: core fix.rs moved apply_rebaseline's evidence-line lookup and revision-row tail into shared helpers (evidence_edit_lines, append_log_rows, revision_log_refusal) that apply_reverify also calls, with the same rows and error text; cli commands/fix.rs gained validate_reason, back_pointer_evidence, into_failures and identity_notices extracted from fix and reverify with the same behaviour; core reverify.rs calls the helpers and its first-pin matcher no longer splices into a longer anchored locator; cli commands/reverify.rs now refuses a record with no usable revision log in detect, refuses any non-human actor at an error-level period, and honours --ids under --force. Each claim these records make about those files still holds. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` 88c815c0639a -> aff4037c8a9a. Reason: Read the diff of each pinned file since the old pin; the claim still holds. The files changed in this PR only to read the declared Done means section name in messages, to confine locator probes to the repository, to excuse a waived item from locator-exists only while its waiver is unexpired, and to carry the section name to reverify messages; the behaviour each record relies on is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
