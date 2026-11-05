---
Status: Done
Stable-Id: 01M1Y5JFS4DE9EE9HFVD4H4QHP
Phase: '0'
Track: section-checks
Implements: RFC-13
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/section.rs, code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/section_rules_integration.rs, test:rust/crates/urzua-cli/tests/spec_shipped_integration.rs
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

## Done means

A type declares statuses under `sections.exempt_statuses` whose records `section.required` and `section.shape` treat as out of scope, a planted record in an exempt status is not reported and one in another status is, and an exemption that matches no record is reported rather than silently doing nothing; the require-by-state half (a spec whose `Status` says the work shipped must carry `Embodiment` and `Realized-by`) is declared in config and a fixture shows an unconditional-field gap now reported; the capability is exercised in urzua's own `.urzua/config.yaml` and records.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Reshaped by `ADR-53`, and **not** dissolved by it. **Why:** worth stating, because opt-in looks like it answers this and does not. Opt-in turns a rule off for a whole repository; this needs the rule **on** while exempting records in a declared state -- a `Withdrawn` ADR should not be asked for sections a live one must have. That is a rule *option*, not a rule level, so it lands with the function vocabulary rather than before it. | **substantive** |
> | 2026-09-17 | `Blocked-on` now names the blocking record rather than describing it. **Why:** it held a sentence (*'Milestone: Build the closed section-parser...'*) which is legal -- `Blocked-on` is a declared `narrative_field`, and prose is what those are for, since "blocked on a decision about X" is a real value no pointer expresses. Changed because in *these three* cases the prose described exactly one existing record, so naming it lets `narrative-field.stale` see the dependency and report when the target goes terminal. Not a defect being fixed; a narrative value replaced by a more useful one. | **structural** |
> | 2026-09-17 | Recorded that the same primitive is wanted in both directions. **Why:** filed as exempting `Withdrawn` records from requirements; `BUG-26` needs the opposite -- requiring `Embodiment`/`Realized-by` of a spec whose `Status` says the work shipped. `required_fields` is unconditional, so today's only choices are "required of every spec, including drafts" or "required of none", and the second is configured. Building the exemption alone would deliver half a capability. | **substantive** |
> | 2026-09-18 | `Blocked-on` now names `MILE-98`. **Why:** the declared document model had no milestone -- `MILE-4` was marked absorbed into `RFC-33` and the work moved into an RFC, so six records were blocked on something the plan did not track. Naming it makes the dependency resolvable, and `narrative-field.stale` can report when it moves. | **structural** |
> | 2026-11-02 | Unblocked: `RFC-33` was Accepted; maintainer decision 2026-11-02. `Blocked-on` now reads `MILE-5, MILE-98`; `Status` stays `Planned`. **Why:** `narrative-field.stale` flagged the reference once `RFC-33` reached a terminal status, and the maintainer decided the dependency no longer holds. `MILE-5, MILE-98` is still open and stays named. | **substantive** |
> | 2026-11-04 | Built the exemption half: `exempt_statuses` under a type's declared `sections` lists statuses whose records `section.required` and `section.shape` count as out of scope (`SPEC-3`). `Status` stays `Planned`; the require-by-state half (`Embodiment`/`Realized-by` of a shipped spec) is not built and stays this milestone's open work. **Why:** the narrowest reading of where the set lives, since status vocabulary is another change's territory (`MILE-132`): it is read from the declaration, matched exactly against the type's declared status field, and a record whose header cannot be read is counted unreadable rather than assumed exempt. Declaring it without `required` fails the config load. | **substantive** |
> | 2026-11-04 | `Blocked-on` now reads `MILE-98`. **Why:** `narrative-field.stale` flagged a blocker that reached `Done`, and the delivery backfill of `MILE-183` set it; the remaining blocker is still open. `Status` unchanged. | **substantive** |
> | 2026-11-04 | Evidence so far, no `Realized-by` yet. **Why:** the exemption half is built (`sections.exempt_statuses`, `rules/section.rs`, `section_rules_integration.rs`, merged in #172); the require-by-state half (`Embodiment` and `Realized-by` required of a shipped spec) is not built. `Status` stays Planned until it is. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-04 | `Blocked-on` cleared: `MILE-98` is Done. `Status` stays Planned. The require-by-state half is now buildable as a declaration with no new code: `statuses.require` (`MILE-132`, `MILE-137`) already ties a field to the statuses that need it, and `bug` uses it for `Regression-test`. Declaring `require: [{when: [Accepted], fields: [Embodiment, Realized-by]}]` on the `spec` type was tried against this repository and reports 28 findings across the 14 Accepted specs that carry neither field; it was not kept. **Why:** two things stand between the try and a Done milestone. The `spec` vocabulary has only `Draft` and `Accepted`, so `Accepted` cannot tell a spec whose work shipped from one that is only agreed, and the declaration needs a status that says *shipped* (a vocabulary decision for the maintainer). And the 14 specs need their `Embodiment` and `Realized-by` backfilled before the rule can be enabled at `error`. | **substantive** |
> | 2026-11-04 | `Status` `Planned` -> `Done`; `Realized-by` set. **Why:** the require-by-state half is built as configuration alone, with no engine change: this repository's `spec` type declares `Shipped` after `Accepted` and `statuses.require` for `Embodiment` and `Realized-by`, and `spec_shipped_integration.rs` shows a `Shipped` spec without them reported by `status.required-field-missing` while `Draft` and `Accepted` specs are not. Together with the exemption half (`sections.exempt_statuses`, in use on `adr` and `milestone`), that meets Done means. No spec is `Shipped` yet; moving one is the maintainer's call (`SPEC-18`). | **substantive** |
