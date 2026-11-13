---
Status: Done
Stable-Id: 01M1Y5JHCGH1AM59YY07F3YG9J
Phase: '0'
Track: roadmap-tracking
Implements: ADR-36
Blocked-on: —
Realized-by: spec:docs/adr/ADR-36-filenames-carry-the-type-prefix-going-forward.md, test:rust/crates/urzua-core/src/rules/filename.rs#filename_title_consistency_skips_a_legacy_shaped_filename@sha256:fcc933b49ab9893dec737244c2cfb1d39b1fefd85463a1e70d512f5769b3b825, code:rust/crates/urzua-core/src/rules/filename.rs@sha256:78e015d40b70c5f5aefe2ca67f4553248889bf753905d98991b7d5932686a423
---
# 10 — Decide the filename convention: type prefix, zero-padding, and whether to keep slugs

## What

Whether filenames should embed the type prefix (ADR-30-slug.md) instead of relying on directory context, whether numbers should stay zero-padded, and whether a human-readable slug belongs in the filename at all versus just the H1.

## Why

Raised directly; explicitly not decided yet -- BUG-2 already removed the actual defect (numbers already grow unbounded in both parsing and display), so this is a considered style question, not an urgent fix, and shouldn't be rushed into a corpus-wide rename.

## Done means

- `ADR-36` is Accepted and states the filename convention (the type prefix in the filename going forward, zero-padding and slugs both kept, legacy filenames never renamed and never stopping working). — evidence: spec:docs/adr/ADR-36-filenames-carry-the-type-prefix-going-forward.md
- A legacy-shaped filename is skipped rather than reported by `filename_title_consistency` (`filename_title_consistency_skips_a_legacy_shaped_filename`). — evidence: test:rust/crates/urzua-core/src/rules/filename.rs#filename_title_consistency_skips_a_legacy_shaped_filename@sha256:fcc933b49ab9893dec737244c2cfb1d39b1fefd85463a1e70d512f5769b3b825
- The helpers that rule uses no longer assume a fixed number width (the second instance of the `BUG-2` defect class). — evidence: code:rust/crates/urzua-core/src/rules/filename.rs@sha256:78e015d40b70c5f5aefe2ca67f4553248889bf753905d98991b7d5932686a423

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Decided (ADR-36): type prefix in the filename going forward, zero-padding and slugs both kept, legacy filenames never renamed and never stop working. Also caught and fixed a second independent instance of BUG-2's defect class in filename_title_consistency's own helpers. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/rules/filename.rs@sha256:10e3d229281d Reason: Pinned the files as they are now after checking each against the record it evidences: claim.rs and claim_field.rs still read a changeset's declared closes field as a claim (BUG-66), release-invariants.test.sh still plants a release-prep version bump and an unmatched level and asserts the guards fail, and checks.yml now runs it on the unattended path (BUG-92, BUG-93, BUG-96), new.rs and new_record.rs still take the configured header shape over a template (MILE-1), filename.rs still compares the filename's type prefix and padded number with the H1 (MILE-10), config.yaml still declares config.scope-matches-nothing and checks.yml runs the gate (MILE-106, MILE-110), and header.rs, narrative.rs, pointer.rs, field.rs, fix.rs, the Makefile and checks.yml still read the declared relation field names with relation_field_literals.rs failing on a restated literal (MILE-107, MILE-109). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/filename.rs` 10e3d229281d -> eb654501113d. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: filename.rs: split filename_identity_pattern into identity_findings_for_type. Behaviour unchanged, outputs byte-identical (check, check --base and fix --rebaseline JSON compared before and after). **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/filename.rs` eb654501113d -> b8eaa6832564. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: filename.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/filename.rs` b8eaa6832564 -> 78e015d40b70. Reason: Read the diff against the pin. filename.rs: expected_name strips the .md extension through record::strip_md_extension (any case of the extension) instead of strip_suffix(".md"); lowercase names behave as before. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
