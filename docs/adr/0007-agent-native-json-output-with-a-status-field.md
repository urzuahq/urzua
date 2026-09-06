# 0007 — Every entry point emits JSON with a status field on stdout

> Status: Superseded
> Embodiment: Implemented
> Date: 2026-09-04
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: ADR-0023
> Derives-from: RFC-0003 (Accepted)

## Context

RFC-0003 proposed that every `urzua` entry point emit a structured JSON report on stdout — not just
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

- RFC-0003 — the proposal this decides.
- `docs/specs/0002-urzua-check.md` — the concrete output contract this ADR ratifies.
- `rust/crates/urzua-core/src/report.rs` — the implementation.
