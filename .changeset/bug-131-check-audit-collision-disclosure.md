---
default: patch
---

`urzua check` and `urzua audit` build a shared record-identity index that silently keeps one of several
colliding records (first-seen-wins) for six independently opt-in reference-resolving rules
(`pointer.resolution`, `pointer.target-status`, `claim.status-agreement`,
`relation.supersession-reciprocity`, `relation.target-status-undeclared`, `narrative-field.stale`). If a
repository enabled any of those but left `identity.collision` off, a real identifier collision produced
no finding and no visible warning at all -- every consuming rule silently resolved against whichever
record happened to be inserted first.

Both commands now disclose every identity collision as a `Notice`, regardless of whether
`identity.collision` is enabled -- the same disclosure `urzua graph` already got in a prior release.
`identity.collision`'s own blocking `Finding` is unaffected: still exactly as opt-in as before. No
finding-count change on a corpus with no collisions.
