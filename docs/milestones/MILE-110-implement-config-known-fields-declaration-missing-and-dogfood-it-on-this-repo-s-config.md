---
Stable-Id: 01M3643KM15RFHGG41K2QNHQRA
Status: Done
Phase: '1'
Track: schema-governance
Implements: RFC-43
Blocked-on: —
---
# 110 — Implement config.known-fields-declaration-missing and dogfood it on this repo's config

## What

`RFC-43`/`ADR-62`'s implementation: a new opt-in rule, `config.known-fields-declaration-missing`,
reporting a record type that declares no `known_fields` at all — mirroring
`config.pointer-declaration-missing`'s shape and message style exactly. Enabled in this repository's
own `.urzua/config.yaml` in the same change.

## Why

A round-20 code review flagged that a type with no `known_fields` gets no field-set governance at all
(`ADR-53`'s "declared, not voted" default, working correctly) — a real but already-decided design, not
a bug. The genuine question underneath it was whether a repository should be able to *require* the
declaration be made explicitly, the same lever `config.pointer-declaration-missing` already gives for
a different pair of fields. `RFC-43`/`ADR-62` decided yes, opt-in, dogfooded here immediately.

## Verification

Full test suite (224 tests in `urzua-core`, 2 new), `cargo fmt`, `clippy -D warnings`, `make ci`
(which runs `urzua check docs/` with the new rule enabled), and a real-corpus `check` run reporting the
same 70 findings before and after — this repository's six declared record types already declare
`known_fields` for every one of them, so enabling the rule here required no config fixes, only the
one-line addition to `rules:`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and implemented in one round. **Why:** `RFC-43`/`ADR-62` decided the design the same day the underlying question surfaced from a refuted code-review finding; scheduled and built together. | **substantive** |
