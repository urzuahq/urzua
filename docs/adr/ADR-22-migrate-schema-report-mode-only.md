---
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/migrate.rs@sha256:e39131bfea8d07b33ffbece724c29f9a48f45d59a816f74a40955d93b68af89d, code:rust/crates/urzua-cli/src/commands/migrate.rs@sha256:0d031cec31058a8e148fc646d12dde0c0e9323a2068db9e41dba6a2c3431d448, test:rust/crates/urzua-core/src/migrate.rs@sha256:e39131bfea8d07b33ffbece724c29f9a48f45d59a816f74a40955d93b68af89d
Date: 2026-09-06
Accepted-on: 2026-09-06
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: RFC-8, ADR-11
---
# 22 — `urzua migrate schema`: report mode only

## Context

SPEC-1 names `migrate schema` as a field-rollout assistant with three tiers: report (what would
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
  waiver) preserves the review property RFC-11's waiver model depends on.
- Apply is blocked on the same eligibility question ADR-19/20 already answered for `urzua fix`
  Tier 1: this tier only ever applies to the rare newly-required field that's also mechanically
  derivable, which `Reviewers`-style fields structurally never are.

## References

- SPEC-1 — the three-tier design this ADR ships one slice of.
- ADR-11 — the waiver-as-record precedent waiver-assist would need to respect.
- ADR-19/20 — the eligibility question apply mode would delegate to.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | `Realized-by` locators pinned by content hash (`@sha256:`) after re-verifying each cited file against the Decision. **Why:** Re-verified against current code; maintainer decision 2026-11-04. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 99ef892a2. | **structural** |
> | 2026-11-04 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/migrate.rs` 1702fc44b4ce -> e39131bfea8d, `rust/crates/urzua-cli/src/commands/migrate.rs` 7442ba6c06b2 -> 0d031cec3105. Reason: Re-verified: migrate schema --report stays read-only with no apply path; core/cli changes are struct defaults, fixtures and header-error wording. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
