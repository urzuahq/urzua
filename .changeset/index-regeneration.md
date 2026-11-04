---
default: minor
---

Index checking reads more shapes, and a new command regenerates an index. A declared `index.freshness`
`format` may span several lines (write the lines separated by `\n`), so a YAML-style or
link-plus-description entry is checked as one entry; it needs `{id}` or `{file}`, so a link list that
shows no id is identified by its file; and `{title}` is the heading without the record's number or
id. `urzua index` regenerates the declared index artifacts from the records: it only reports which
differ until `--apply` writes them, keeps everything outside the `begin`/`end` markers, and a second
run changes nothing.
