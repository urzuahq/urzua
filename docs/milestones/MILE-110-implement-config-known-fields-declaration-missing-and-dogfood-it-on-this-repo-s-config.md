---
Stable-Id: 01M3643KM15RFHGG41K2QNHQRA
Status: Done
Phase: '1'
Track: schema-governance
Implements: RFC-43
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/declaration.rs#config_known_fields_declaration_missing@sha256:674e57b0dcb0ae8595f836a8c9797ff34f8716b6efc54bc79ced0a720c3267f7, test:rust/crates/urzua-core/src/rules/declaration.rs#a_type_with_no_known_fields_declared_is_a_missing_declaration_observed_failing@sha256:377acaa022cf93d5c34474568f74c2a750978863a6380c414dbda2dafb69dde6, test:rust/crates/urzua-core/src/rules/declaration.rs#a_type_declaring_known_fields_as_an_explicit_empty_list_is_not_missing@sha256:0475a5fad5b779ee457590af1ba67b79110b430882884cc86118d4585d8f5fd6, code:.urzua/config.yaml@sha256:653bb0f54ca6c2a3ac5009060305356e6ea00960928a6d8f50fb991d63314a5d, code:.github/workflows/checks.yml@sha256:a201f4e57f5f91a353ee0a7449831c0d03263200ac8e83a8f2a7a5b54fbe95a9
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

- The opt-in rule `config.known-fields-declaration-missing` reports a record type declaring no `known_fields` at all, mirroring `config.pointer-declaration-missing`'s shape and message. — evidence: code:rust/crates/urzua-core/src/rules/declaration.rs#config_known_fields_declaration_missing@sha256:674e57b0dcb0ae8595f836a8c9797ff34f8716b6efc54bc79ced0a720c3267f7, test:rust/crates/urzua-core/src/rules/declaration.rs#a_type_with_no_known_fields_declared_is_a_missing_declaration_observed_failing@sha256:377acaa022cf93d5c34474568f74c2a750978863a6380c414dbda2dafb69dde6
- A fixture covers a declaring and a non-declaring type. — evidence: test:rust/crates/urzua-core/src/rules/declaration.rs#a_type_declaring_known_fields_as_an_explicit_empty_list_is_not_missing@sha256:0475a5fad5b779ee457590af1ba67b79110b430882884cc86118d4585d8f5fd6, test:rust/crates/urzua-core/src/rules/declaration.rs#a_type_with_no_known_fields_declared_is_a_missing_declaration_observed_failing@sha256:377acaa022cf93d5c34474568f74c2a750978863a6380c414dbda2dafb69dde6
- The rule is enabled at `error` in this repository's own `.urzua/config.yaml`, which passes it. — evidence: code:.urzua/config.yaml@sha256:653bb0f54ca6c2a3ac5009060305356e6ea00960928a6d8f50fb991d63314a5d, code:.github/workflows/checks.yml@sha256:a201f4e57f5f91a353ee0a7449831c0d03263200ac8e83a8f2a7a5b54fbe95a9

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and implemented in one round. **Why:** `RFC-43`/`ADR-62` decided the design the same day the underlying question surfaced from a refuted code-review finding; scheduled and built together. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: .urzua/config.yaml@sha256:151830e5926d, .github/workflows/checks.yml@sha256:4afa5fe0db59 Reason: Pinned the files as they are now after checking each against the record it evidences: claim.rs and claim_field.rs still read a changeset's declared closes field as a claim (BUG-66), release-invariants.test.sh still plants a release-prep version bump and an unmatched level and asserts the guards fail, and checks.yml now runs it on the unattended path (BUG-92, BUG-93, BUG-96), new.rs and new_record.rs still take the configured header shape over a template (MILE-1), filename.rs still compares the filename's type prefix and padded number with the H1 (MILE-10), config.yaml still declares config.scope-matches-nothing and checks.yml runs the gate (MILE-106, MILE-110), and header.rs, narrative.rs, pointer.rs, field.rs, fix.rs, the Makefile and checks.yml still read the declared relation field names with relation_field_literals.rs failing on a restated literal (MILE-107, MILE-109). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 151830e5926d -> 0b59c6dfc8b9, `.github/workflows/checks.yml` 4afa5fe0db59 -> 3bd0a6795184. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. config.yaml: config.yaml gained Kind among the milestone type's known fields and a kind declaration (build and decision) under statuses.delivery.items (ADR-96 amendment of 2026-11-07), and every other declaration the record relies on is unchanged; checks.yml: the single rust job became parallel lint, build, three nextest test shards, four acceptance shards, an acceptance-gate and a result job (ADR-98), and every step the record relies on (fmt, clippy, deny, the records gate, the release guards, the rule table check, the gate action tests and harness status) is still run. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0b59c6dfc8b9 -> 0a00038a6388. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: this change declares status.done-items-shape (error), status.waived-item-stale (warn) and embodiment.claim-renewal-unattested (warn) and declares relation.cross-type-disallowed, section.shape and diff.source-and-records-mixed as off; every other declaration is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0a00038a6388 -> 3d51c2fac8b7. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: only the comment above the three rules declared off changed, to say they report `not-enabled` exactly as an undeclared rule does; no declaration changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/checks.yml` 3bd0a6795184 -> a201f4e57f5f. Reason: checks.yml: the only change since the pinned version is the four tool-download curl lines (cargo-deny, cargo-machete), which gained --retry 5 --retry-all-errors --retry-delay 5 and changed -sSLfo to -sSLf with a separate -o (#441); no step, job or check changed, so the evidence still holds **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 3d51c2fac8b7 -> 653bb0f54ca6. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: the only change is a new optional pending list (two placeholder patterns) under record_types::milestone statuses.delivery.items; nothing else in the file changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/declaration.rs#config_known_fields_declaration_missing` (lines 34-67) bbe1caf37d7d -> 674e57b0dcb0. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: declaration.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
