# 3 — urzua new ignores the configured header_shape when a template file exists

> Status: Open
> Stable-Id: 01M1Y7W55XWYY7DJF2GF8PZX9T
> Found-in: found live while writing ADR-33 -- flipped adr's header_shape to yaml-frontmatter and ran `urzua new adr`, expecting YAML frontmatter output, and got blockquote content back unchanged
> Regression-test: rust/crates/urzua-cli/tests/check_integration.rs (not yet written -- planned: assert `new` emits YAML frontmatter when header_shape is yaml-frontmatter, even with a blockquote template present)
> Realized-by: —

## What was wrong

`render_from_template` in `new_record.rs` is chosen unconditionally whenever
`.urzua/templates/<type>.md` exists, regardless of what `header_shape` the type's config declares.
A type configured as `yaml-frontmatter` with an existing blockquote template still gets blockquote
output from `urzua new` -- silently inconsistent with what `check`/`fix` will then try to parse it
as.

## Why nothing caught it

No existing test constructs a type with both a template file present *and* a non-default
`header_shape` declared -- the two dimensions (template existence, configured shape) were never
tested in combination, only independently.

## References

- ADR-27 -- the original template-vs-synthesis decision this defect lives inside.
- ADR-33 -- where this was found live, and which explicitly named this as a required fix before
  this repo's own config could safely flip to yaml-frontmatter.
- Milestone-0001 -- the planned work to fix it, cross-linked via this record's own filename
  reference and the milestone's `Implements` field.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial record: defect diagnosed, not yet fixed. | **structural** |
