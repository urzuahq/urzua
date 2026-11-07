---
default: minor
closes: [MILE-213]
---

A record type declares its revision log's class vocabulary and columns in `revision_log`:
`classes`, `class_column`, `date_column` and `tool_class`. `revision-log.change-class-required`
accepts exactly the declared classes and lists them, and reads the class and date from the columns
named by header, in any position; a header that lacks a declared column or repeats it is one
finding with an `edit-section` action, and `revision-log.version-bump-required` reports that
record, or a merge-base log it cannot read, as unreadable. `urzua fix --apply` and
`--rebaseline` write the type's `tool_class`, laid out by the log's header. A type that declares
none of the keys behaves as before.
