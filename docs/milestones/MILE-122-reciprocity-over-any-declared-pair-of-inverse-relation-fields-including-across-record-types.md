---
Stable-Id: 01M6CXZWGF7KPM5ZBYF8X8J8PQ
Status: Done
Phase: '0'
Track: schema-governance
Realized-by: code:rust/crates/urzua-core/src/rules/relation.rs#relation_pair_reciprocity@sha256:9822ade0e7189a92c8cc94295ecc6a3aa89787b5cedf5ee2d4c75b9b9b8cfab2, test:rust/crates/urzua-core/src/rules/relation.rs#a_pair_may_span_two_record_types@sha256:3ecc2f8f6740e439f134637ec2f549a109c2c19a130b82a8fdee7cd387d4e41c, test:rust/crates/urzua-cli/tests/check_integration.rs#check_reports_an_unechoed_claim_in_a_declared_cross_type_relation_pair@sha256:e90dd7cc0837d4749c72e64f36a9ba5758148d108f2615d3998ad7ce77ff7a06, test:rust/crates/urzua-core/src/rules/relation.rs#an_inverse_claim_the_forward_side_does_not_echo_is_a_finding@sha256:4b7a28c106941bb21338aa8b220ba2c9c2eb06d7378e6d246e382451b9901955, test:rust/crates/urzua-core/src/rules/relation.rs#a_reciprocated_pair_has_no_finding_and_both_slots_are_examined@sha256:e50c0bcfb5cb5c447277c5ca5837ef6dc7112aa20ecc8478bb7239b7b382882a, code:rust/crates/urzua-core/src/rules/relation.rs#relation_cross_type_disallowed@sha256:e8c32fa328095ac078db882d9b80c036f8fe0efb395c2c1933352b138975099d, test:rust/crates/urzua-core/src/rules/relation.rs#a_cross_type_claim_in_a_same_type_pair_is_left_to_its_own_rule@sha256:ceb32e0dfa276c23d7e0ea545f0f18ccea505552721ef26df748ce603825a290, test:rust/crates/urzua-core/src/rules/relation.rs#a_one_directional_supersession_claim_is_a_reciprocity_violation@sha256:832e3c1b52727f47ccb60457ba95cfe493925a685cbfd11663c4971a5e5d46de, test:rust/crates/urzua-core/src/rules/relation.rs#a_reciprocated_supersession_produces_no_finding@sha256:6a8c4e9f2db315cdba05b1c9cead7614a025dd73f6c55bff68d6534d5fe2b965, spec:docs/specs/SPEC-3-configuration.md
---
# 122 — Reciprocity over any declared pair of inverse relation fields, including across record types

## What

Generalize reciprocity checking to any declared pair of inverse relation fields, including pairs that
span two record types (for example a field on one type and its inverse on another), instead of the one
`Supersedes / Superseded-by` pair the engine checks. The pair is declared in config; the rule reports a
claim made on one side that the other side does not echo.

## Why

`BUG-41` records that companion-field pairs are checked for one pair and hardcoded to it, and `BUG-17`
records that `Amends` has no reciprocal field or check at all. Both are the same gap seen from two
records; neither is a milestone, and each has stayed `Open`. A corpus declaring inverse
pairs of its own, some across types, cannot be checked until this exists, so it is a requirement for
`MILE-102`, not a nicety. It is also
what the config-driven thesis asks for: a hardcoded pair is a type-specific assumption in Rust source.

## Done means

- A `relation_pairs` config declaration names any pair of inverse relation fields, including one field on one type and its inverse on another. — evidence: code:rust/crates/urzua-core/src/rules/relation.rs#relation_pair_reciprocity@sha256:9822ade0e7189a92c8cc94295ecc6a3aa89787b5cedf5ee2d4c75b9b9b8cfab2, test:rust/crates/urzua-core/src/rules/relation.rs#a_pair_may_span_two_record_types@sha256:3ecc2f8f6740e439f134637ec2f549a109c2c19a130b82a8fdee7cd387d4e41c, test:rust/crates/urzua-cli/tests/check_integration.rs#check_reports_an_unechoed_claim_in_a_declared_cross_type_relation_pair@sha256:e90dd7cc0837d4749c72e64f36a9ba5758148d108f2615d3998ad7ce77ff7a06
- `relation.pair-reciprocity` reports a claim on one side that the other does not reciprocate. — evidence: test:rust/crates/urzua-core/src/rules/relation.rs#an_inverse_claim_the_forward_side_does_not_echo_is_a_finding@sha256:4b7a28c106941bb21338aa8b220ba2c9c2eb06d7378e6d246e382451b9901955, test:rust/crates/urzua-core/src/rules/relation.rs#a_reciprocated_pair_has_no_finding_and_both_slots_are_examined@sha256:e50c0bcfb5cb5c447277c5ca5837ef6dc7112aa20ecc8478bb7239b7b382882a
- `relation.cross-type-disallowed` reports a cross-type pair that a type disallows. — evidence: code:rust/crates/urzua-core/src/rules/relation.rs#relation_cross_type_disallowed@sha256:e8c32fa328095ac078db882d9b80c036f8fe0efb395c2c1933352b138975099d, test:rust/crates/urzua-core/src/rules/relation.rs#a_cross_type_claim_in_a_same_type_pair_is_left_to_its_own_rule@sha256:ceb32e0dfa276c23d7e0ea545f0f18ccea505552721ef26df748ce603825a290
- `relation.supersession-reciprocity` is unchanged. — evidence: test:rust/crates/urzua-core/src/rules/relation.rs#a_one_directional_supersession_claim_is_a_reciprocity_violation@sha256:832e3c1b52727f47ccb60457ba95cfe493925a685cbfd11663c4971a5e5d46de, test:rust/crates/urzua-core/src/rules/relation.rs#a_reciprocated_supersession_produces_no_finding@sha256:6a8c4e9f2db315cdba05b1c9cead7614a025dd73f6c55bff68d6534d5fe2b965
- The narrowest reading of each question `RFC-47` leaves open is in `SPEC-3`. — evidence: spec:docs/specs/SPEC-3-configuration.md
- Fixtures cover a reciprocated pair, a one-sided claim and a cross-type pair. — evidence: test:rust/crates/urzua-core/src/rules/relation.rs#a_reciprocated_pair_has_no_finding_and_both_slots_are_examined@sha256:e50c0bcfb5cb5c447277c5ca5837ef6dc7112aa20ecc8478bb7239b7b382882a, test:rust/crates/urzua-core/src/rules/relation.rs#an_inverse_claim_the_forward_side_does_not_echo_is_a_finding@sha256:4b7a28c106941bb21338aa8b220ba2c9c2eb06d7378e6d246e382451b9901955, test:rust/crates/urzua-core/src/rules/relation.rs#a_pair_may_span_two_record_types@sha256:3ecc2f8f6740e439f134637ec2f549a109c2c19a130b82a8fdee7cd387d4e41c

## References

- `BUG-41` -- companion-field pairs checked for one pair, hardcoded.
- `BUG-17` -- `Amends` has no reciprocal field or check.
- `MILE-102` -- the adopter replacement this serves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `BUG-17` and `BUG-41` describe halves of one gap and neither plans the general fix; filed as the milestone that does. | **substantive** |
> | 2026-11-04 | Built: `relation_pairs` config declaration and the `relation.pair-reciprocity` and `relation.cross-type-disallowed` rules, beside the unchanged `relation.supersession-reciprocity`. The narrowest reading of each question `RFC-47` leaves open is recorded in `SPEC-3`. **Why:** `RFC-47` was accepted on 2026-11-04. `Status` is left for the maintainer. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #163: declared `relation_pairs` with `relation.pair-reciprocity` and `relation.cross-type-disallowed`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/relation.rs#relation_pair_reciprocity` (lines 403-444) bb167d7d0ef9 -> 9822ade0e718. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: relation.rs: split supersession_reciprocity into SupersessionCheck::judge and check_reference, and relation_pair_reciprocity into PairReciprocityCheck::judge and check_claim. Behaviour unchanged, outputs byte-identical (check, check --base and fix --rebaseline JSON compared before and after). **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
