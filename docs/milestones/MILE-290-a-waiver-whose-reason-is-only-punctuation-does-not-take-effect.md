---
Stable-Id: 01M7HW0JV72KQ2GD2JTRCQHEQM
Status: Done
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 290 — A waiver whose reason is only punctuation does not take effect

## What

Treat a required waiver field that holds only a dash or other punctuation as blank, as the override log already does.

## Why

Found on 2026-11-16 while urzua's own builders and reviewers used the tool to land a week of pull requests. Two reviewers reproduced that a waiver with a reason of a single em dash takes effect and suppresses its finding, while the override log's check reads dashes as blank.

## Done means

- A waiver whose `Reason` is `—`, `-` or other punctuation with no letter or digit does not take effect and is reported, shown by a test. — evidence: test:rust/crates/urzua-core/src/waiver.rs@sha256:2016ec1cda92737859c8d152292ecdb4ab153cdbb3e9ecfa2289d4d0295880b5, code:rust/crates/urzua-core/src/waiver.rs#SuppressionType::field_state@sha256:9721f54466cb8a90bc28ba4328f0fda4b07c84f7ec5f68d81477669165a4eb95
- The override log and the waiver judgement share one definition of a blank value. — evidence: code:rust/crates/urzua-core/src/field_state.rs#is_blank_value@sha256:c453f106d3371ed0268f0a06189843dd4a4539bff8480b1c864c50b9b0810719, code:rust/crates/urzua-core/src/rules/override_log.rs#reviewer_state@sha256:a0bb6f40f1f85826489245aaa4d46f5847f2afd1b69a112c9ca440c8df72ddb2, test:rust/crates/urzua-core/src/field_state.rs@sha256:703b101454db6a4b86b5ef4ddf7ee51b398a7d51a5cc19c28f30fcb37a1684da

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (2 of 2 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
> | 2026-11-16 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 5 locators pinned, 2 file-level, 3 scoped. Evidence: rust/crates/urzua-core/src/waiver.rs@sha256:d6cc43b080a8, rust/crates/urzua-core/src/waiver.rs#SuppressionType::field_state@sha256:9721f54466cb, rust/crates/urzua-core/src/field_state.rs#is_blank_value@sha256:c453f106d337, rust/crates/urzua-core/src/rules/override_log.rs#reviewer_state@sha256:a0bb6f40f1f8, rust/crates/urzua-core/src/field_state.rs@sha256:bc254c840275 Reason: Pinned the tests and code each delivered Done-means item cites, as they stand in this pull request. **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/field_state.rs` bc254c840275 -> c6e7a582b17f. Reason: field_state.rs adds default_vocabulary_of, which names the vocabulary that reads a value as a marker, for doctor; classification is unchanged. The record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/waiver.rs` d6cc43b080a8 -> 5d2d1d6d977d, `rust/crates/urzua-core/src/field_state.rs` c6e7a582b17f -> 703b101454db. Reason: field_state.rs replaces default_vocabulary_of with vocabularies_of, which names every vocabulary that reads a value as a marker and whether the type listed it itself, for doctor; classification is unchanged; waiver.rs reads an empty Expires as no Expires rather than as an expired date, with a unit test. The record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/waiver.rs` 5d2d1d6d977d -> 2016ec1cda92. Reason: waiver.rs reads a present but empty or null Expires as the empty string again, so the waiver is expired and covers nothing as on release 0.4.7, never permanent; its unit test covers both forms and an absent Expires. The record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | The rows of 2026-11-17 (re-baseline) describe intermediate states of the pull request for `MILE-322` and `MILE-324` that the final change does not contain: a `field_state::default_vocabulary_of` function; reading an empty waiver `Expires` as no `Expires`. The final change writes such a field empty and lists it in `to_fill`, has no such flag or function, leaves `is_record_reference`'s visibility as on main, and keeps a present but empty `Expires` expired, as releases 0.4.7 and 0.4.8 do. The earlier rows stay as history. **Why:** review of that pull request asked that rows naming designs absent from the final tree be closed. | **structural** |
