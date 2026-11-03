---
default: patch
---

`urzua init` now proposes the header shape each record type's existing records use, instead of
`yaml-frontmatter` for every type, so the first `check` reads an adopted corpus rather than reporting
every record as having no header. A type whose records mix shapes gets the majority shape and a
`header-shape` notice naming the split; a type whose records parse under no shape keeps
`yaml-frontmatter` with a notice. A deprecated shape is still flagged by `header.deprecated-shape`.
