---
default: minor
closes: [BUG-66]
---

A release fragment can now declare the records it closes in a one-line frontmatter key, and
`claim.status-agreement` checks that data instead of reading a sentence. Declare the key's name with
the new `closes` option (`closes: closes`) and write `closes: [BUG-60, BUG-61]`, a single id, or
`closes: []` for a change that closes nothing. Each listed id must name a record and that record must
be at a terminal status its own type declares; an open record, an id that names no record
(`claim.id-unresolved`) and a value that is not a one-line list of ids (`claim.field-malformed`, which
includes a block list and a list over several lines) are errors. The optional `strict: true` also
asks every fragment that has frontmatter for the key (`claim.field-missing`), which catches a
fragment that names a record without saying whether it closes it. A file with no frontmatter is not a
fragment: it is counted out of scope and named in a notice. A reference in the text is never read as
a claim by the new option, and the existing closing-verb reading keeps working unchanged, with a
notice that says how many claims it still found. Declaring neither option changes nothing.
