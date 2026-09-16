---
Stable-Id: 01M2MF5T28P4GAQRK2VYBDFZRE
Status: Fixed
Found-in: 'a review pass over urzua-core while fixing BUG-23 -- `is_active` compares `Expires` lexically without ever checking it is a date'
Regression-test: 'rust/crates/urzua-core/src/waiver.rs :: an_unparseable_expiry_does_not_suppress_observed_failing'
Realized-by: code:rust/crates/urzua-core/src/waiver.rs
---
# 31 — A waiver with an unparseable Expires value never expires

## What was wrong

`Waiver::is_active` decided expiry with a string comparison:

```rust
Some(expires) => today <= expires.as_str(),
```

`Expires` was never parsed or validated. ASCII digits sort below letters, so `"2026-09-16" <= "soon"`
is `true` — and stays true on every future date. `Expires: soon`, `Expires: TBD`, `Expires: Q4`, and
a mistyped `Expires: 2026-9-16` all produced a waiver that **can never expire**.

The method's own doc comment asserted the invariant it did not enforce: *"ISO 8601 dates compare
correctly as plain strings."* True once both sides are known dates, which nothing established.

This is the one direction a suppression mechanism must not fail. The module doc argues a malformed
waiver "fails toward more findings, not fewer" — correct for a missing `Rule` or `Scope`, which
`load_waivers` drops via `filter_map`, and exactly inverted here.

## Why nothing caught it

`docs/waiver/` is empty — this repo has never written a waiver, so the path has never run outside its
own unit tests, and those only ever passed well-formed dates. `field.quality` does not reach it
either: that rule examines a type's `required_fields`, and `Expires` is optional for `waiver`, so
even `tbd` — already in `PLACEHOLDER_TOKENS` — is never classified here.

A mechanism whose only consumer is its own tests is not exercised, and a suppression mechanism that
is never exercised fails in the expensive direction the first time someone uses it.

## The fix

`is_active` requires `YYYY-MM-DD` before trusting the comparison; anything else expires immediately.
Calendar validity is deliberately unchecked — a 31st of February still orders correctly against a
real date, which is all this comparison asks of it.

**Deliberately not fixed here:** a malformed expiry now suppresses nothing, but nothing reports that
it was malformed either. Surfacing it needs a `waiver.*` rule, which would mean a new rule id, a
`SPEC-2` row and a `README` row — scope this change does not carry. Filed as the remaining half.

## References

- `rust/crates/urzua-core/src/waiver.rs` -- `is_active`, `is_iso_date`.
- RFC-15 -- waivers as time-boxed first-class records; §3 is the rule this restores.
- ADR-9 -- a detector fails open, a prohibition fails closed. A waiver is a prohibition on reporting.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Filed and fixed in the same change, minus the reporting half. **Why:** latent -- there are no waiver records in this corpus, so this has never fired and the original framing of it as urgent was wrong. It is still the only place in the tool where failing open removes findings rather than adding them, which is why it was worth fixing before anyone writes the first waiver. | **substantive** |
