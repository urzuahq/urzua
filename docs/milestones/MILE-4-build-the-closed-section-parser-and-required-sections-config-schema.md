---
Status: Planned
Stable-Id: 01M1Y5JE63FXMT7PV7QJQN7BXV
Phase: '0'
Track: section-checks
Implements: RFC-17
Blocked-on: RFC-33
---
# 4 — Build the closed section-parser and required_sections config schema

## What

A shared, closed function locating every ## section's body, mirroring header.rs's closed-header model, plus a required_sections config schema with a composable content shape per section.

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
