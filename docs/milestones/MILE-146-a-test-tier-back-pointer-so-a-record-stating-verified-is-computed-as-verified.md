---
Stable-Id: 01M6JKNM82HMHR4D1TCD2B4WMG
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/back_pointer.rs#back_pointer_evidence@sha256:8d4657b2fa8d03ae2e12961e30be957b468f8e5d0e891921d11a1f8c2300ef0f, test:rust/crates/urzua-cli/tests/test_tier_integration.rs#a_test_pointer_verifies_and_a_code_pointer_alone_does_not@sha256:b89acfd47fe19792744041288825020baf8894d3ceca6d2c5ff5bb21ef9f01dc, test:rust/crates/urzua-core/src/rules/embodiment.rs#a_test_back_pointer_computes_verified_observed_failing@sha256:a43003404dc1f130bd5932572c094563b524e6467bf4ffada2798a347cf380ad, test:rust/crates/urzua-cli/tests/test_tier_integration.rs#with_no_test_class_declared_a_verifies_comment_is_not_evidence@sha256:6d6253a7d10d2ee6d66b3a2f2b530e9ce8b0e598a1cc7dd2fc5aecc78a8bff4b, test:rust/crates/urzua-cli/tests/test_tier_integration.rs#a_glob_test_path_and_a_prefix_work_together_and_an_unmatched_entry_is_reported@sha256:44b405239ec6f65ebd42ed91d62faaeb026f846600a6f3add413ee9f1747893d, code:.urzua/config.yaml#rules::embodiment.back-pointer-resolution@sha256:be8df16da0185d6abd1edd784937fc9145c897eb80138620087e6bd2d3e35bd9, test:rust/crates/urzua-cli/tests/claim_field_integration.rs
---
# 146 — A test-tier back-pointer so a record stating Verified is computed as Verified

## What

Read a back-pointer from a test file (for example `Verifies: ADR-N`), or otherwise let a declared file class count as test evidence, so the computed Embodiment can reach the verified tier from tests that cite a record.

## Why

A real adopter corpus has a few hundred records stating `Verified`. urzua computes `Implemented` for them because no test-tier pointer is read, so the only way to agree is a very long hand-maintained list of `test:` locators per record, some lines thousands of characters long. That is the opposite of urzua's principle that evidence is read from the artifact rather than restated beside it, and it rots the moment a test moves. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- A test file carrying a back-pointer to a record is read as test evidence. — evidence: code:rust/crates/urzua-core/src/rules/back_pointer.rs#back_pointer_evidence@sha256:8d4657b2fa8d03ae2e12961e30be957b468f8e5d0e891921d11a1f8c2300ef0f, test:rust/crates/urzua-cli/tests/test_tier_integration.rs#a_test_pointer_verifies_and_a_code_pointer_alone_does_not@sha256:b89acfd47fe19792744041288825020baf8894d3ceca6d2c5ff5bb21ef9f01dc
- A record whose only extra evidence is such tests computes `Verified`. — evidence: test:rust/crates/urzua-core/src/rules/embodiment.rs#a_test_back_pointer_computes_verified_observed_failing@sha256:a43003404dc1f130bd5932572c094563b524e6467bf4ffada2798a347cf380ad
- The file class is declared in configuration. — evidence: test:rust/crates/urzua-cli/tests/test_tier_integration.rs#with_no_test_class_declared_a_verifies_comment_is_not_evidence@sha256:6d6253a7d10d2ee6d66b3a2f2b530e9ce8b0e598a1cc7dd2fc5aecc78a8bff4b, test:rust/crates/urzua-cli/tests/test_tier_integration.rs#a_glob_test_path_and_a_prefix_work_together_and_an_unmatched_entry_is_reported@sha256:44b405239ec6f65ebd42ed91d62faaeb026f846600a6f3add413ee9f1747893d
- A fixture covers both a test-pointer and a non-test pointer to the same record. — evidence: test:rust/crates/urzua-cli/tests/test_tier_integration.rs#a_test_pointer_verifies_and_a_code_pointer_alone_does_not@sha256:b89acfd47fe19792744041288825020baf8894d3ceca6d2c5ff5bb21ef9f01dc
- The capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml#rules::embodiment.back-pointer-resolution@sha256:be8df16da0185d6abd1edd784937fc9145c897eb80138620087e6bd2d3e35bd9, test:rust/crates/urzua-cli/tests/claim_field_integration.rs

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Test-tier back-pointer (`SPEC-2` `0.30`, `SPEC-3` `0.16`): `Verifies:` in a file under the declared `test_paths` computes `Verified`; exercised by `purity.rs` citing `ADR-5` in this repository, plus a fixture citing one record at both tiers. **Why:** the Done-when asks for it. Status unchanged; the maintainer closes. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #198: `Verifies:` in a file under the declared `test_paths` computes `Verified`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
