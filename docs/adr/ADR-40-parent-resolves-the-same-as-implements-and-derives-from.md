---
Status: Accepted
Stable-Id: 01M1Z6WV7FRDW9DP8D07GBQMFF
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/rules/pointer.rs@sha256:22715cc9865f08db72d682d61091677380dc52398b5bdb0fad59a7b724aa99f3, test:rust/crates/urzua-core/src/rules/pointer.rs@sha256:22715cc9865f08db72d682d61091677380dc52398b5bdb0fad59a7b724aa99f3
Date: 2026-09-07
Accepted-on: 2026-09-07
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: RFC-12
---
# 40 — Parent resolves the same as Implements and Derives-from

## Context

`pointer.resolution` has always scanned `Implements` and `Derives-from` for references and resolved
them against the discovered corpus. Every spec since SPEC-2 also carries a `**Parent:** SPEC-N`
field — the same shape of cross-reference — but `Parent` was never added to the rule's scanned field
list. Found live while writing the spec-bundling backfill (MILE-77): every spec's `Parent` pointer
had been completely unchecked since the field was invented. A typo'd or dangling `Parent` value
would never have been caught by anything.

## Decision

In the context of a reference field with the exact same resolve-or-error shape as `Implements`/
`Derives-from` sitting entirely outside the one rule that already checks that shape, we decided:
**add `"Parent"` to `pointer_resolution`'s scanned field list.** No new rule, no new code path — the
existing rule is already generic over field name and already handles trailing prose after a
reference correctly (`extract_references` takes only the first comma-separated entry's leading
`PREFIX-NUM` token, so `"SPEC-1 (v0 CLI). Cross-cutting rules — no-silent-no-op..."` still resolves
to exactly `SPEC-1`).

## Reversibility

Trivial: one array literal gains one string. Removing it later is equally trivial and doesn't
require any other change.

## Consequences

- Every existing spec's `Parent: SPEC-1` pointer is now verified on every `check` run, for free.
- A future record type introducing its own hierarchical pointer field under a different name (not
  `Parent`) would need the same one-line addition — this ADR doesn't generalize the rule to "any
  field that looks like a reference," it names `Parent` specifically, matching how `Implements`/
  `Derives-from` are also named specifically rather than inferred.

## References

- RFC-12 — the decision-before-implementation gate `pointer.resolution` already serves for
  `Implements`/`Derives-from`, now extended to `Parent`.
- SPEC-2 — where `Parent` was first introduced as a spec-to-spec pointer, unchecked until now.
- MILE-77 — the spec-bundling work that surfaced this gap while writing SPEC-8 through SPEC-15's
  own `Parent` pointers.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 0219adc7c. | **structural** |
> | 2026-11-04 | Pinned `rust/crates/urzua-core/src/rules/pointer.rs` by content hash after re-verifying it against the Decision (attested by beauwilliams). **Why:** Re-verified: parent resolves like Implements; the changes to pointer.rs are severity, exclude_files and fixtures only. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/rules/pointer.rs` f25158f2e1d0 -> a66a71521611. Reason: Re-verified: since the pin, pointer.rs changed only by adding suggested_action and action_source to its findings (ADR-73). Parent still resolves exactly as Implements and Derives-from do, which is what this decision is about. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/rules/pointer.rs` a66a71521611 -> f3c650ac0f3b. Reason: pointer.rs gained one test, a_bare_hyphen_number_token_is_not_a_reference_observed_failing (BUG-72's regression test), in its tests module; no non-test code in the file changed **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/pointer.rs` f3c650ac0f3b -> e4d8db46f2e3. Reason: Read the diff of each pinned span since the old pin and the claim still holds. rules/pointer.rs: only a test was added since the pin (a declared pointer field with a custom name is resolved and checked clean), no rule code changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/pointer.rs` e4d8db46f2e3 -> 2d164d27a2e9. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: pointer.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/pointer.rs` 2d164d27a2e9 -> 08a62da2bc14. Reason: Adds declared target types for reference fields (MILE-259). config.rs: a pointer_fields entry may be {name, targets} (the typed parse keeps only names), Config gains a skipped pointer_targets map, RelationPair gains optional target_types, a new load error InvalidPointerTargets; every other key unchanged. rules: new rule pointer.target-type (catalog.rs, pointer.rs, table.rs, default-on at error, not-enabled when nothing is declared); relation.rs cross-type-disallowed defers to it for a claim outside target_types. graph.rs lists relation-pair edges. fix.rs, migrate.rs, property.rs, type_scope.rs, sequence.rs: test fixtures add the new fields only. check_integration.rs: the header-none fixture declares a target list so the new rule is exercised. SPEC-3 documents the keys. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/pointer.rs` 08a62da2bc14 -> d056bfb37d37. Reason: pointer.rs: pointer.target-type's finding message reads '<field>: <ref> is of type '<t>', but <field> on type '<u>' may only name records of type <list>' instead of using an article before the quoted type; no rule's behaviour or other message changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/pointer.rs` d056bfb37d37 -> 2ef9b7057c50. Reason: rules/pointer.rs: merged main's pointer.target-type; this branch drops its early not-enabled return, since DefaultOn.applies now keeps it off when no target types are declared, with identical output **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/pointer.rs` 2ef9b7057c50 -> 22715cc9865f. Reason: rules/pointer.rs: header.pointer-field-clean treats an empty or null pointer value as naming no reference and reports nothing, with a unit test; it also carries the changes merged from main since the pin, which main's own review covered. The record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
