# 3 — urzua new ignores the configured header_shape when a template file exists

> Status: Fixed
> Stable-Id: 01M1Y7W55XWYY7DJF2GF8PZX9T
> Found-in: found live while writing ADR-33 -- flipped adr's header_shape to yaml-frontmatter and ran `urzua new adr`, expecting YAML frontmatter output, and got blockquote content back unchanged
> Regression-test: rust/crates/urzua-cli/tests/new_integration.rs::new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present -- observed failing against the pre-fix code (blockquote leaked through) before confirming it passes post-fix
> Realized-by: code:rust/crates/urzua-cli/src/main.rs, code:rust/crates/urzua-core/src/new_record.rs, test:rust/crates/urzua-cli/tests/new_integration.rs

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

## How it was fixed

`run_new` now branches on `type_config.header_shape` first: a `yaml-frontmatter` type always calls
`render_synthetic_yaml`, regardless of whether a template file exists. A new `template_body`
function extracts everything from a template's first `## ` section heading onward, so a template's
scaffolding (`## What`, `## Why`, the revision log) still gets spliced in under the synthesized
header when a template happens to exist -- fixing the header shape doesn't cost the template's body.
Every other shape (the default, `blockquote`/`bold-list`) keeps using `render_from_template`
unchanged.

## References

- ADR-27 -- the original template-vs-synthesis decision this defect lives inside.
- ADR-33 -- where this was found live, and which explicitly named this as a required fix before
  this repo's own config could safely flip to yaml-frontmatter.
- MILE-1 -- the planned work that fixed it, cross-linked via this record's own filename
  reference and the milestone's `Implements` field.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial record: defect diagnosed, not yet fixed. | **structural** |
> | 2026-09-07 | Fixed: configured header_shape wins unconditionally over a template's own shape; template body still spliced in under synthesized frontmatter. | **substantive** |
