---
Status: Planned
Stable-Id: 01M1Y5JE63FXMT7PV7QJQN7BXV
Phase: '0'
Track: section-checks
Implements: RFC-17
Blocked-on: RFC-33, MILE-98
---
# 4 — Require sections per type, as a rule over the declared document model

## What

A rule that a record type's declared sections are present, and that each carries the content shape
its type expects.

**Locating sections is no longer part of this.** That is `MILE-98`'s `sections.from`, with `depth` and
`items`. This milestone is the policy half only: *which* sections a type must have, and *what* must be
true of their contents.

## Why

check validates header fields today but nothing validates what's inside a required section's prose -- SPEC-2 names this rule category and it has never been built.

## Evidence from MILE-51 (2026-09-16)

Run against `npryce/adr-tools`, whose records carry their metadata in sections — `## Status`,
`## Context`, `## Decision`, `## Consequences` — and nothing else. `check` reads none of it: 9 errors
about a missing header, and no rule that looks at a section. This milestone is gap 4 of the five
`MILE-51` names.

One question to settle before building, surfaced by that run: `ADR-8`'s closed-header model reports
*unknown keys*. Applied to sections, that flags every `## Alternatives` or `## Notes` a foreign corpus
carries — flooding the corpora this is for. `ADR-8` already names the phasing (*"not yet enforced as
an error by default -- that phasing is Phase 6 scope"*) and the trigger (*"once there's a real
adopter's config to test that transition against"*). That adopter now exists, so the question is
answerable rather than open.

Also worth narrowing when this is picked up: this milestone's *"composable content shape per
section"* is `MILE-5`'s y-statement work and is not needed to check a Nygard corpus. Presence alone
closes gap 4.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Absorbed into `RFC-33`'s declared document model. **Why:** this was scoped as a standalone section parser plus a `required_sections` schema. Under `ADR-53` those are two different layers: locating sections is `sections.from` in the document model (shape, `RFC-34`), and requiring them is a rule (policy). Building it as one thing would rebuild the conflation `ADR-53` exists to remove. The MADR paper test also already moved the target -- `sections.from: h2` is too flat, and `depth`/`items` are needed. | **substantive** |
> | 2026-09-18 | `Blocked-on` now names `MILE-98`. **Why:** the declared document model had no milestone -- `MILE-4` was marked absorbed into `RFC-33` and the work moved into an RFC, so six records were blocked on something the plan did not track. Naming it makes the dependency resolvable, and `narrative-field.stale` can report when it moves. | **structural** |
> | 2026-09-19 | Re-scoped: the section *parser* moves to `MILE-98`, and this keeps `required_sections` as a rule. **Why:** filed as one deliverable -- a parser plus a schema -- which `ADR-53` splits in two. Locating a `##` block is shape and may ship as a primitive (`RFC-34`); requiring one is governance and must be declared. Marked *absorbed into `RFC-33`* on 2026-09-17, which was half right and left this record a duplicate of one third of `MILE-98`. Narrowed instead of closed, because the policy half is real work that nothing else owns. | **substantive** |
