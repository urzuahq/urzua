---
Stable-Id: 01M35T9QHFVNYZEDK6X18YTPWJ
Status: Fixed
Found-in: "A /code-review 108 pass, reviewing RFC-42/ADR-61's implementation"
Realized-by: test:rust/crates/urzua-core/src/fix.rs#a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing@sha256:d3d973d82fa85c9fe422813eeb6d8d95caff14147c12778f942d8afc6cd7443b
---
# 118 — urzua fix ignores a type's relation_fields override for Embodiment and Realized-by

## What was wrong

`RFC-42`/`ADR-61` made `Status`/`Embodiment`/`Realized-by`/`Supersedes / Superseded-by` adopter-declared
per type, and rewired all seven rules `check` runs through the new `relation_field(role)` accessor. It
missed a caller outside `check`: `urzua_core::fix::detect_repairs` still read the literals `"Embodiment"`
and `"Realized-by"` directly, with no `Config` parameter to resolve an override from at all.

A type declaring `relation_fields.embodiment_state`/`embodiment_locator` — exactly the shape `RFC-42`'s
own new tests exercise for `embodiment_consistency` — got `check` correctly flagging a mismatch while
`urzua fix` silently reported zero repairs for the same record: two commands reading one concept,
disagreeing the moment an adopter used the mechanism the review's own PR had just built.

## Why nothing caught it

`RFC-42`'s review scope was the diff itself (`config.rs`, `rules.rs`, `property.rs`, `check.rs`) — `fix.rs`
wasn't touched by that PR, so nothing in the diff pointed at it, and `fix`'s own tests all used the
default field names, which never exercises the divergence.

## What changed

`detect_repairs` now takes `&Config` and resolves `RelationRole::EmbodimentState`/`EmbodimentLocator`
through the same `relation_field_name` helper `check`'s rules use (promoted to `pub` to cross the crate
boundary into `urzua-cli`). The `Repair.field`/`evidence` values now name the resolved field, not a
hardcoded literal.

## References

- `RFC-42`, `ADR-61` — the mechanism this bug's fix threads into `fix.rs`.
- `BUG-119`, `BUG-120` — the same gap found the same day in `explain`/`graph` and drift detection.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass. **Why:** found by a full-PR code review of `RFC-42`'s implementation; a planted-violation test observed failing against the hardcoded literals before the fix. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs#a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing` (lines 1063-1129) 5aa5095668de -> 46f01692c3ff. Reason: Read the diff of each pinned span since the old pin and the claim still holds. fix.rs#a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing: the fixture literals gained the new default fields (roadmap, attestation, new, discovery, harness, authority, permanent) and the test helper detect replaced the detect_repairs call, assertions unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs#a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing` (lines 1288-1355) 46f01692c3ff -> 8be1ccb0c55d. Reason: fix.rs: the only change is one added `hooks: Default::default()` field in a Config struct literal, needed by the new optional hooks key; behaviour is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs#a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing` (lines 1294-1362) 46f01692c3ff -> d3450d9ad562. Reason: fix.rs only gains rejected_values and waiver_fields in a test fixture's RecordTypeConfig literal. The claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Pin recomputed on merge (attested by beauwilliams, identity: gh-verified, actor: agent): both merged sides re-pinned, `rust/crates/urzua-core/src/fix.rs#a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing` 8be1ccb0c55d and d3450d9ad562 -> 11d346bc001e; the new hash is the merged file's, which is git's automatic merge of the two re-pinned commits. No person wrote a reason for the merged state. **Why:** `urzua fix --conflicts` resolved a conflict that lay only in re-pin lines; a reviewer of the pull request should confirm the merged pin. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs#a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing` (lines 1508-1578) 11d346bc001e -> 570e5bf3c148. Reason: Adds declared target types for reference fields (MILE-259). config.rs: a pointer_fields entry may be {name, targets} (the typed parse keeps only names), Config gains a skipped pointer_targets map, RelationPair gains optional target_types, a new load error InvalidPointerTargets; every other key unchanged. rules: new rule pointer.target-type (catalog.rs, pointer.rs, table.rs, default-on at error, not-enabled when nothing is declared); relation.rs cross-type-disallowed defers to it for a claim outside target_types. graph.rs lists relation-pair edges. fix.rs, migrate.rs, property.rs, type_scope.rs, sequence.rs: test fixtures add the new fields only. check_integration.rs: the header-none fixture declares a target list so the new rule is exercised. SPEC-3 documents the keys. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs#a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing` (lines 1517-1587) 11d346bc001e -> 57dc9069b997. Reason: fix.rs: a test fixture's RecordTypeConfig literal gains field_values: None; no behaviour changed **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Pin recomputed on merge (attested by beauwilliams, identity: gh-verified, actor: agent): both merged sides re-pinned, `rust/crates/urzua-core/src/fix.rs#a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing` (Realized-by) 570e5bf3c148 and 57dc9069b997 -> 654660a51035; the new hash is the merged file's, which is git's automatic merge of the two re-pinned commits. No person wrote a reason for the merged state. **Why:** `urzua fix --conflicts` resolved a conflict that lay only in re-pin lines; a reviewer of the pull request should confirm the merged pin. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs#a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing` (lines 1519-1591) 654660a51035 -> b2fb85387b35. Reason: rust/crates/urzua-core/src/fix.rs: a test fixture that builds a Config gains the new declared_status_entries field; no production code changed **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-18 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs#a_type_declaring_custom_embodiment_field_names_is_repaired_by_those_names_observed_failing` (lines 1521-1595) b2fb85387b35 -> d3d973d82fa8. Reason: fix.rs: test and fixture Config literals gain the part_of and export fields; no production behavior changed **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
