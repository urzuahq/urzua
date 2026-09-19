---
default: minor
---

A hyphenated type prefix now parses (`BUG-58`). `urzua init` emits one when two
directories share a last component, and `urzua new` derives the filename from
it -- so the tool wrote files it could not read back, and handed out the same
number forever. The number is now found rather than assumed to be the second
segment.

A two-digit pair in a slug is no longer mistaken for a date. `0013-80-20-rule.md`
is a record; only a four-digit year with a plausible month and day is a date.

New rule `type.dir-matches-nothing`: a declared record type whose `dir` holds no
records. `RFC-35` made `dir` mean that directory rather than that subtree, which
is unambiguous but unforgiving -- a `dir` one level off matches nothing, and
mixed with any working type that was silent. Found one in this project's own
config on the first run.

A `claim_paths` entry that exists but is empty no longer aborts the run. Git does
not track empty directories, so a repository following the documented
`claim_paths: [".changeset"]` pattern would have lost `check` entirely once a
release consumed the last fragment.

Two error messages still used TOML table syntax after the YAML move.
