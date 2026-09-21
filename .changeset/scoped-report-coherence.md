---
default: patch
---

A scoped `check` no longer reports `status: not-run` beside `blocking: true`. A config finding
survives every scope filter, so scoping to a path holding no records produced a report claiming both
that nothing ran and that a blocking error was found, exiting 2 where 1 was correct.

`records_read_by_any_rule` is now counted over the same set as `files_examined`. Rules run against the
whole corpus so a reference resolves outside the scope, but counting those records here put the two
numbers on different denominators — a scoped run reported reading four records while examining one.
