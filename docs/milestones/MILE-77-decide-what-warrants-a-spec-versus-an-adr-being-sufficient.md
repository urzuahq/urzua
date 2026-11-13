---
Status: Done
Stable-Id: 01M1YX4KG8PJNP7J1NRGCYPA2E
Phase: '1'
Track: governance-process
Kind: decision
Implements: ADR-41
Blocked-on: —
Realized-by: spec:docs/adr/ADR-41-specs-are-organized-by-feature-area-decided-editorially.md, spec:docs/specs/SPEC-8-urzua-fix.md, spec:docs/specs/SPEC-9-the-bug-record-type.md, spec:docs/specs/SPEC-10-the-waiver-record-type.md, spec:docs/specs/SPEC-11-urzua-audit.md, spec:docs/specs/SPEC-12-urzua-new.md, spec:docs/specs/SPEC-13-urzua-explain-and-urzua-graph.md, spec:docs/specs/SPEC-14-urzua-migrate.md, spec:docs/specs/SPEC-15-urzua-doctor.md
---
# 77 — Decide what warrants a spec versus an ADR being sufficient

## What

Decide the actual criterion for "this needs a spec" versus "an ADR documenting the decision is
sufficient" — there isn't one today, only precedent that's already inconsistent. Once decided,
backfill specs for whatever falls inside the criterion (candidates found live: `bug` and `waiver`
as record types; `urzua new`, `fix`, `audit`, `migrate ids`/`migrate schema`, and `explain`/`graph`
as command surfaces). Per ADR-14's amendment, a backfilled spec must be a complete, replayable
specification of its subject as it exists today, not a shipped-feature note.

## Why

Found live, reviewing the spec set against the ADR set: `milestone` got both an ADR (34) and a spec
(SPEC-6, "the buildable units now have their own specs") — but `bug` (ADR-35) and `waiver` (ADR-11)
are the same shape of decision (a configured record type) and got no matching spec. Symmetrically,
`check`/config/`init` have specs (SPEC-2/3/5) while `new`/`fix`/`audit`/`migrate`/`explain`/`graph`
are real, multi-part command surfaces documented only in ADRs. Backfilling without first naming the
rule would just add more inconsistent precedent on top of the inconsistent precedent already there.

## Done means

- `ADR-41` records the criterion for a spec versus an ADR being sufficient: a spec groups a coherent feature area, decided editorially, not by a count. — evidence: spec:docs/adr/ADR-41-specs-are-organized-by-feature-area-decided-editorially.md
- The areas `ADR-41` named have complete replayable specs: `SPEC-8` (`fix`), `SPEC-9` (`bug`), `SPEC-10` (`waiver`), `SPEC-11` (`audit`), `SPEC-12` (`new`), `SPEC-13` (`explain` and `graph`), `SPEC-14` (`migrate`) and `SPEC-15` (`doctor`). — evidence: spec:docs/specs/SPEC-8-urzua-fix.md, spec:docs/specs/SPEC-9-the-bug-record-type.md, spec:docs/specs/SPEC-10-the-waiver-record-type.md, spec:docs/specs/SPEC-11-urzua-audit.md, spec:docs/specs/SPEC-12-urzua-new.md, spec:docs/specs/SPEC-13-urzua-explain-and-urzua-graph.md, spec:docs/specs/SPEC-14-urzua-migrate.md, spec:docs/specs/SPEC-15-urzua-doctor.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Resolved by editorial judgment, not a mechanical formula. A first attempt at a mechanical trigger ("a second ADR amendment promotes the subject to a spec") was drafted and explicitly rejected -- a spec groups a coherent *feature area* together (the same call that produced SPEC-6 for `milestone`, and SPEC-3 already bundling `doctor` before this session), not a count of anything. Applied directly: wrote SPEC-8 (`fix`), SPEC-9 (`bug`), SPEC-10 (`waiver`), SPEC-11 (`audit`), SPEC-12 (`new`), SPEC-13 (`explain`/`graph`, bundled as one feature area), SPEC-14 (`migrate ids`/`migrate schema`, bundled), and split `doctor` out of SPEC-3 into its own SPEC-15 (a real, standalone feature area that had been folded into configuration's spec by default, not by a deliberate call). **Why:** these areas were genuinely coherent, buildable functional units documented only as ADRs, the same gap `milestone`'s SPEC-6 had already closed for itself; `doctor` needed the opposite move, since it had been bundled into the wrong spec rather than left with none. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | `Kind: decision` added. **Why:** maintainer decision of 2026-11-07 (`ADR-96` amendment, milestone kinds): this milestone records a decision, so it is judged against a decided record (an Accepted decision or a Shipped spec) rather than a built tier. Its `Status` is computed from its Done means items by `urzua fix`, not set by hand. | **structural** |
