# 0022 — `urzua migrate schema`: report mode only

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:rust/crates/urzua-core/src/migrate.rs, code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-core/src/migrate.rs
> Date: 2026-09-06
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0008 (Accepted), ADR-0011 (Accepted)

## Context

SPEC-0001 names `migrate schema` as a field-rollout assistant with three tiers: report (what would
newly fail), waiver-assist (generate draft waivers a human still writes the `Reason` for), and apply
(delegate to `urzua fix` for whichever fields happen to be tool-writable). Only report has a real,
useful shape without new infrastructure: it reuses the same blank/placeholder/pending/present
classification `field.quality` already applies to declared required fields, against a field that
isn't declared yet.

## Decision

In the context of the three designed tiers, facing waiver-assist needing a real waiver-authoring flow
and apply needing `urzua fix` to know a field is tool-writable (true for almost none of them — most
newly-required fields, like `Reviewers`, need a human), we decided to ship **report only**:
`urzua migrate schema --report --field <Name>` lists every record lacking a real value for `Name`,
without touching config or any record. Read-only, same as `check`.

## Reversibility

Additive: a new pure function (`schema_report`) and a read-only CLI path. Waiver-assist and apply
remain named, not silently dropped — `MigrateTarget::Schema` already carries the shape for them, the
implementation doesn't exist yet.

## Consequences

- `urzua migrate schema` without `--report --field <Name>` errors with exactly what's missing,
  rather than doing nothing silently.
- Waiver-assist is still blocked on deciding whether a generated draft waiver (placeholder `Reason`,
  human edits before commit) is acceptable, or whether only a plain report (human writes the whole
  waiver) preserves the review property RFC-0011's waiver model depends on.
- Apply is blocked on the same eligibility question ADR-0019/0020 already answered for `urzua fix`
  Tier 1: this tier only ever applies to the rare newly-required field that's also mechanically
  derivable, which `Reviewers`-style fields structurally never are.

## References

- SPEC-0001 — the three-tier design this ADR ships one slice of.
- ADR-0011 — the waiver-as-record precedent waiver-assist would need to respect.
- ADR-0019/0020 — the eligibility question apply mode would delegate to.
