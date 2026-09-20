---
default: patch
---

`rules_executed` entries begin carrying a `population` — what a rule was eligible to examine and what
it judged, in a named unit. The six configuration rules and `type.record-outside-declared-dir` report
it first: a config rule now says `record-type 6/6` rather than `records_examined: 6`, which claimed a
quantity it was not counting. `records_examined` stays authoritative until every rule is converted.
