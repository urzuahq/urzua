---
Stable-Id: 01M6CXZV44D4GZWR1D3KT6PZ3J
Status: Planned
Phase: '0'
Track: governance-process
---
# 118 — Re-examine the 'no external adopters yet' premise now that a first real adopter corpus is the Phase 0 target

## What

Re-examine the premise that this tool has no external adopters, now that a first real adopter corpus
is the Phase 0 target (`MILE-102`). List every place that relies on it, decide which still hold, and
record the outcome. Every statement below was checked by grep against this branch:

- `AGENTS.md`, "Don't build speculative capability": "this tool has no external adopters yet whose
  behavior needs preserving".
- `rust/crates/urzua-core/src/config.rs` (doc comment near line 193): "no real external adopters yet
  whose behavior a hardcoded fallback would need to preserve".
- `ADR-44` (Accepted): "zero real external adopters" as the stated cost of dropping the default, and
  "imagined future adopters, not real ones" as the rationale for the shim it removed.
- `ADR-54` (Accepted): the removed migration notice "served a population of zero", and a future break
  "with real adopters behind it is a different question".
- `RFC-23` (Accepted): "this tool has no real external adopters yet to preserve".
- `RFC-38` (Draft): quotes the `config.rs` comment as the reason this is "the cheapest moment".
- `RFC-26` (Draft): "External adopters do not exist yet (AGENTS.md's 'no external adopters' premise)".

`ADR-33` and `ADR-61` also say "no adopter" but about pressure to migrate and about declared fields
respectively, not the premise itself, so they are not counted.

## Why

Several removals and non-compatibility decisions were justified by a population of zero. The Phase 0
target is a first real adopter corpus, so the premise is about to be false, and each dependent
decision should be re-read against a nonzero population before the next break is made on its strength.
The two Accepted ADRs and the Accepted RFC listed above are not edited: per `AGENTS.md` ("Never silently
rewrite an Accepted decision") any correction goes in a dated note in their Consequences or in a new
record that narrows them. The Draft RFCs, `AGENTS.md` (through `SPEC-19`) and the `config.rs` comment
can change in place.

## References

- `MILE-102` -- replace a hand-written linter outright; the first adopter this premise predates.
- `SPEC-19` -- governs `AGENTS.md`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** A first real adopter corpus is now the Phase 0 target while seven places still justify decisions by there being none; filed to re-read each, leaving Accepted records to dated notes. | **substantive** |
> | 2026-11-04 | Re-phased from Phase 0 to Phase 1; `Status` unchanged. Not needed to replace the adopter's linter. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the exit is the adopter's linter deleted and urzua's own repo clean in CI, and this work is on neither path, so it moves out of Phase 0. | **substantive** |
> | 2026-11-04 | Re-phased from Phase 1 to Phase 0 (pulled into Phase 0 as foundation work); `Status` unchanged. **Why:** Maintainer decision of 2026-11-04 defining Phase 0 as parity on a solid codebase (`SPEC-1`'s Phase 0 exit): it decides whether config and output changes may break callers, which gates every refactor in the foundation set, and adoption follows Phase 0. | **substantive** |
