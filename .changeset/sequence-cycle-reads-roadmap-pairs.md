---
default: minor
closes: [MILE-192]
---

`sequence.cycle` reads the pairs `roadmap.pairs` names as one graph, so `urzua check` reports a loop that uses an edge of each of two declared sequencing pairs, on every record in the loop, with the cycle's path and the pairs it crosses. This is the same graph `urzua roadmap` reads, so the two now agree. A new finding variant, `sequence-cycle-cross-pair`, marks it. A loop inside one pair is reported as before, an `acyclic` pair that `roadmap.pairs` does not name is still checked on its own, and a repository with no `roadmap.pairs` is unchanged.
