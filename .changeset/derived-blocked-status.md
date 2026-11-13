---
default: minor
closes: [MILE-144]
---

`blocked` is a derived value. A record type's `statuses` can declare `progress`, the statuses that
claim work has advanced (a type lists `Done` there explicitly), and a top-level `roadmap.pairs`
names the sequencing pairs that count as dependencies, so a containment pair declared `acyclic`
blocks nothing. The new opt-in rule `sequence.status-agreement` reports a blocked record that states
a `progress` status. A record is blocked when any record it depends on is not in a status its own
type declares `success`, which includes a dependency in a `failure` status, with no status, or of a
type that declares no `success`. A type can still declare `statuses.blocked`, the written statuses
that mean waiting, to have a written one checked both ways. A blocked value that cannot be computed
(a dependency that resolves to nothing, or a record whose header cannot be read) is counted
unreadable and not guessed. Existing configs are unchanged.
