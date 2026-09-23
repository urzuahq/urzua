---
default: patch
---

`urzua graph` used the plain, non-collision-reporting record index — unlike `check`/`audit` (since
round 24), it never even computed whether two records shared an identifier, so an edge naming a
colliding identifier could silently point at the arbitrary winner with no way to tell. `graph` now
uses the same collision-aware index builder and discloses any collision found as a `Notice`
(`GraphReport`'s existing `notices` field), regardless of whether `identity.collision` is enabled —
this doesn't override the adopter's rule policy (`identity.collision`'s own blocking `Finding` stays
exactly as opt-in as before); it discloses that the engine's own shared computation had to resolve an
ambiguity, the same class of fix `BUG-125`/`RFC-45` shipped for `Population.unreadable()`.

No adopter-facing behavior change for a corpus with no identity collisions (this repository's own):
verified with the full test suite, clippy, `make ci`, and a real-corpus `check`/`graph` run — `check`
reports the same 70 findings, `graph` emits no new notices.
