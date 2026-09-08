---
Stable-Id: 01M21FK8QA0D6QJQWHKBND90WB
Status: Draft
Date: 2026-09-08
Author: '@beauwilliams'
Implements: BUG-8
---
# 23 — Config-declared pointer and blocking fields, instead of hardcoded in `urzua-core`

## Summary

`pointer_resolution`, `blocked_on_stale`, and `header.pointer-field-clean` each hardcode a fixed
list of field names (`Implements`/`Derives-from`/`Parent`/`Blocked-on`) directly in Rust source,
rather than reading them from `.urzua/config.toml` the way `header_shape`/`known_fields`/`spec` are
already declared per type. Propose making the field lists themselves configurable, closing the real
gap `BUG-8` names.

## Motivation

Found live, discussing whether an adopting org could add their own relationship vocabulary (e.g. a
field named `Feeds-into`, or `Realizes`, or `DependsOn`) the way they can already add their own
record types via config with zero `urzua-core` changes. They can't, today: `pointer_resolution`
scans a fixed array; a field outside it is invisible to resolution checking, dangling-reference
detection, and `urzua graph`, no matter what `known_fields` declares. This directly contradicts
`BUG-8`'s finding that the README's own pitch overstates this as already working.

The gap is sharper than "one list should be config": there are genuinely **two different kinds** of
pointer field already implicit in the code, never named as such:

1. **Plain resolving pointers** (`Implements`, `Derives-from`, `Parent` today) -- exist, resolve to
   a real record, nothing else checked.
2. **Staleness-aware, prose-tolerant pointers** (`Blocked-on` today) -- resolve *if* a reference is
   present, tolerate free text alongside or instead of one, and additionally check the target's
   `Status` for terminal-ness.

Making only the *list* configurable without naming this distinction would let an org declare a
field as "pointer-like" without saying which behavior it should get -- `header.pointer-field-clean`
would then have no way to know whether to enforce cleanliness (kind 1) or tolerate prose (kind 2).

## Proposal

Two new per-type config concepts, declared the same way `known_fields` already is:

```toml
[record_types.rfc]
# ...existing fields...
pointer_fields = ["Implements", "Derives-from", "Amends"]
blocking_fields = ["Blocked-on"]
```

- **`pointer_fields`**: resolved by `pointer_resolution` (existence-checked, `urzua graph` edges
  labeled by field name), and enforced clean by `header.pointer-field-clean` -- any comma-separated
  entry that isn't exactly a reference token is a finding, no exceptions.
- **`blocking_fields`**: resolved by `pointer_resolution` too, but *not* subject to the
  clean-field check, and separately checked by `blocked_on_stale` for target terminal-status.
  `extract_references`'s existing lenient, leading-token extraction is exactly right for this kind
  and stays unchanged.

`Parent`/`Implements`/`Derives-from` become the *default* `pointer_fields` when a type declares
none (backward-compatible with every type in this repo's own corpus today); `Blocked-on` likewise
defaults into `blocking_fields`. An adopting org overrides either list per type, the same additive,
non-breaking shape `header_layout`/`known_fields` already established.

## Open questions

- **Does this apply per-type or globally?** Milestones might reasonably want a `Feeds-into` field
  that adr/rfc never use -- per-type declaration (as sketched above) seems right, but adds one more
  config surface per type; worth weighing against a single global list if no real case demands
  per-type variance yet.
- **Backward compatibility precisely**: does an *undeclared* `pointer_fields`/`blocking_fields` on
  an existing type silently default to today's hardcoded behavior, or does `doctor` need a check
  flagging "no pointer fields declared" the way `type.no-declared-spec` does for missing specs?
- **Does `urzua graph`'s output format need to change** to distinguish a `pointer_fields` edge from
  a `blocking_fields` edge, or is the field-name label (already present on every edge) sufficient?
- **README correction scope** -- once this lands, is the `Feeds-into` example in the lineage section
  finally true as written, or does it still need adjusting for some other reason found along the way?
- **This RFC's own header is a live instance of the gap it names.** `Implements: BUG-8` above
  resolves correctly (`pointer_resolution` doesn't care that `rfc`'s `known_fields` never declared
  it), but `header.field-set-consistency` separately flags it as undeclared -- deliberately left
  firing rather than quietly silenced by adding `Implements` to `rfc`'s `known_fields`, since
  whether `rfc` *should* declare it is exactly the kind of small, real decision this RFC's own
  proposal would make legible instead of ad hoc. Left open, not decided here.

## Non-goals

- **Does not decide the exact config key names** (`pointer_fields`/`blocking_fields` are working
  names, not final).
- **Does not implement a third pointer-field kind** beyond the two identified -- if a real case
  needs something else later, that's a separate RFC, per this project's own "add a field once a
  pattern recurs" discipline.
- **Does not retroactively migrate this repo's own config** -- covered by whatever milestone
  implements this RFC, not decided here.

## References

- BUG-8 -- the concrete defect this RFC's `Implements` pointer closes: the README's overstated
  claim, and the underlying hardcoding it was wrong about.
- ADR-38/39/43 -- the "declared, not voted or inferred" precedent this proposal extends to pointer
  fields.
- RFC-20 -- the CLI/command taxonomy RFC, whose own table already asserts pointer fields are
  "plain field names an org declares" -- this RFC is what would make that literally true.
- RFC-22 -- a related, separately-filed idea (a `blocker` record type) that would sit on top of
  whichever design this RFC lands on, not a substitute for it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial RFC. **Why:** found live discussing whether an adopting org could add its own relationship vocabulary the way it can already add record types -- it can't, today, closing exactly the gap BUG-8 names in the README's own claim. | **structural** |
