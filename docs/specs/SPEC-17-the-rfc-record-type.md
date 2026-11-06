---
Stable-Id: 01M6R3NK5FRHCY6MQ6VB26JKFC
Version: '0.12'
Date: 2026-09-08
Status: Shipped
Embodiment: Verified
Realized-by: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs
Author: beauwilliams
Subject: 'The `rfc` record type -- schema, required fields, and lifecycle for proposals.'
Parent: —
Implements: ADR-10
---
# SPEC-17 — The `rfc` record type

## Purpose

`rfc` is one of the three founding record types, alongside `adr` and `spec` — a proposal, not yet a
decision. Where an ADR records a decision fact, an RFC records the case being made for one, staying
in `Draft`/`Discussion` until an ADR (`Derives-from: RFC-N`) either accepts or rejects it. Backfilled
alongside `SPEC-16` (adr) and `SPEC-18` (spec), for the same reason: the founding types never got the
schema-spec treatment `milestone`/`bug`/`waiver` already had (ADR-41).

## Schema

```yaml
record_types:
  rfc:
    dir: "docs/rfc"
    required_fields: ["Status", "Date", "Author"]
    header_shape: "yaml-frontmatter"
    known_fields: ["Stable-Id", "Accepted-on", "Supersedes / Superseded-by", "Amends", "Amended-by", "Amended-by-pending", "Implements", "Non-reference-ids"]
    statuses:
      values: ["Draft", "Discussion", "Accepted", "Rejected", "Superseded"]
      success: ["Accepted"]
      failure: ["Rejected", "Superseded"]
      implementable: ["Accepted"]
      initial: ["Draft"]
      transitions:
        Draft: ["Discussion", "Accepted", "Rejected"]
        Discussion: ["Draft", "Accepted", "Rejected"]
        Accepted: ["Superseded"]
        Rejected: ["Draft"]
      require:
        - {when: ["Accepted", "Superseded"], fields: ["Accepted-on"]}
      forbid:
        - {when: ["Draft", "Discussion", "Rejected"], fields: ["Accepted-on"]}
    sections:
      from: headings
      required:
        - {name: Summary}
        - {name: Motivation}
        - {name: Proposal}
    spec: "SPEC-17"
    pointer_fields: ["Implements", "Amends"]
    narrative_fields: []
```

`Status` is mechanically validated against `statuses`: `status.undeclared-value` reports a value outside the
five, and `status.transition-allowed` reports a move the graph does not allow (on a `--base` run, against
the merge base's copy). A proposal is filed `Draft` (`initial`), moves between `Draft` and `Discussion`,
and is `Accepted` or `Rejected` from either; an `Accepted` one can be `Superseded`. A `Rejected` proposal
can be reopened as a `Draft` (`RFC-30`), the only move out of a terminal value. `sections` requires
`Summary`, `Motivation` and `Proposal` on every RFC, the template's headings; none is exempt, since a
rejected proposal still made its case.

| Field | Values | Notes |
|---|---|---|
| `Status` | `Draft` \| `Discussion` \| `Accepted` \| `Rejected` \| `Superseded` | An RFC's own lifecycle is independent of whether an ADR has decided it yet — `Accepted`/`Rejected` here means the *proposal itself* reached that state, not that a governing ADR necessarily exists (`pointer.resolution` surfaces a Draft-target's status when an ADR derives from it, per RFC-12, but never judges it). |
| `Date` | `YYYY-MM-DD` | When proposed. |
| `Author` | a real identity, no `@` prefix | Resolved automatically by `urzua new`, same as `adr` -- including the form: no decorative `@` prefix, which nothing reads and which forces avoidable YAML quoting (BUG-18, SPEC-16). |
| `Stable-Id` | a ULID, optional | Assigned unconditionally by `urzua new rfc` (ADR-21: every type gets one); backfilled by `migrate ids` for records predating it. |
| `Supersedes / Superseded-by` | comma-separated or `—`, optional | Same reciprocity model as `adr`. |
| `Accepted-on` | `YYYY-MM-DD`, optional | The day the proposal was accepted, not the day it was written; never earlier than `Date` and well-formed (`date.field-order`, `date.field-valid`). Required at `Accepted` and `Superseded`, forbidden at `Draft`, `Discussion` and `Rejected`, through `statuses.require` and `statuses.forbid` (`SPEC-3`). |
| `Amended-by` | comma-separated, optional | The reciprocal end of `Amends`, in the repository's `amendment` relation pair (`relation_pairs`, `SPEC-3`): an RFC that names another in `Amends` must be named back here, and a claim from an RFC that is not yet accepted is a warning, an error once it is accepted without the echo. Only RFCs carry the pair. |
| `Amended-by-pending` | comma-separated, optional | Lets the amended RFC acknowledge a pending amendment without asserting it before the amending RFC is accepted (`acknowledged_by`). |
| `Amends` | comma-separated, optional | An RFC narrowing or correcting an earlier one without fully superseding it — distinct from `Supersedes`, which replaces outright. Declared as one of this type's `pointer_fields` entries (MILE-90/ADR-44) — previously never resolved by anything. |
| `Implements` | comma-separated, optional | Already-generic field (RFC-1) — points at whichever bug or decision this RFC's proposal addresses (e.g. `RFC-23`'s own `Implements: BUG-8`). Added to `known_fields` and declared as this type's other `pointer_fields` entry in the same pass (MILE-90/ADR-44) — previously in real use but never declared, silently tripping `header.field-set-consistency`. |
| `Non-reference-ids` | comma-separated exact ids, optional | Ids this RFC names without citing as a reference (an example, a retracted number): `citation.resolution` does not resolve or report a citation of them in this record only, and reports a listed id the RFC never cites. Named by the rule's `non_reference_field` option (SPEC-3). Not a pointer field. |

No `Deciders` field — an RFC is a proposal one person can author; deciding it is what the resulting
ADR's own `Deciders` field records.

## What's deliberately not built

- **A discovery-gate stage before Draft** — explored as an open question (MILE-35), not built.

## References

- ADR-10 — the decision this spec details: `adr`/`rfc`/`spec` as config-declared profiles of one
  unified core schema, not types the tool's source code hardcodes.
- RFC-1 — the proposal ADR-10 decided; the unified core-plus-profile schema `rfc` is an instance of.
- RFC-10 — the closed-header model.
- ADR-33/42 — the `yaml-frontmatter` migration this spec's `header_shape` reflects.
- ADR-41 — the spec-organization principle this backfill follows.
- ADR-43 — the `type.no-declared-spec` rule whose live finding for `rfc` this spec closes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial spec. **Why:** `rfc` — one of the three founding record types — never got a schema spec; found live via `type.no-declared-spec`'s own inventory check (ADR-43), alongside the same gap for `adr`/`spec`. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-08 | Added `Derives-from: RFC-1 (Accepted)`. **Why:** `rfc` is a founding type decided by RFC-1, not by any single ADR (unlike `milestone`/`bug`/`waiver`, each pointing at the ADR that decided them) -- this spec cited RFC-1 in prose and References but never backlinked it in the header, the same pointer every other type declares. | **substantive** |
> | 2026-09-08 | Corrected: replaced `Derives-from: RFC-1` with `Implements: ADR-10`. **Why:** the previous entry's premise was wrong -- `ADR-10` is exactly the single decision record `milestone`/`bug`/`waiver`'s own specs each point at (`Implements: ADR-N`), just missed when this spec was first written; `rfc` is not an exception to that pattern after all. RFC-1 stays cited in References as the proposal ADR-10 decided. | **substantive** |
> | 2026-09-08 | Added `Stable-Id` to `known_fields`. **Why:** found live creating the first `rfc` records via `urzua new rfc` (RFC-18/19/20) -- `render_synthetic_yaml` assigns every type a `Stable-Id` unconditionally (ADR-21), but no prior `rfc` had ever been created through the tool, so this gap sat unexercised until now. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`; also corrected `Parent` from `SPEC-1` to `—` (`BUG-10`): this spec's real lineage is already stated via its own `Implements`/`Derives-from`, not a narrowing of `SPEC-1`'s v0-CLI scope. | **structural** |
> | 2026-09-09 | Added `Implements` to `known_fields`, declared `pointer_fields = ["Implements", "Amends"]`/`narrative_fields = []` in config (MILE-90/ADR-44). **Why:** `RFC-23`'s own header (`Implements: BUG-8`) had been silently tripping `header.field-set-consistency` since it was written -- `rfc` never declared `Implements` as known despite already using it in practice. `Amends` had also never been resolved by anything; declaring it a `pointer_fields` entry makes it real. | **substantive** |
> | 2026-09-11 | Bumped to `0.8`. **Why:** `Author`'s row stated the resolution mechanism ("same as `adr`") but not the written form, so BUG-18's no-`@` convention wasn't actually inherited by a reader of this spec alone -- it only looked like it was. Stated explicitly; this type's own `Author` values were backfilled in the same change. | **substantive** |
> | 2026-09-17 | This type's `[record_types.*]` declaration re-rendered in YAML (`ADR-52`, shipped in the same change). The same declaration, same keys, same values -- only the syntax differs. | **structural** |
> | 2026-11-04 | Bumped to `0.9`. Added the optional `Non-reference-ids` field to `known_fields`: the exact ids a record names without citing as a reference (`BUG-190`, `ADR-75`). **Why:** `RFC-5` writes an example id from another corpus that a real record could otherwise capture. | **substantive** |
> | 2026-11-05 | Bumped to `0.10`. Added `Realized-by` naming the implementing code and the tests that exercise this spec: `an_rfc_follows_its_declared_lifecycle_and_vocabulary`, `an_rfc_amending_another_must_be_echoed_by_the_amended_rfc`. **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`; the tests run this repository's own `.urzua/config.yaml`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Status changed from Accepted to Shipped and bumped to `0.11`. **Why:** maintainer decision of 2026-11-05 to ship the specs whose implementation is verified; `Embodiment: Verified` with `Realized-by` naming the implementing code and tests. | **structural** |
> | 2026-11-05 | Bumped to `0.12`. The schema excerpt now shows the type as configured: `Accepted-on`, `Amended-by` and `Amended-by-pending` among the known fields, the `statuses` declaration (`initial`, `transitions` including the `Rejected` to `Draft` reopen of `RFC-30`, and `require`/`forbid` for `Accepted-on`) and the required `Summary`, `Motivation` and `Proposal` sections. Rows are added for the three fields, and "Mechanical `Status` enum validation: not built" is removed, since `status.undeclared-value` and `status.transition-allowed` run. **Why:** the spec described an engine and a config that had since moved on, so a reader could not tell that the reopen is allowed or that the acceptance date is checked by status. | **substantive** |
