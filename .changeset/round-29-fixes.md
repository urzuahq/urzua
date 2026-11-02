---
default: patch
---

`fix --apply` and `migrate ids --apply` now change only the lines they mean to: a CRLF record keeps its
line endings, a file with no trailing newline stays without one, and `fix --apply` adds its revision-log
row inside the table whether the log is a blockquote table or a plain one. `fix --apply --ids <entry>`
now reports an entry that matches no pending repair as a failure (exit 1) naming it, instead of a silent
`ok` that wrote nothing. `urzua new` refuses a title with no ASCII letters or digits instead of creating
a slug-less `ADR-N-.md` whose number the next `new` would reissue.
