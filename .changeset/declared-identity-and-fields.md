---
default: minor
closes: []
---

A record type can now declare how its records are identified and read. `identity` takes a
`pattern` over the file name, with the type prefix and the record number as `(?P<prefix>...)` and
`(?P<number>...)` groups; the prefix also declares that titles open `PREFIX-N`. A pattern without a
prefix group fails the load, and a file name without the type prefix gets no identity. `fields.from` chooses how header
fields are read: `yaml-frontmatter`, `blockquote`, `bold-list`, `none`, or the new `prefix-lines`
for bare `Key: value` lines before the first `##` heading. A record type that declares neither
loads exactly as before, and a pattern that does not compile fails the load.
