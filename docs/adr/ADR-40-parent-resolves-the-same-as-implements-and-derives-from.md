---
Status: Accepted
Stable-Id: 01M1Z6WV7FRDW9DP8D07GBQMFF
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/rules/pointer.rs@sha256:e4d8db46f2e32fd5b80d2a62fe9938bf7ef057e18de0e92d2f22c66987b057d9, test:rust/crates/urzua-core/src/rules/pointer.rs@sha256:e4d8db46f2e32fd5b80d2a62fe9938bf7ef057e18de0e92d2f22c66987b057d9
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
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/rules/pointer.rs` f25158f2e1d0 -> a66a71521611. Reason: Re-verified: since the pin, pointer.rs changed only by adding suggested_action and action_source to its findings (ADR-73). Parent still resolves exactly as Implements and Derives-from do, which is what this decision is about. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/rules/pointer.rs` a66a71521611 -> f3c650ac0f3b. Reason: pointer.rs gained one test, a_bare_hyphen_number_token_is_not_a_reference_observed_failing (BUG-72's regression test), in its tests module; no non-test code in the file changed **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/pointer.rs` f3c650ac0f3b -> e4d8db46f2e3. Reason: Read the diff of each pinned span since the old pin and the claim still holds. rules/pointer.rs: only a test was added since the pin (a declared pointer field with a custom name is resolved and checked clean), no rule code changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
