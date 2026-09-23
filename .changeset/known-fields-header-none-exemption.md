---
default: patch
---

Fixes `BUG-124`: the new `config.known-fields-declaration-missing` rule (`RFC-43`/`ADR-62`) flagged a
`header_shape: none` type for not declaring `known_fields`, even though such a type has nowhere for
any header field to be (`ADR-50`) and `known_fields` is meaningless for it. Now exempted, matching
`config.header-none-has-no-required-fields`'s existing reasoning for `required_fields`.
