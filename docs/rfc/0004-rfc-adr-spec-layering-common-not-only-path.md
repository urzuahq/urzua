# 0004 — RFC → ADR → Spec layering: the common path, not the only path

> Status: Draft
> Date: 2026-07-30
> Author: (session author)
> Supersedes / Superseded-by: —

> **Split from RFC-0001, 2026-07-30.** This was originally §2 of
> `0001-unified-schema-and-layering.md`. RFC-0001 had grown to cover four separable decisions
> (core schema/profiles, this layering model, the `realized_by` claim graph, and the living-spec
> problem) with corrections bolted on in place rather than split out — a real signal it had become
> a dumping ground rather than one proposal. Split so each is independently acceptable/rejectable.
> Content is unchanged from the original §2, per this repo's own rule against silent rewrites.

## Summary

Propose that the natural sequence — proposal (RFC) discussed, then a decision made (ADR), then a
decision built (Spec) — is the schema's *common* path, made cheap via optional pointers
(`derives_from`, `implements`), but not the *only* representable path. A strict linear model doesn't
hold up against real usage, and the schema needs to represent that honestly rather than forcing
every decision through a pipeline it didn't actually take.

## Motivation

See RFC-0001's Motivation — the underlying schema-unification motivation applies here too. This
RFC's specific motivation is that a strict RFC→ADR→Spec pipeline would misrepresent real usage
patterns (see Proposal below), producing a schema that's clean in theory but false to how decisions
actually get made in practice.

## Proposal

The natural sequence — proposal (RFC) discussed, then a decision made (ADR), then a decision built
(Spec) — is the common case, and the schema should make that path cheap: an ADR can `derives_from`
an RFC, a Spec's `implements` field points at ADRs.

But it must not be the *only* representable path, because real precedent already contradicts a
strict linear model:

- **A real system can have no RFC concept at all** — a Spec's own `Status: Draft` already covers
  "not yet finished," and ADRs get authored directly without a prior RFC. Forcing every decision
  through an RFC stage first would be false to how a real, working system actually operates.
- **A decision can precede any spec entirely** (an ADR with `Embodiment: Inactive` — a negative
  decision, e.g. "we're not building X" — never produces a Spec, by design, and that's a valid
  terminal state, not a gap).
- **An RFC can be filed for something already built** — a filed RFC can be marked `Accepted` on
  migration precisely because "the proposal was discussed and built, spawning its own ADRs" — a
  normal terminal state, not a category error.

**Design implication:** `derives_from`/`implements` are optional pointers, not required stages.
The schema should support "just an ADR, no RFC" and "just a Spec, no formal ADR, `Status: Draft`
covers it" as first-class, not degraded, paths. What the *unified* schema buys isn't "force
everyone through the same pipeline" — it's "whichever subset of RFC/ADR/Spec a given org/repo
actually uses, the underlying validator, backfill engine, and drift-detector are the same code."

## Open questions

None specific to layering beyond what RFC-0001 and RFC-0005 (embodiment) already carry — the
optional-pointer model above is stated as a design implication, not left open.

## Non-goals

Does not attempt to design the paging/notification mechanism, dashboard/UI, or storage technology
— same scope boundary as RFC-0001. Does not re-litigate the core schema/profile shape (RFC-0001)
or the Embodiment/`realized_by` model (RFC-0005) — this RFC only concerns how records of different
types point at each other over time.

## References

- RFC-0001 — the core schema/profile model this layering proposal sits on top of
  (`derives_from`/`implements` are core-schema fields per RFC-0001 §1).
- RFC-0005 — Embodiment/`realized_by`, referenced above via the `Embodiment: Inactive` negative-
  decision example.
