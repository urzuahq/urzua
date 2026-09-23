---
default: patch
---

Fixes `BUG-128`: `config.header-none-has-no-required-fields` only forbade a `header_shape: none` type
from declaring `required_fields` — the same contradiction applies to `known_fields`, `pointer_fields`,
`narrative_fields`, and `relation_fields`, none of which had an equivalent guard. A type declaring
`header_shape: none` and `pointer_fields: ["Parent"]`, for example, previously passed config
validation and then had every one of its records misleadingly reported as `Outcome::Unreadable` by
every rule reading that slot. The same rule now checks all five field-declaration lists.

No adopter-facing behavior change for a config that doesn't hit this combination (this repository's
own config has no `header_shape: none` types declaring any of the four): verified with the full test
suite and `make ci`.
