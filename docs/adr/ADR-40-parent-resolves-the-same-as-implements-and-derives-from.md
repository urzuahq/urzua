---
Status: Accepted
Stable-Id: 01M1Z6WV7FRDW9DP8D07GBQMFF
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/rules.rs, test:rust/crates/urzua-core/src/rules.rs
Date: 2026-09-07
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
