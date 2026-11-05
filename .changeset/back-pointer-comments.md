---
default: minor
closes: []
---

A source comment of the form `Implements: ADR-12` now counts as realization evidence for the record
it cites. Declare the new opt-in rule `embodiment.back-pointer-resolution` with a `back_pointer_paths`
list of path prefixes; `urzua check` and `urzua fix` then compute the cited record's `Embodiment`
as at least `Implemented`, and the rule reports a back-pointer that cites a record that does not
exist. Only whole-line comments are read, and a comment never outranks a `test:` locator or drift.
