---
Stable-Id: 01M36HC1J96P0329KDV0X4TVQR
Status: Fixed
Found-in: "Investigating a follow-up question during round 21's PR review: whether Header's region/parse_error Option pair (BUG-125's root shape) recurs elsewhere in the codebase"
Regression-test: "a_header_none_type_with_known_fields_is_a_contradiction_observed_failing, a_header_none_type_with_pointer_fields_is_a_contradiction_observed_failing, a_header_none_type_with_narrative_fields_is_a_contradiction_observed_failing, a_header_none_type_with_a_relation_field_override_is_a_contradiction_observed_failing, rules.rs"
---
# 128 — config.header-none-has-no-required-fields doesn't cover pointer/narrative/relation/known fields

## What was wrong

`config.header-none-has-no-required-fields` (`ADR-50`) forbids a `header_shape: none` type from also
declaring a non-empty `required_fields` -- a type with no header has nowhere for a required field to
be. `known_fields`, `pointer_fields`, `narrative_fields`, and `relation_fields` are the exact same
contradiction, one level over: each names a field, or a field-name override, that a type with no
header has nowhere to hold. None of the four had an equivalent guard.

Reachable, not hypothetical: a config declaring

```yaml
record_types:
  waiver:
    header_shape: none
    pointer_fields: ["Parent"]
```

passes config validation today. Every rule that reads that declared slot (`header.pointer-field-clean`,
`pointer.resolution`, `pointer.target-status`, ...) then reports every one of that type's records as
`Outcome::Unreadable`, via the same `Header::is_unreadable()` this session's `BUG-125` fix introduced.
That's true in the narrowest sense -- there genuinely is no header to read -- but it's a misleading
diagnosis for a config authoring mistake, not a defect in any actual record.

## Why nothing caught it

`config_header_none_has_no_required_fields` was built and named for the one field list (`required_fields`)
that prompted `ADR-50`'s guard; `known_fields`, `pointer_fields`, `narrative_fields`, and
`relation_fields` were added to the config schema later (`MILE-90`/`ADR-44`, `RFC-42`/`ADR-61`) without
anyone re-checking this specific rule's scope against the new fields. This repository's own config has
no `header_shape: none` type declaring any of the four, so nothing in this corpus's own `make ci` run
could have surfaced the gap.

Found by generalizing a smaller, adjacent observation: `Header`'s own `region`/`parse_error` fields
(this session's `BUG-125`) represent a state space narrower than their types allow, and different call
sites trusted different subsets of it. A follow-up sweep asking "does this same shape recur elsewhere"
found this: `config_header_none_has_no_required_fields`'s existing check is correct but scoped to only
one of five field-declaration lists that are all the same contradiction under `header_shape: none`.

## What changed

`config_header_none_has_no_required_fields`'s body now checks all five: `required_fields` (unchanged),
`known_fields`, `pointer_fields`, `narrative_fields`, and `relation_fields` (any declared role
override). Each produces its own finding naming the specific field list at fault. An empty list
(`pointer_fields: []`) is still not a contradiction -- `MILE-90`/`ADR-44` already treats declaring an
explicit empty list as a conscious "zero fields" choice, not a gap.

## References

- `ADR-50` -- the original decision this rule enforces, now enforced for all five field-declaration
  lists instead of one.
- `BUG-125` -- the adjacent defect (`Header`'s own `region`/`parse_error` pair) whose investigation
  surfaced this one.
- `MILE-90`/`ADR-44`, `RFC-42`/`ADR-61` -- where `known_fields`/`pointer_fields`/`narrative_fields`/
  `relation_fields` were added to the schema without this rule's scope being revisited.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass. **Why:** found by generalizing `BUG-125`'s root shape (a type representing a narrower state space than it allows) to ask whether it recurred elsewhere in the codebase; confirmed reachable via a real config shape, currently a no-op on this repository's own config, and fixed with four new planted-violation tests each observed failing before the fix. | **substantive** |
