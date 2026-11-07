---
Stable-Id: 01M3643KM15RFHGG41K2QNHQRA
Status: Done
Phase: '1'
Track: schema-governance
Implements: RFC-43
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/declaration.rs#config_known_fields_declaration_missing@sha256:bbe1caf37d7d997564f711762caa6ca1e5a3ec1dd4419d43ec703b3c744f8d31, test:rust/crates/urzua-core/src/rules/declaration.rs#a_type_with_no_known_fields_declared_is_a_missing_declaration_observed_failing@sha256:377acaa022cf93d5c34474568f74c2a750978863a6380c414dbda2dafb69dde6, test:rust/crates/urzua-core/src/rules/declaration.rs#a_type_declaring_known_fields_as_an_explicit_empty_list_is_not_missing@sha256:0475a5fad5b779ee457590af1ba67b79110b430882884cc86118d4585d8f5fd6, code:.urzua/config.yaml, code:.github/workflows/checks.yml
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

## Done means

- The opt-in rule `config.known-fields-declaration-missing` reports a record type declaring no `known_fields` at all, mirroring `config.pointer-declaration-missing`'s shape and message. — evidence: code:rust/crates/urzua-core/src/rules/declaration.rs#config_known_fields_declaration_missing@sha256:bbe1caf37d7d997564f711762caa6ca1e5a3ec1dd4419d43ec703b3c744f8d31, test:rust/crates/urzua-core/src/rules/declaration.rs#a_type_with_no_known_fields_declared_is_a_missing_declaration_observed_failing@sha256:377acaa022cf93d5c34474568f74c2a750978863a6380c414dbda2dafb69dde6
- A fixture covers a declaring and a non-declaring type. — evidence: test:rust/crates/urzua-core/src/rules/declaration.rs#a_type_declaring_known_fields_as_an_explicit_empty_list_is_not_missing@sha256:0475a5fad5b779ee457590af1ba67b79110b430882884cc86118d4585d8f5fd6, test:rust/crates/urzua-core/src/rules/declaration.rs#a_type_with_no_known_fields_declared_is_a_missing_declaration_observed_failing@sha256:377acaa022cf93d5c34474568f74c2a750978863a6380c414dbda2dafb69dde6
- The rule is enabled at `error` in this repository's own `.urzua/config.yaml`, which passes it. — evidence: code:.urzua/config.yaml, code:.github/workflows/checks.yml

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and implemented in one round. **Why:** `RFC-43`/`ADR-62` decided the design the same day the underlying question surfaced from a refuted code-review finding; scheduled and built together. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
