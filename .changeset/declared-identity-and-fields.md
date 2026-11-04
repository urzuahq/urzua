---
default: minor
---

A record type can now declare how its records are identified and read. `identity` takes a
`pattern` over the file name, with the record number as a `(?P<number>...)` group, so a corpus whose
files are named `0001-slug.md` gets identifiers without being renamed; an optional `(?P<prefix>...)`
group declares that titles open `PREFIX-N` instead of a bare number. `fields.from` chooses how header
fields are read: `yaml-frontmatter`, `blockquote`, `bold-list`, `none`, or the new `prefix-lines`
for bare `Key: value` lines before the first `##` heading. A record type that declares neither
loads exactly as before, and a pattern that does not compile fails the load.
