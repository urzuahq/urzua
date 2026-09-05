# 0011 — A waiver is a first-class record, not an ignore list

> Status: Accepted
> Embodiment: Not started
> Date: 2026-09-04
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0015 (Accepted)

## Context

RFC-0015 proposed that a corpus record a reviewed exception to a rule as its own record type
(`waiver`) rather than a config-level ignore list — the ignore-list shape is exactly what RFC-0011
§5 and RFC-0013 already reject as ad-hoc, invisible, and unbounded. This decision was made before
any foreign corpus exists to force it under pressure, which is deliberate: the plan this project
follows explicitly schedules this ahead of testing against a corpus this project doesn't control.

## Decision

In the context of a validator that will eventually run against corpora with real, deliberately
accepted exceptions, facing the choice between a config-level ignore list and a reviewable record,
we decided that **a waiver is a record**: it names the rule and scope it covers, requires a
non-empty reason, and carries an optional `expires` date. No expiry means the exception is asserted
structural (the same shape as SPEC-0001's permanent content-scope ceiling); a stated expiry means
the waiver **reverts to blocking automatically** once passed, with no separate "expired" state
anyone has to notice or configure.

`check`'s output lists every waived finding — never omits it — with a `waived` field pointing at the
covering waiver record. `status` and `blocking` are computed from non-waived findings only.

## Reversibility

The record shape and the report field are both additive to the existing schema and output contract
— nothing existing has to change to add this. Cheap to build before any real waiver exists in this
corpus; the design cost was in deciding the shape, not in the migration.

## Consequences

- A `waiver` record type must be added to `.urzua/config.toml`'s `record_types` and to the schema
  the same way `adr`/`rfc`/`spec` are — no special-casing in the tool.
- `CheckReport`'s `Finding` type gains a `waived: Option<String>` field (the covering waiver's ID).
- Expiry checking needs a notion of "today," which the core schema so far has avoided needing —
  this is the first rule that reads wall-clock time rather than only corpus content.
- The role-enforcement question (does granting a waiver need the same Decider rigor as an ADR) stays
  open, deferred with RFC-0001 §1e's reciprocity work.

## References

- RFC-0015 — the proposal this decides.
- RFC-0011 §5, RFC-0013 — the ignore-list anti-pattern this ADR avoids.
- SPEC-0003 — "reported as skipped, never omitted," extended here to waived findings.
