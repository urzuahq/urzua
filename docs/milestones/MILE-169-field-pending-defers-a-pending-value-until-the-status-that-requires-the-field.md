---
Stable-Id: 01M6K1Y9EKS4A75KJWY8ZK18EM
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: test:rust/crates/urzua-core/src/rules/field.rs#pending_is_legitimate_until_the_required_status_observed_failing@sha256:831e14ee29357f9fb914d54a33e3023296f63acc912ddd3298b0bfebc73a67be, test:rust/crates/urzua-core/src/rules/field.rs#without_the_option_pending_is_reported_at_every_status@sha256:3241786933aa39fb5a5a677b6cc6cf95682ab8dd06a20a123a7c6976ed20e2b2, test:rust/crates/urzua-core/src/rules/field.rs#the_option_is_refused_on_any_other_rule@sha256:f6c6d0fc1fb8dc45a38e205c63d193edea376e6132c3172dc4e73fea896814d8, code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-field-rules-run.yaml
---
# 169 — field.pending defers a pending value until the status that requires the field

## What

Give `field.pending` an opt-in `pending_until_required` option: a pending value in a field the type's `statuses.require` ties to certain statuses is not reported while the record's status is not one of them. A field no `require` entry names is judged as before.

## Why

A placeholder `Regression-test` ("not yet written", "planned: ...") is a legitimate state while a bug is `Open` and a real gap once it is `Fixed`. `status.required-field-missing` already flags only the `Fixed` ones, but `field.pending` read the same `statuses.pending` list with no status scoping and warned on about twenty `Open` bugs. Pending is a legitimate state until the record reaches the status that requires the field; the type already declares which status that is.

## Done means

- With the option declared `true`, a pending value in a required-at-status field is not reported before that status and is reported at it. — evidence: test:rust/crates/urzua-core/src/rules/field.rs#pending_is_legitimate_until_the_required_status_observed_failing@sha256:831e14ee29357f9fb914d54a33e3023296f63acc912ddd3298b0bfebc73a67be
- A skipped slot stays eligible and is counted unexamined, so it never disappears from the population. — evidence: test:rust/crates/urzua-core/src/rules/field.rs#pending_is_legitimate_until_the_required_status_observed_failing@sha256:831e14ee29357f9fb914d54a33e3023296f63acc912ddd3298b0bfebc73a67be
- A field no `require` entry names keeps its behaviour. — evidence: test:rust/crates/urzua-core/src/rules/field.rs#pending_is_legitimate_until_the_required_status_observed_failing@sha256:831e14ee29357f9fb914d54a33e3023296f63acc912ddd3298b0bfebc73a67be, test:rust/crates/urzua-core/src/rules/field.rs#without_the_option_pending_is_reported_at_every_status@sha256:3241786933aa39fb5a5a677b6cc6cf95682ab8dd06a20a123a7c6976ed20e2b2
- The option is refused on any other rule at load. — evidence: test:rust/crates/urzua-core/src/rules/field.rs#the_option_is_refused_on_any_other_rule@sha256:f6c6d0fc1fb8dc45a38e205c63d193edea376e6132c3172dc4e73fea896814d8
- This repository declares `pending_until_required` on `field.pending` and the placeholder patterns on its types, so `urzua check` exercises the option. — evidence: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-field-rules-run.yaml
- This repository's `bug` type declares `require: [{when: [Fixed], fields: [Regression-test]}]`. — waived: superseded-by-decision (ADR-84) -- ADR-84 retired the bug `Regression-test` field, so the `bug` type declares no `require` entry; the `adr` type's `require` on `Deciders` is what exercises the option here

The option is off by default. Making it the default would silently stop warning on every existing repository whose `require` already names a field it also holds as pending at an earlier status (a `Draft` ADR with `Deciders: TBD`), which is a behaviour change nobody asked for; the declaration is the type's, the opt-in is the rule's, as with `allow_extension_prefix` (`MILE-141`).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed and built: `field.pending` takes `pending_until_required` (`SPEC-2` 0.57, `SPEC-3` 0.45); refused on any other rule at load. Exercised by this repository's `bug` type. **Why:** the Done means above; `MILE-162` built the pattern entries this relies on. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #224: `field.pending` takes `pending_until_required`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
