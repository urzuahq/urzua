---
Status: Planned
Stable-Id: 01M1Y5JJECJSQBGTZSAH8WHYRP
Phase: '0'
Track: governance-process
Implements: RFC-7
Blocked-on: —
---
# 12 — Decide RFC-7: agent enforcement harness

## What

An audit trail independent of the editing agent's own compliance.

## Why

Still Draft; genuinely undecided.

## Done means

The parts of `RFC-7` that Phase 0 builds are recorded as Accepted ADRs (`ADR-69`, `ADR-70`, `ADR-71`), and each part left open states that it is deferred and why: the independent audit trail, which is not the editing agent's own compliance, is either decided in an ADR or listed as an open question on `RFC-7` naming its owner.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `ADR-69`, `ADR-70` and `ADR-71` are Accepted (maintainer decision, 2026-11-04) and record the decision for the parts of `RFC-7` that Phase 0 builds: the pre-write hook relaying `urzua`'s verdict (section 2 and the first bullet of section 3, `ADR-69`, built by `MILE-178`), drafting commands (the second bullet of section 3, `ADR-70`, `MILE-177`) and the layered gate (the third bullet, `ADR-71`, `MILE-179`, `MILE-180`). Not decided, so this milestone stays open and `Status` stays `Planned`: where an override is persisted beyond the hook's output, whom an unacknowledged block routes to, and whether other computed fields join `Embodiment` (`ADR-69`); section 4's scope-mismatch check, whose Phase 0 milestone is `MILE-142`, and section 5's mechanism-selection nudge; and `RFC-7`'s open questions on hook UX, the revision log and workflow-stage skips. Milestones carry no declared evidence field on `main` yet, so the three ADRs are cited here rather than in a `Realized-by`. `RFC-7`'s `Status` is unchanged. **Why:** the Done-when for a decide-milestone is that the decision exists for what is built, and `RFC-7` has parts in Phase 0 scope that no ADR yet covers. | **substantive** |
> | 2026-11-04 | Moved from Phase 2 to Phase 0. The enforcement harness is part of the tooling around a real adopter's records linter (an editor hook, git hooks, a CI integration and agent commands: `MILE-177` to `MILE-180`), and `ADR-69` to `ADR-71` draft the parts of `RFC-7` those milestones would build; this milestone is the decision they derive from. `Status` stays `Planned`. **Why:** Maintainer decision, 2026-11-04: Phase 0 parity includes the tooling around the linter, not only its checks. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `ADR-78` (Proposed) records the remaining decisions on `RFC-7`: the override log, the hook judging every rule on the edited record, section 4 satisfied by the existing `diff.source-and-records-mixed`, the section 5 nudge, a waiver for a skipped stage, and no revision record for a blocked edit. This milestone closes when `ADR-78` is accepted. `Status` stays `Planned`. **Why:** Maintainer decision, 2026-11-05: with `ADR-69` to `ADR-71` and `ADR-78`, every part of `RFC-7` is decided or deferred by name; closing waits for the maintainer to accept `ADR-78`. | **substantive** |
> | 2026-11-05 | `ADR-78` is Accepted (maintainer decision, 2026-11-05). Its parts that need building are filed: `MILE-187` (`check --stdin-path` and the hook relaying every finding), `MILE-188` (the committed override log and its unreviewed-override finding) and `MILE-189` (the `Realized-by` mechanism nudge); item 3 is the existing `diff.source-and-records-mixed` (`MILE-142`) and items 5 and 6 build nothing. This milestone closes, with `Realized-by` and `Status: Done`, in the pull request that completes the last of the three. `Status` stays `Planned`. **Why:** `ADR-78` notes that no milestone existed for these parts. | **substantive** |
