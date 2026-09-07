# 77 — Decide what warrants a spec versus an ADR being sufficient

> Status: Planned
> Stable-Id: 01M1YX4KG8PJNP7J1NRGCYPA2E
> Phase: 1
> Track: governance-process
> Implements: —

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

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
