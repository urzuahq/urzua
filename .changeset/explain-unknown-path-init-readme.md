---
default: patch
closes: []
---

`urzua explain` on a path that is no file or directory, matches no glob and that no record cites is
now an error (exit 2) instead of an `ok` report with an empty `governing_records`; a rule id given
as a path says to use `--rule`. `urzua init` states what `check` does with records whose headers it
cannot read (nothing, until the type declares `required_fields`) and what to add, and writes
`exclude_files: [README.md]` for a proposed directory that holds a README.
