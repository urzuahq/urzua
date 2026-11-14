---
Stable-Id: 01M6CXZYE7Q8TBZQ00SXP63SKQ
Status: Done
Phase: '0'
Track: embodiment-model
Realized-by: code:rust/crates/urzua-core/src/rules/back_pointer.rs#back_pointer_evidence@sha256:3004719e176df34bdd8f1c392982fcd352242ccaf5a142a1e69f4ff373ccb0fe, test:rust/crates/urzua-cli/tests/back_pointer_integration.rs#a_cited_record_computes_to_implemented_and_a_dangling_citation_is_reported@sha256:50bfe5b05c503a4f4fb643ef7707da6236b73f92a0d69d6b5f761245e7be6b5e, test:rust/crates/urzua-cli/tests/back_pointer_integration.rs#fix_detect_agrees_with_check_about_a_cited_record@sha256:751e68d7240f406dd9179fbf0021a140e01db1130c32f7dffe0d91f747cb4b94, test:rust/crates/urzua-cli/tests/back_pointer_integration.rs#without_the_rule_the_comment_is_not_evidence@sha256:80a04a4c437165fe8b5c435084705bf905e12b00e38e4ea686fa3cc4e12c50ed, code:rust/crates/urzua-core/src/rules/back_pointer.rs#embodiment_back_pointer_resolution@sha256:3925330147be2f2e23da1983595610b53aa6789dba37b1a7643a87bf5153a5df, spec:docs/adr/ADR-66-a-source-comment-citing-a-decision-counts-as-realization-evidence.md
---
# 128 — Decide whether a source-code comment back-pointer counts as realization evidence

## What

Decide whether a back-pointer comment in source code (a comment naming the governing record) counts as
realization evidence for the record, in the sense `Realized-by` locators are evidence today.
`RFC-5` section 2 ("The pointer lives on the record, not embedded in application code") chose
record-side pointers and says comment-in-code is not the strongest available signal; `RFC-28`
("Why the record side alone is not enough") restates that `RFC-5` section 2 rejected in-code
`Implements:` comments as a way to discover what governs a file, and says it does not reopen that.

## Why

A real adopter relies on such comments at scale, across several hundred records, so the evidence model
either has a place for them or the adopter keeps a parallel mechanism and the replacement bar
(`MILE-102`) is not met. The prior rejections were about discovery and strength of signal, not about
whether a comment may be accepted as a claim the record already makes; that distinction is the decision
to make. If decided in favour, it needs its own ADR that narrows `RFC-5` section 2 rather than editing
it.

## Decision (2026-11-04)

A code comment citing a decision (`Implements: ADR-N`) counts as realization evidence, and a comment citing a record that does not exist is reported.

## Done means

- A whole-line `Implements: <id>[, <id>]` comment in a file under a declared `back_pointer_paths` entry is code-tier evidence in `embodiment.consistency` and `urzua fix`. — evidence: code:rust/crates/urzua-core/src/rules/back_pointer.rs#back_pointer_evidence@sha256:3004719e176df34bdd8f1c392982fcd352242ccaf5a142a1e69f4ff373ccb0fe, test:rust/crates/urzua-cli/tests/back_pointer_integration.rs#a_cited_record_computes_to_implemented_and_a_dangling_citation_is_reported@sha256:50bfe5b05c503a4f4fb643ef7707da6236b73f92a0d69d6b5f761245e7be6b5e, test:rust/crates/urzua-cli/tests/back_pointer_integration.rs#fix_detect_agrees_with_check_about_a_cited_record@sha256:751e68d7240f406dd9179fbf0021a140e01db1130c32f7dffe0d91f747cb4b94, test:rust/crates/urzua-cli/tests/back_pointer_integration.rs#without_the_rule_the_comment_is_not_evidence@sha256:80a04a4c437165fe8b5c435084705bf905e12b00e38e4ea686fa3cc4e12c50ed
- The opt-in rule `embodiment.back-pointer-resolution` reports a back-pointer citing no record. — evidence: code:rust/crates/urzua-core/src/rules/back_pointer.rs#embodiment_back_pointer_resolution@sha256:3925330147be2f2e23da1983595610b53aa6789dba37b1a7643a87bf5153a5df, test:rust/crates/urzua-cli/tests/back_pointer_integration.rs#a_cited_record_computes_to_implemented_and_a_dangling_citation_is_reported@sha256:50bfe5b05c503a4f4fb643ef7707da6236b73f92a0d69d6b5f761245e7be6b5e
- The decision is recorded in `ADR-66`, which is Accepted. — evidence: spec:docs/adr/ADR-66-a-source-comment-citing-a-decision-counts-as-realization-evidence.md
- `back_pointer_integration.rs` covers a resolving and a dangling back-pointer. — evidence: test:rust/crates/urzua-cli/tests/back_pointer_integration.rs#a_cited_record_computes_to_implemented_and_a_dangling_citation_is_reported@sha256:50bfe5b05c503a4f4fb643ef7707da6236b73f92a0d69d6b5f761245e7be6b5e

## References

- `RFC-5` -- section 2: the pointer lives on the record.
- `RFC-28` -- restates the section 2 decision and does not reopen it.
- `MILE-102` -- the adopter replacement this serves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** Two accepted-or-drafted records reject in-code comments as the evidence model while a first adopter uses them at scale; filed as a decision. | **substantive** |
> | 2026-11-04 | Decision recorded: a code comment citing a decision (`Implements: ADR-N`) counts as realization evidence, and a comment citing a record that does not exist is reported. `Status` stays `Planned`. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | Implemented as opt-in rule `embodiment.back-pointer-resolution`: a whole-line `Implements: <id>[, <id>]` comment under a `back_pointer_paths` entry is code-tier evidence in `embodiment.consistency` and `urzua fix`, and a back-pointer citing no record is reported. Recorded in `ADR-66` (Proposed). `Status` stays `Planned`. **Why:** the 2026-11-04 decision; narrowed to a fixed marker, whole-line comments and code tier, with the wider forms left as open questions in the ADR. | **substantive** |
> | 2026-11-04 | Reads the rule's description and option as part of the `SPEC-2` rule table. **Why:** the rule table is generated from the rule catalog, so the description is the rule's one statement of the narrowest reading. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #170: the decision is recorded in the accepted `ADR-66` and built as `embodiment.back-pointer-resolution`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/back_pointer.rs#back_pointer_evidence` (lines 374-434) 8d4657b2fa8d -> 3004719e176d, `rust/crates/urzua-core/src/rules/back_pointer.rs#embodiment_back_pointer_resolution` (lines 502-592) b275f5f4567d -> 88391fb788c0. Reason: Read the diff of each pinned file or range since the old pin against what this record evidences: back_pointer_evidence reads the shared ignore_marker, leaves a back-pointer on a line carrying it out of the code and test evidence and counts those lines; unchanged without a marker; embodiment_back_pointer_resolution reads the shared ignore_marker, skips a back-pointer on an excused line, counts excused lines in the population and reports a marker on a line with no back-pointer it reads; unchanged without a marker. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/back_pointer.rs#embodiment_back_pointer_resolution` (lines 493-582) 88391fb788c0 -> 1c2e7b0c09d5. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: back_pointer.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/back_pointer.rs#embodiment_back_pointer_resolution` (lines 602-704) 1c2e7b0c09d5 -> 3925330147be. Reason: MILE-207 path-scoped source read: back_pointer.rs: the resolution and decision rules take the sources value, judge the files under the requested paths and count the unread ones in deferred_by_scope; adds cites_any_record and decision_findings; a run with no scope is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
