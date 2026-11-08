---
Stable-Id: 01M6R3NK4S1K3KKMGNV0RC498S
Status: Superseded
Embodiment: Deprecated
Realized-by: code:rust/crates/urzua-core/src/report.rs@sha256:f8332710ffd705380ccc6c22a075ba94cb38f3b703389f6c8d4e11ab3d88b4fb, spec:docs/specs/SPEC-2-urzua-check.md
Date: 2026-09-04
Accepted-on: 2026-09-05
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: ADR-23
Derives-from: RFC-3
---
# 7 — Every entry point emits JSON with a status field on stdout

## Context

RFC-3 proposed that every `urzua` entry point emit a structured JSON report on stdout — not just
as an opt-in flag, but as the primary contract every downstream consumer (a reviewing agent, a CI
gate, a human scripting against it) can act on without re-parsing prose. Phase 3 built exactly this
for `urzua check`: `status`, `files_examined`, `rules_executed` (with per-rule input population),
`scope`, `blocking`, and `findings`, selectable via `--format json` alongside a human-readable
default.

This ADR exists to decide the RFC now that the contract has been built and run against a real
corpus, rather than leaving a working, exercised design sitting in `Draft`.

## Decision

In the context of a CLI whose output needs to be consumed by both humans and agents/scripts, facing
the choice between a human-readable-only format and a structured one, we decided that **every entry
point emits a JSON report by default option (`--format json`), with the same shape `urzua check`
already ships**: `status` (`ok | findings-present | not-run` — never `ok` on a zero selection),
`files_examined`, `rules_executed` (each with the count of records it actually examined),
`scope`, `blocking`, and `findings` (each with `rule`, `severity`, `file`, `line`, `message`).

Accepting the field names as implemented (`snake_case`, matching `serde`'s default rather than the
RFC's illustrative `camelCase`) rather than re-deriving them — the implementation is the more
concrete artifact.

## Reversibility

The output contract is consumed by anything scripting against `urzua`. Changing field names or the
status enum after external consumers exist is a breaking change, not a patch — this is exactly the
kind of commitment that motivates a format/config stability policy (still open, tracked separately).
Cheap to revise before v0 ships to any external user.

## Consequences

- Every future command (`audit`, `migrate`, `export`/`import`, `doctor`) must emit this same shape,
  not invent its own — a second, incompatible JSON shape would defeat the point.
- `rules_executed`'s per-rule input population is now a hard requirement for every future rule, not
  an aspiration: a rule that can't report how many records it actually examined isn't done.
- The human-readable format is not contractual and may change formatting without a breaking-change
  concern; only the JSON shape is the contract.

## References

- RFC-3 — the proposal this decides.
- `docs/specs/SPEC-2-urzua-check.md` — the concrete output contract this ADR ratifies.
- `rust/crates/urzua-core/src/report.rs` — the implementation.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Pinned `rust/crates/urzua-core/src/report.rs` by content hash after re-verifying it against the Decision (attested by beauwilliams). **Why:** Re-verified for a Superseded record: report.rs changes are the refs and citation.issue-exists additions; the status-field JSON output this record decided is unchanged. | **structural** |
> | 2026-11-05 | `Embodiment` set to the declared sticky state `Deprecated` (was `Implemented`). Reason: drift review of 2026-11-04. This record is Superseded by `ADR-23` and its Decision no longer describes the code: it decides an opt-in `--format json` flag with human-readable output by default, while the CLI has no `--format` flag and prints only JSON. Re-baselining the pin on `report.rs` would certify a claim the code does not make. **Why:** a Superseded record's evidence is not re-verified; `Deprecated` is human-set and `embodiment.consistency` leaves it alone. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/report.rs` 81ca2e7df37e -> 206e1e3374db. Reason: report.rs gains the FindingSeverity::Info variant (serialized "info"), FindingSeverity::rank and an Implements: ADR-87 comment (ADR-87, MILE-194); the status field and the report shape this decision records are unchanged, so the evidence still holds **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 206e1e3374db -> 60b6889bf4eb. Reason: Read the diff of each pinned span since the old pin and the claim still holds. urzua-core/src/report.rs: added FindingSeverity::at_least, the deferred-by-scope population count and the untracked and pull-refs report fields, the JSON envelope (status values and exit codes) is as before. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 60b6889bf4eb -> f8332710ffd7. Reason: Read the diff of each re-pinned file. report.rs: added the AsOf and AsOfSourceKind types and an as_of field on CheckReport and FixReport; the existing fields, notices and exit_code contract are unchanged. Nothing this record relies on changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
