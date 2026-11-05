---
default: patch
---

A record of a type that requires a revision log, with no revision-log section, is now a finding of
`revision-log.change-class-required`, with the `edit-section` `add` action naming the section to add.
Before, a record that lost its log left the rule's scope and the check stayed green. A type that sets
`revision_log: {required: false}` is unchanged: its records without a log stay unexamined.
