---
Stable-Id: 01M6JKNT9WX2Y4PVJYEMKKZQ6X
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/config.rs, test:rust/crates/urzua-cli/tests/scoped_pair_integration.rs#the_same_claim_errors_when_the_pair_is_global@sha256:fac1298577d8aac20e6c670c8bcb2093a6d678b6e1e29f203c69fa9cd93c1dc2, test:rust/crates/urzua-cli/tests/scoped_pair_integration.rs#an_rfc_amending_an_adr_is_eligible_and_unexamined_under_the_scoped_pair_observed_failing@sha256:ebe864b5e5fecfd6f27e8d808e702fe8781466ea402b6cf6b6d4a814a4a504de, spec:docs/specs/SPEC-3-configuration.md, code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-relation-rules-run.yaml
---
# 155 — Relation pairs declared per type so a pair between specs does not error on a type that cannot carry the inverse

## What

Let a relation pair be bound to the record types that carry it, so a pair declared between one type does not error on a claim to a type that cannot hold the inverse field.

## Why

`relation_pairs` are global. A pair declared between specs errors on a claim to a type with no inverse field, and `cross_type: false` only moves the error to another rule. SPEC-3 describes per-type binding as none, which is the gap; the rule cannot express a pair that exists only for some types. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- A pair can name its carrying types. — evidence: code:rust/crates/urzua-core/src/config.rs, test:rust/crates/urzua-cli/tests/scoped_pair_integration.rs#the_same_claim_errors_when_the_pair_is_global@sha256:fac1298577d8aac20e6c670c8bcb2093a6d678b6e1e29f203c69fa9cd93c1dc2
- A claim to an uncarrying type is judged by that scoping rather than erroring. — evidence: test:rust/crates/urzua-cli/tests/scoped_pair_integration.rs#an_rfc_amending_an_adr_is_eligible_and_unexamined_under_the_scoped_pair_observed_failing@sha256:ebe864b5e5fecfd6f27e8d808e702fe8781466ea402b6cf6b6d4a814a4a504de
- The change is recorded in the spec that owns relation pairs. — evidence: spec:docs/specs/SPEC-3-configuration.md
- A fixture covers a scoped pair and a global one. — evidence: test:rust/crates/urzua-cli/tests/scoped_pair_integration.rs#an_rfc_amending_an_adr_is_eligible_and_unexamined_under_the_scoped_pair_observed_failing@sha256:ebe864b5e5fecfd6f27e8d808e702fe8781466ea402b6cf6b6d4a814a4a504de, test:rust/crates/urzua-cli/tests/scoped_pair_integration.rs#the_same_claim_errors_when_the_pair_is_global@sha256:fac1298577d8aac20e6c670c8bcb2093a6d678b6e1e29f203c69fa9cd93c1dc2
- The capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-relation-rules-run.yaml

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Implemented: a relation pair may declare `types`; a claim to a type outside it is out of scope in `relation.pair-reciprocity`'s population. This repository scopes the `amendment` pair to `[rfc]`, and `scoped_pair_integration.rs` shows an RFC amending an ADR scoped out and the same claim erroring under a global pair. `Status` stays `Planned`. **Why:** the Done-when contract; status is a human decision. | **substantive** |
> | 2026-11-04 | Follow-up: a claim to a type outside a pair's `types` is eligible and unexamined, not out of scope, so `config.scope-matches-nothing` does not warn on it (`SPEC-2` 0.42, `SPEC-3` 0.30). **Why:** a legitimate cross-type claim is not a configuration the tool cannot reach. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #196: a relation pair declares the `types` that carry it. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
