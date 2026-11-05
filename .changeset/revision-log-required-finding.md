---
default: patch
closes: [BUG-50]
---

A record of a type that declares `revision_log: {required: true}`, with no revision-log section, is now a finding of
`revision-log.change-class-required`, with the `edit-section` `add` action naming the section to add.
Before, a record that lost its log left the rule's scope and the check stayed green. A type that does not declare it is unchanged: its records without a log stay unexamined.
