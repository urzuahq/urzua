---
Stable-Id: 01M6DEN8ZYA5Z4DVW6E7SA10F3
Status: Done
Phase: '0'
Track: schema-governance
Realized-by: code:rust/crates/urzua-core/src/status_model.rs@sha256:ca68bc6df9e89c132363f95d1f950fcd481cd985a368d293f8884c2a4d9063cb, code:rust/crates/urzua-core/src/rules/status.rs@sha256:ffb14887aa68074a6d28ae9f9b8aa5713735811571e2ea803138f97b867cfcb2, test:rust/crates/urzua-cli/tests/check_integration.rs#declared_statuses_are_enforced_end_to_end@sha256:a907e0b1e48e96373c41f5ff7abfe722c66924e2d53731b825d18d35f491deb2, test:rust/crates/urzua-core/src/rules/claim.rs#a_claim_against_a_declared_failure_terminal_is_not_an_error_observed_failing@sha256:db367747e84e82fcdefd797396e87e3b332342e25e97ae8f71423862787e6308, test:rust/crates/urzua-core/src/rules/narrative.rs#narrative_field_stale_reads_the_declared_terminals_observed_failing@sha256:90c542edfa84fdf8c0d9c15fd4a4cc96a6e6d3811d5564525545ab2d9cc504f9
---
# 132 — Declared state machines per record type

## What

A record type declares its status field as a state machine in config: the states, which are initial
and which terminal, the allowed transitions, and the fields or sections each state requires. Rules
refer to the declaration ("terminal", "initial") instead of copying status lists, and the engine
reports:

- a status value the type does not declare;
- a field a state requires but the record lacks, including Deciders on an accepted record;
- a field a state forbids but the record carries (for example an acceptance date on a record that is
  not accepted).

This milestone is the status vocabulary and the status-dependent required and forbidden fields, and
stays Phase 0. Declared transitions, unreachable states, the derived `blocked` value and computed
lifecycles such as the embodiment tier are split out to `MILE-144` (Phase 1).

## Why

No engine code validates a status value: `Error::UnknownStatus` exists in `lib.rs` and nothing
produces it. Each type's vocabulary lives only in its spec's prose, and the set of finished statuses
is copied by hand into rule options, where the copies already disagree (`BUG-178`). Records already
use values no spec declares (`Not a bug` on two bugs), the embodiment tier `Inactive` is declared but
cannot be produced, and status-dependent requirements (a real regression test on a Fixed bug, a
blocker on a Blocked milestone) are prose that nothing checks. Which statuses a type has is
repository configuration, not engine policy: this repository's bug workflow is one configuration of
the primitive.

## Done means

- A type declares its statuses in config and `check` reports an undeclared status value. — evidence: code:rust/crates/urzua-core/src/status_model.rs@sha256:ca68bc6df9e89c132363f95d1f950fcd481cd985a368d293f8884c2a4d9063cb, code:rust/crates/urzua-core/src/rules/status.rs@sha256:ffb14887aa68074a6d28ae9f9b8aa5713735811571e2ea803138f97b867cfcb2, test:rust/crates/urzua-cli/tests/check_integration.rs#declared_statuses_are_enforced_end_to_end@sha256:a907e0b1e48e96373c41f5ff7abfe722c66924e2d53731b825d18d35f491deb2
- A state can declare required and forbidden fields, and a record violating either is reported: a planted accepted record without `Deciders` fails while a Proposed one passes. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#declared_statuses_are_enforced_end_to_end@sha256:a907e0b1e48e96373c41f5ff7abfe722c66924e2d53731b825d18d35f491deb2
- The hand-copied terminal and closed status lists are read from the declaration instead (`BUG-178`). — evidence: test:rust/crates/urzua-core/src/rules/claim.rs#a_claim_against_a_declared_failure_terminal_is_not_an_error_observed_failing@sha256:db367747e84e82fcdefd797396e87e3b332342e25e97ae8f71423862787e6308, test:rust/crates/urzua-core/src/rules/narrative.rs#narrative_field_stale_reads_the_declared_terminals_observed_failing@sha256:90c542edfa84fdf8c0d9c15fd4a4cc96a6e6d3811d5564525545ab2d9cc504f9

## References

- `BUG-178` -- the duplicated, disagreeing status lists.
- `ADR-64` -- structured in, structured out; derived values are not hand-written.
- `RFC-48` -- the derived `blocked` value, now carried by `MILE-144`.
- `MILE-144` -- the split-out half: transitions, unreachable states and the derived `blocked` value.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** A gap the Phase 0 roadmap found with no record. | **substantive** |
> | 2026-11-02 | Re-scoped from declared status values to declared state machines. **Why:** The implicit state machine inventory found twelve undeclared lifecycles and disagreeing copies; author asked for declared state machines. | **substantive** |
> | 2026-11-04 | Split: status vocabulary and status-dependent required and forbidden fields (including Deciders on accepted) stay here at Phase 0; transitions, unreachable states, the derived `blocked` value and computed lifecycles moved to `MILE-144` at Phase 1, which this record now points at. Added Done-means. `Status` unchanged. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter needs a status enum and per-status required and forbidden fields, and does not need transitions or derived values. | **substantive** |
> | 2026-11-04 | Phase 0 half built: a type declares `statuses` in config (values, success and failure terminals, fields required or forbidden at a status, its own pending vocabulary); `status.undeclared-value`, `status.required-field-missing` and `status.forbidden-field-present` report against it; `narrative-field.stale` and `claim.status-agreement` read the declared terminals. This repository declares the vocabulary for all five of its types. `Status` unchanged. **Why:** the Phase 0 exit needs the status enum and per-status required and forbidden fields (`SPEC-3`). | **substantive** |
> | 2026-11-04 | The status vocabulary gains an optional `implementable` list beside `success` and `failure` (`SPEC-3` 0.10). `Status` unchanged. **Why:** a gate needs a per-type declaration of the statuses at which implementation is permitted (`MILE-133`). | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #173: a type declares `statuses`; `status.undeclared-value`, `status.required-field-missing` and `status.forbidden-field-present` read it. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/status_model.rs@sha256:744b341a578a, rust/crates/urzua-core/src/rules/status.rs@sha256:a5311e755ba5 Reason: Pinned the files as they are now after checking each against the record it evidences: status_model.rs and rules/status.rs still declare and judge the per-type state machine (MILE-132), acceptance.rs and the corpus-h gate expectation still hold the held-source case (MILE-133), rules/index.rs still compares generated indexes with the records (MILE-139), config.rs still reads field_levels and gate.rs still derives the report status (MILE-140), refs_snapshot.rs and commands/refs.rs still keep and refresh the cited-number snapshot (MILE-143), the corpus-j expectations still plant an unlisted edge and an unreachable status (MILE-144), config.rs still reads fail_on (MILE-145), claim_field_integration.rs still carries a Verifies back-pointer comment of the kind the milestone's own-corpus item names (MILE-146), main.rs and discovery.rs still read the full back-pointer id list and the decision-gate sources (MILE-147), and embodiment.rs still reads sticky_states (MILE-148). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/status_model.rs` 744b341a578a -> 9cbbc69bdcdc, `rust/crates/urzua-core/src/rules/status.rs` a5311e755ba5 -> de7275132d83. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. status_model.rs: status_model.rs gained validate_kind, which refuses a statuses.delivery.items.kind declaration that would be inert (a blank or undeclared field, no kinds, an unreachable delivered tier), and the state machine, success terminals and transitions are unchanged; status.rs: the delivery-agreement context is now built with the record, one added argument, and the status rules are unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/status_model.rs` 9cbbc69bdcdc -> ca68bc6df9e8, `rust/crates/urzua-core/src/rules/status.rs` de7275132d83 -> ffb14887aa68. Reason: Read the diff of each pinned file since the old pin; the claim still holds. The files changed in this PR only to read the declared Done means section name in messages, to confine locator probes to the repository, to excuse a waived item from locator-exists only while its waiver is unexpired, and to carry the section name to reverify messages; the behaviour each record relies on is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
