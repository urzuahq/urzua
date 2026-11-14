---
Stable-Id: 01M6CXZXSNMN0AYVBVYEWRY95J
Status: Done
Phase: '0'
Track: schema-governance
Realized-by: spec:docs/adr/ADR-36-filenames-carry-the-type-prefix-going-forward.md, test:rust/crates/urzua-cli/tests/check_integration.rs#a_bare_number_corpus_is_reported_once_per_type_with_rename_guidance@sha256:cbe75ec7f8dd363b5945a74a54669bd1ee721798e2be92e93273d8538909c31d, test:rust/crates/urzua-core/src/values.rs#record_id_compares_by_numeric_value@sha256:2e7b3ad606e780cf911ea4ca968583e6965edd3139e656283974644fb202a570, test:rust/crates/urzua-cli/tests/document_model_integration.rs#an_identity_pattern_without_a_prefix_group_stops_the_run_naming_the_type@sha256:2d0102d6c5355403873cff6f67e9042829dcf6aa7667e0ad8994d25b2d98c747, spec:docs/rfc/RFC-46-a-bare-numeric-filename-gets-its-identity-from-the-type-not-from-the-filename.md
Non-reference-ids: ADR-173, ADR-0173
---
# 126 — Decide how a record gets an identity from a bare zero-padded numeric filename

## What

Decide how a record gets an identity from a bare zero-padded numeric filename such as `0001-a.md`, and
how a `prefix` declared on the type relates to it. Reproduced on a two-record corpus with
`0001-a.md` and `0002-b.md` in `docs/adr`, a config declaring `prefix: ADR`, and
`Supersedes: ADR-1` / `Superseded-by: ADR-2`: `urzua audit` reports both `pointer.resolution` targets as
"does not resolve to any discovered record". Declaring a prefix does not give bare-number files an
identity. Prose and code in such corpora cite the zero-padded form, `ADR-0173`, which the decision must
also say how to resolve.

## Why

A real adopter corpus of several hundred records uses exactly this filename shape, so every cross-record
pointer in it fails to resolve until this is decided; the first `check` is wrong in the same silent way
`BUG-99` found for `audit`. `ADR-36` as amended accepts only type-prefixed filenames, and a reference to a
bare-number filename is dangling; this milestone decides whether that policy should change for
corpora already in this shape. `MILE-98` plans the declared identity pattern that could express the
change. The
open question is whether a prefix on the type is enough, whether the identity pattern is the only
route, and how `ADR-1` and `ADR-0001` relate. The reproduction was recorded when this was filed; re-run it
when the decision is taken.

## Done means

- A record whose filename is a bare zero-padded number (`0001-a.md`) gets no identity from the filename, and such a corpus is renamed to `TYPE-NNNN-slug.md`, as `ADR-36` decides. — evidence: spec:docs/adr/ADR-36-filenames-carry-the-type-prefix-going-forward.md, test:rust/crates/urzua-cli/tests/check_integration.rs#a_bare_number_corpus_is_reported_once_per_type_with_rename_guidance@sha256:cbe75ec7f8dd363b5945a74a54669bd1ee721798e2be92e93273d8538909c31d
- `ADR-0173` resolves against `ADR-173` numerically. — evidence: test:rust/crates/urzua-core/src/values.rs#record_id_compares_by_numeric_value@sha256:2e7b3ad606e780cf911ea4ca968583e6965edd3139e656283974644fb202a570
- A declared `identity` pattern must capture the type prefix. — evidence: test:rust/crates/urzua-cli/tests/document_model_integration.rs#an_identity_pattern_without_a_prefix_group_stops_the_run_naming_the_type@sha256:2d0102d6c5355403873cff6f67e9042829dcf6aa7667e0ad8994d25b2d98c747
- `RFC-46` is rejected. — evidence: spec:docs/rfc/RFC-46-a-bare-numeric-filename-gets-its-identity-from-the-type-not-from-the-filename.md

## References

- `BUG-99` -- `audit` exits 2 on the config `init` writes for a corpus with no type prefix.
- `MILE-98` -- the declared document model, including identity patterns.
- `ADR-36` -- filenames carry the type prefix going forward.

## Decision (2026-11-02)

`RFC-46` was rejected: a bare-number filename does not get an identity from its type. Such a corpus is renamed to `TYPE-NNNN-slug.md` (`ADR-36`), and zero-padded citations such as `ADR-0173` keep resolving because IDs are compared numerically. `MILE-120` reports the mismatch with rename guidance; `MILE-134` is the later rename tool. Closing this milestone is a human decision.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** A declared prefix does not give bare zero-padded numeric filenames an identity, so a real adopter's pointers all fail to resolve; filed as a decision. | **substantive** |
> | 2026-11-02 | Decision recorded from `RFC-46`'s rejection. **Why:** The question this milestone asked is now answered. | **substantive** |
> | 2026-11-04 | `MILE-98`'s declared `identity` pattern must capture the type prefix, so a bare zero-padded filename (`0001-a.md`) still gets no identity and the decision stands: such a corpus is renamed to `TYPE-NNNN-slug.md` (`ADR-36`), where `ADR-0173` resolves against `ADR-173` numerically. A pattern without a `(?P<prefix>...)` group fails the config load. **Why:** `RFC-46` was rejected; the declaration is not a back door to the bare-number identity it declined. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged on `main`: the decision is recorded in the revision log (a bare zero-padded filename gets no identity; such a corpus is renamed to `TYPE-NNNN-slug.md`) and in the accepted `ADR-36`; `RFC-46` was rejected. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-05 | Added the `Non-reference-ids` header field declaring `ADR-173, ADR-0173`. **Why:** the text names these ids as history or illustration, not as citations of a record; the declaration states that on the record itself, so a new unresolved citation in this file is reported, where a file-wide `citation.resolution` waiver had silenced every one (`BUG-190`, `ADR-75`). The field carries no decision; the milestone is unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/document_model_integration.rs#an_identity_pattern_without_a_prefix_group_stops_the_run_naming_the_type` (lines 135-147) e794aafea826 -> 2d0102d6c535. Reason: an_identity_pattern_without_a_prefix_group_stops_the_run_naming_the_type now asserts exit code 2 instead of not 0; unchanged behaviour **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
