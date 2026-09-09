---
Stable-Id: 01M21FJ787H49Q7Z6AK579KJSE
Status: Fixed
Found-in: discussing whether header.pointer-field-clean's hardcoded field list (Implements/Derives-from/Parent) and pointer_resolution's own (Implements/Derives-from/Parent/Blocked-on) match this engine's own "declared, not hardcoded" pitch for an adopting org
Regression-test: 'rust/crates/urzua-core/src/rules.rs :: a_resolving_pointer_surfaces_target_status_without_judging_it (config-declared `pointer_fields` per type, no hardcoded field list)'
---
# 8 — README overstates pointer-field genericity -- Implements/Derives-from/Parent/Blocked-on are hardcoded, not config-declared

## What was wrong

The README's "Lineage between record types is also declared, not assumed" section (added earlier
this session) claims: *"None of `Implements`, `Derives-from`, or the graph's shape is special-cased
in `urzua-core` — they're plain field names an org declares in `known_fields` per type, resolved
generically by one rule..."* and gives, as an example of the engine's flexibility, an org adding a
field like `Feeds-into` that points forward instead of back.

That's not actually true. `pointer_resolution` scans a fixed Rust array --
`["Implements", "Derives-from", "Parent", "Blocked-on"]` -- never read from
`.urzua/config.toml`. An org declaring `Feeds-into` in their own `known_fields` would see it pass
`header.field-set-consistency` (that check IS config-driven) but never get resolved, checked for
dangling references, or shown in `urzua graph` -- because `pointer_resolution` doesn't know it
exists. Same problem in `blocked_on_stale` (hardcodes the literal string `"Blocked-on"`) and the
newly-added `header.pointer-field-clean` (hardcodes `["Implements", "Derives-from", "Parent"]`).

## Why nothing caught it

The README's claim was written to describe the *intent* behind `Implements`/`Derives-from`/`Parent`
not being type-special-cased (true: the same rule resolves them regardless of whether a record is an
`adr`, `rfc`, `milestone`, or anything else an org configures) -- but that's a narrower claim than
"any field name an org declares gets this behavior," which is what the `Feeds-into` example actually
asserted. Nothing checks README prose against the engine's real capabilities (the same permanent
content-scope ceiling this project names elsewhere), so the overstatement shipped uncaught.

## References

- RFC-23 -- the design proposal for making pointer/blocking field declarations real config, closing
  this gap properly rather than patching the README's wording alone.
- `rust/crates/urzua-core/src/rules.rs` -- `pointer_resolution`, `blocked_on_stale`,
  `header_pointer_field_clean`, the three hardcoded field lists this bug describes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial bug record, `Status: Open`. Not yet fixed -- the real fix is a design decision (RFC-23), not a mechanical patch; README wording will be corrected once that's decided, either by making the claim true or by narrowing what it asserts. | **structural** |
> | 2026-09-09 | Fixed by MILE-90/ADR-44: `pointer_resolution`, `header_pointer_field_clean`, and `narrative_field_stale` now read `pointer_fields`/`narrative_fields` per type from `.urzua/config.toml`, with no hardcoded field list anywhere in `rules.rs`. The README's `Feeds-into` example is now literally true, not aspirational. `Status: Fixed`. | **substantive** |
