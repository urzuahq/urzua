---
default: patch
---

Fixes two silent-corruption risks from ADR-57's exact-match `Header::get`. `waiver`'s `Rule`/`Scope`/
`Expires` and `migrate ids`'s `Stable-Id` are the engine's own reserved keys, never adopter-declared
vocabulary — there is exactly one field named `Stable-Id`, so a record spelling it `stable-id` has made
a typo, not declared a different field. Reading them exactly let a waiver written with lowercase keys
be silently dropped from the active set, and let `migrate ids --apply` assign and write a second,
conflicting `Stable-Id` onto a record that already had one under different casing. Both now match
case-insensitively via a new `Header::get_reserved`, kept separate from `get`'s adopter-vocabulary exact
match. `Status`/`Embodiment`/`Realized-by` are unaffected: they are adopter-declared `known_fields`
vocabulary and ADR-57's exact match is the intended behaviour for them.
