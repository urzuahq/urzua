---
Stable-Id: 01M369EMT7WH2ECKJ8RH2ZRQ4M
Status: Fixed
Found-in: "A /code-review 109 pass, reviewing this round's PR before merge"
Regression-test: "a_header_none_type_with_no_known_fields_is_not_a_missing_declaration_observed_failing, rules.rs"
---
# 124 — config.known-fields-declaration-missing flags a header-shape-none type that structurally cannot declare known_fields

## What was wrong

`config_known_fields_declaration_missing` (`RFC-43`/`ADR-62`) reported every record type declaring no
`known_fields` at all, with no exemption for a type declaring `header_shape: none`. A `none`-shaped
type has nowhere for any header field to be at all (`ADR-50`) — `known_fields` governs which fields
are permitted *beyond* `required_fields`, which is meaningless when no field can exist. Enabling the
new rule on a repository with any `none`-shaped type would force a meaningless `known_fields: []`
onto a type that structurally can never have one, the same reasoning
`config.header-none-has-no-required-fields` already applies to `required_fields` for the same shape.

## Why nothing caught it

Found by a `/code-review 109` pass the same day the rule shipped, before merge — its own tests all
used ordinary header-bearing types, none exercising a `header_shape: none` type against the new rule.

## What changed

Added the same `header_shape != HeaderShape::None` guard `config_header_none_has_no_required_fields`
already uses for `required_fields`, skipping a `none`-shaped type entirely rather than reporting it.

## References

- `ADR-50` — `header_shape: none`, the shape this bug's exemption applies to.
- `config.header-none-has-no-required-fields` — the sibling rule whose exemption reasoning this
  mirrors.
- `RFC-43`, `ADR-62` — the rule this bug is a gap in.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass. **Why:** found by a `/code-review 109` pass before merge; a planted-violation test observed failing against the un-exempted rule before the fix. | **substantive** |
