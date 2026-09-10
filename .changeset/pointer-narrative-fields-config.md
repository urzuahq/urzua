---
default: major
---

# `pointer_fields`/`narrative_fields` are now config-declared per type

`pointer_resolution`, `header.pointer-field-clean`, and the renamed `narrative-field.stale` (was
`blocked-on.stale`) previously scanned a fixed Rust array — `Implements`/`Derives-from`/`Parent`/
`Blocked-on` — regardless of what a type's `.urzua/config.toml` entry actually declared. An org
adding a custom relationship field (e.g. `Feeds-into`) would pass `header.field-set-consistency`
but never get it resolved, checked for dangling references, or shown in `urzua graph`.

Both are now real per-type config keys, `pointer_fields` (clean, comma-separated references,
format-enforced) and `narrative_fields` (prose-tolerant, staleness-checked) — undeclared means zero
fields of that kind checked, no implicit default. **If your own `.urzua/config.toml` doesn't declare
either key for a type, that type silently loses the pointer-resolution/clean-format checking it used
to get for free from the old hardcoded field list** — add `pointer_fields`/`narrative_fields`
explicitly to keep the same coverage; three new `check` rules will otherwise stay silent on an
undeclared type rather than warning you. `config.pointer-declaration-missing` (a type must declare
both lists explicitly, even as `[]`), `config.pointer-field-not-known` (a declared field must also
be in `required_fields`/`known_fields`), and `config.pointer-narrative-overlap` (a field can't be in
both lists).

`urzua graph` gains a `kind: "pointer" | "narrative"` field on every edge, is now config-driven
instead of a hardcoded 3-field list (so a type's own `Parent` now appears for free), and no longer
reports a false `dangling: true` for a padded reference against an unpadded filename (a normalization
gap `pointer_resolution` never had).
