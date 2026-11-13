---
default: minor
closes: []
---

Add three opt-in rules over a record's section structure, each declared per record type under
`sections`. `section.duplicate` (`unique_headings: true`) reports a heading that appears twice at one
level under one parent, naming both lines. `section.level-skipped` (`no_skipped_levels: true`)
reports a section nested more than one level below the one above it, such as a `###` with no `##`
above it, reading levels from `##` down to the type's `depth`. `section.order`
(`order: {from: template}`) reports a section that follows one the type's template
(`.urzua/templates/<type>.md`) puts after it. A type that declares none of the keys is unchanged, and
a declared order whose template is missing or holds no section is an error on each record of the type, so it fails a `fail_on: warning` run and the records are counted unexamined.
