---
default: patch
---

Round 27's `/code-review` found and fixed four more defects:

- `header.field-set-consistency`, `header.pointer-field-clean`, `field.untrimmed-value`, and
  `header.field-case-mismatch` misclassified a `header_shape: none` type's declared slots as
  `Unreadable` instead of `OutOfScope` -- the same defect `BUG-137`/`BUG-142` fixed elsewhere,
  recurring in four more rules (`BUG-147`).
- `header.pointer-field-clean` rejected the `—` no-value placeholder when mixed with a real reference
  in the same field (`Derives-from: RFC-1, —`), contradicting its own documented contract (`BUG-148`).
- `IDENTITY_DEPENDENT_RULES` omitted `relation.target-status-undeclared`, so `init` could propose it on
  a prefix-less corpus even though it resolves references through the same identity-dependent mechanism
  as its listed sibling (`BUG-149`).
- `discover_tracked_files` spawned a redundant third `git` subprocess on every invocation; merged into
  the second call's own richer output, with a staged rename's two-path-field shape explicitly handled
  and tested (`BUG-150`).
- `init`'s config-rendering YAML serialization now fails loudly instead of silently writing an empty
  config body (`BUG-146`).

Two round-27 findings verified and refuted: a proposed status-comparison trim would have reversed an
already-decided, tested policy (`ADR-57`'s exact-match extends to status comparisons by design); a
population-eligibility change from `RFC-42`/`ADR-61` is deliberate, not an oversight.

All fixes: no behavior change beyond the corrected defect itself; full test suite and `make ci` pass
throughout.
