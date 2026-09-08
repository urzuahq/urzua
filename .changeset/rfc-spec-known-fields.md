---
default: patch
---

# Fix: `rfc`/`spec` never declared `Stable-Id` in `known_fields`

`render_synthetic_yaml` (what `urzua new` uses for any `yaml-frontmatter`-shaped type) assigns
every type a `Stable-Id` unconditionally (ADR-21) -- but `rfc` and `spec`'s own `known_fields`
never listed it, so the first `rfc`/`spec` record ever created through `urzua new` would trip
`header.field-set-consistency` on its own `Stable-Id` field. Fixed: both types now declare it.
