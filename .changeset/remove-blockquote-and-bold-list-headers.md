---
default: major
closes: [BUG-184]
---

**Breaking: the `blockquote` and `bold-list` header shapes are removed.**

The `blockquote` (`> Key: value`) and `bold-list` (`- **Key:** value`) header shapes no longer have a reader. YAML
frontmatter is the one header shape with a reader (alongside `none` for a type with no header), and
`header_shape` now defaults to `yaml-frontmatter` when omitted.

A config that declares `header_shape: blockquote` or `bold-list` (or the same under `fields.from`)
fails to load with an error naming the removed shape. A record still written in either form is
reported by `header.required-fields` as unreadable, with the form named and the fix stated, rather
than as a record with no header; `urzua init` counts such records in a notice, and `urzua migrate ids`
names the form when it skips one. `urzua new` always writes YAML frontmatter.

The `header.layout-consistency` and `header.deprecated-shape` rules and the `header_layout` option are
removed with the shapes they judged; naming either rule under `rules:` is now an unknown-rule error.
Rewrite an un-migrated corpus's headers as YAML frontmatter before upgrading.
