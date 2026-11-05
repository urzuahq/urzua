---
default: minor
---

A record type can declare `statuses.blocked`, the non-terminal statuses a person writes to say a
record is waiting. The new opt-in rule `sequence.status-agreement` holds the written `Status` to the
derived `blocked` value: a record is blocked when any record it depends on, over every `acyclic`
relation pair, is not in a status its own type declares `success`. It reports an error for a written
blocked status while nothing blocks the record, and for a blocked record that states an
`implementable` status. Enabling the rule with no `acyclic` pair or no type declaring `blocked` fails
the config load. Existing configs are unchanged.
