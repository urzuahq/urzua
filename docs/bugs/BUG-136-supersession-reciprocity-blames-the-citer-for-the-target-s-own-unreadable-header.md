---
Stable-Id: 01M371VXJRK2JTHEJACPC7WHET
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 25"
Regression-test: "a_target_with_an_unreadable_header_is_not_blamed_for_failing_to_reciprocate_observed_failing, rules.rs"
---
# 136 — supersession_reciprocity blames the citer for the target's own unreadable header

## What was wrong

`supersession_reciprocity` reads a resolved target's own `Supersedes / Superseded-by` value via
`target.header.get(target_field).unwrap_or("")`, with no check on whether `target`'s header is
unreadable first — unlike its own check for the *citing* record's header (`record.header.is_unreadable()`,
just above in the same function) and unlike `declared_cross_record_value`'s `FieldRead::Unreadable`
handling used everywhere a *target*'s field is read for reciprocity/consistency elsewhere in this file.

`Header::get` returns `None` for an unreadable header exactly as it would for a genuinely absent field
— there is no way to tell the two apart from `get`'s return value alone. So if record A validly cites
record B in a `Supersedes` field, but B's YAML header fails to parse, `unwrap_or("")` reads as B having
never named A back, and A gets a spurious "does not reciprocally name back" finding for a failure that
is entirely B's — already disclosed by B's own `Unreadable` population entry and by
`header.required-fields`.

## Why nothing caught it

No test exercised a target whose header does not parse; every existing `supersession_reciprocity` test
either has both records parse cleanly or has the *citing* record's own header be unreadable, which is
already correctly guarded.

## References

- `declared_cross_record_value`/`FieldRead::Unreadable` — the established pattern this function should
  have followed for reading a target's field.
- `supersession_reciprocity_discloses_unreadable_on_its_own_observed_failing` — the sibling test
  covering the citer's own header, which is what made this asymmetry easy to miss.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass. **Why:** found by a full-release code review; a planted-violation test observed failing (a spurious finding against the citer) before the fix, confirming clean after. | **substantive** |
