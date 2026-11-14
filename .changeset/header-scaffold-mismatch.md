---
default: minor
closes: []
---

Adds the opt-in rule `header.scaffold-mismatch`. Under `urzua check --base`, it compares the header of
each record the diff adds with the header `urzua new` would have written for it, and warns when they
differ: a field added, a field `new` fills (the id, author, date) missing, the fields reordered, or a
value `new` wrote (the initial status, the starting version) changed. A field `new` leaves null may be
filled in or left out. The header is re-rendered from the
run's own configuration and the type's template, with the record's own number, title, `Stable-Id`,
author and date, so a pull request sees a hand-written header before it merges. A record renamed from a
deleted one is not examined, and a run with no `--base` reports the rule `not-applicable`. The rule is
off unless `rules` declares it, so a repository that does not is unchanged.
