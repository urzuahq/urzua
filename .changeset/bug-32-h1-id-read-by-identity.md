---
default: patch
closes: [BUG-32]
---

`filename.title-consistency` no longer builds a record number out of the digits of a title's first word.
A title such as `# 2026-09-16 Notes` or `# v1.2 Release` is reported as carrying no record id instead of
claiming the number 20260916. A type's declared `identity` pattern decides the title form (its `prefix`
and a number); a type with no pattern accepts a leading `PREFIX-N` or a bare number. A number too large to
compare is reported as such.
