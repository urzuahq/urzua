---
Status: Planned
Stable-Id: 01M1Y5JFS4DE9EE9HFVD4H4QHP
Phase: '0'
Track: section-checks
Implements: RFC-13
Blocked-on: MILE-5, RFC-33, MILE-98
---
# 7 — Resolve RFC-13 via a decisionless-status exemption

## What

A configured set of statuses (e.g. Withdrawn) exempt from required_sections and shape checks entirely.

## Why

A tombstone record for a vacated number never made a real decision -- requiring a Y-statement from it forces it to invent content that lies to satisfy the checker.

## The requirement runs both ways

Filed as an *exemption*: a `Withdrawn` ADR should not be asked for sections a live one must have. The
same primitive is wanted in the opposite direction, and naming only one half would build half of it.

- **Exempt by state** -- a `Withdrawn` record is not asked for required sections.
- **Require by state** -- a `spec` whose `Status` says the work shipped **must** carry `Embodiment`
  and `Realized-by`. Today `required_fields` is unconditional, so the only options are "required of
  every spec including the ones still being drafted" or "required of none". The second is what is
  configured, and it is why five specs sit `Draft` while their subject ships (`BUG-26`), examined by
  nothing (`BUG-40`).

One capability: **a rule whose applicability is conditioned on a record's declared state.** Whether
it reads as an exemption or a requirement is which way the condition points.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Reshaped by `ADR-53`, and **not** dissolved by it. **Why:** worth stating, because opt-in looks like it answers this and does not. Opt-in turns a rule off for a whole repository; this needs the rule **on** while exempting records in a declared state -- a `Withdrawn` ADR should not be asked for sections a live one must have. That is a rule *option*, not a rule level, so it lands with the function vocabulary rather than before it. | **substantive** |
> | 2026-09-17 | `Blocked-on` now names the blocking record rather than describing it. **Why:** it held a sentence (*'Milestone: Build the closed section-parser...'*) which is legal -- `Blocked-on` is a declared `narrative_field`, and prose is what those are for, since "blocked on a decision about X" is a real value no pointer expresses. Changed because in *these three* cases the prose described exactly one existing record, so naming it lets `narrative-field.stale` see the dependency and report when the target goes terminal. Not a defect being fixed; a narrative value replaced by a more useful one. | **structural** |
> | 2026-09-17 | Recorded that the same primitive is wanted in both directions. **Why:** filed as exempting `Withdrawn` records from requirements; `BUG-26` needs the opposite -- requiring `Embodiment`/`Realized-by` of a spec whose `Status` says the work shipped. `required_fields` is unconditional, so today's only choices are "required of every spec, including drafts" or "required of none", and the second is configured. Building the exemption alone would deliver half a capability. | **substantive** |
> | 2026-09-18 | `Blocked-on` now names `MILE-98`. **Why:** the declared document model had no milestone -- `MILE-4` was marked absorbed into `RFC-33` and the work moved into an RFC, so six records were blocked on something the plan did not track. Naming it makes the dependency resolvable, and `narrative-field.stale` can report when it moves. | **structural** |
