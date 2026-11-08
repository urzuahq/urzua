---
Stable-Id: 01M6JCYYQ4SAV1Y7ST34B8YBCJ
Status: Done
Phase: '0'
Track: governance-process
Implements: SPEC-3
Realized-by: code:rust/crates/urzua-core/src/config.rs@sha256:79b8ccfb0a6dd2f7c8366d143105d7dafa9e93099e4ddfdc9195f66770c4e327, test:rust/crates/urzua-cli/tests/fail_on_integration.rs#config_fail_on_warning_makes_a_warning_exit_1_with_unchanged_json_observed_failing@sha256:d1e8e19c3842276d697a34cbf386f69ca04921417026f4d0cc1a5f9a86219b04, test:rust/crates/urzua-core/src/report.rs#a_waived_warning_never_fails_under_fail_on_warning@sha256:c5809caf0107252dfa4be40786a9490b1c81ba67d1a6c2c6c929bdd780782b3d, test:rust/crates/urzua-cli/tests/fail_on_integration.rs#fail_on_warning_still_exits_0_on_a_clean_corpus@sha256:cf2a2b2f6eccd4280e21ab610fdaea569de763897c41dcea4239b57f0da2d90b, test:rust/crates/urzua-cli/tests/fail_on_integration.rs#the_flag_overrides_the_config_in_both_directions_observed_failing@sha256:9ed82c0a3d5250ed239c08d30b1ffdbe243ccd4fac063298292e2a6bb91a3811, test:rust/crates/urzua-cli/tests/fail_on_integration.rs#an_unknown_fail_on_value_is_refused@sha256:18f38ee5051f554513ab1c062c1d37e90ef3ea48eb8dcfe6af7f1bb287815165, test:rust/crates/urzua-cli/tests/fail_on_integration.rs#an_invalid_config_fail_on_is_a_could_not_run@sha256:0d1e4dc2b6814c65060b1ac658c32d933d950de18ce98801d7b1695d77eb407c, spec:docs/specs/SPEC-3-configuration.md, spec:docs/specs/SPEC-2-urzua-check.md, spec:docs/bugs/BUG-180-a-base-run-reports-unchanged-source-files-as-out-of-scope-so-config-scope-matches-nothing-warns-on-every-pull-request.md
---
# 145 — A fail_on threshold lets a repository make warnings fail urzua check

## What

A top-level `fail_on: error | warning` key in `.urzua/config.yaml`, and a matching `urzua check
--fail-on error|warning` flag that overrides it, set the lowest unwaived finding severity that makes
`check` (and `audit`) exit 1. The default is `error`, so a repository that does not declare it is
unchanged. The JSON on stdout is identical under either value; only the exit code moves. This
repository sets `fail_on: warning`, so both CI steps (whole repo, and `--base` on a pull request)
fail on any warning.

## Why

`urzua check` on `main` reports no findings, but nothing stopped a warning from landing again: every
warning-level rule exits 0, and no flag or key raised the exit threshold. A zero-warning state that
only holds by attention regresses. The threshold is repository policy, so it is a config key like
every other behaviour (`ADR-53`), with a flag for a one-off stricter or laxer run.

## Done means

- `fail_on: warning` makes a run with an unwaived warning exit 1 with unchanged stdout. — evidence: code:rust/crates/urzua-core/src/config.rs@sha256:79b8ccfb0a6dd2f7c8366d143105d7dafa9e93099e4ddfdc9195f66770c4e327, test:rust/crates/urzua-cli/tests/fail_on_integration.rs#config_fail_on_warning_makes_a_warning_exit_1_with_unchanged_json_observed_failing@sha256:d1e8e19c3842276d697a34cbf386f69ca04921417026f4d0cc1a5f9a86219b04
- A waived warning does not make the run exit 1. — evidence: test:rust/crates/urzua-core/src/report.rs#a_waived_warning_never_fails_under_fail_on_warning@sha256:c5809caf0107252dfa4be40786a9490b1c81ba67d1a6c2c6c929bdd780782b3d
- A clean run still exits 0. — evidence: test:rust/crates/urzua-cli/tests/fail_on_integration.rs#fail_on_warning_still_exits_0_on_a_clean_corpus@sha256:cf2a2b2f6eccd4280e21ab610fdaea569de763897c41dcea4239b57f0da2d90b
- `--fail-on` overrides the key in both directions. — evidence: test:rust/crates/urzua-cli/tests/fail_on_integration.rs#the_flag_overrides_the_config_in_both_directions_observed_failing@sha256:9ed82c0a3d5250ed239c08d30b1ffdbe243ccd4fac063298292e2a6bb91a3811
- Any other value is refused, and a config error exits 2. — evidence: test:rust/crates/urzua-cli/tests/fail_on_integration.rs#an_unknown_fail_on_value_is_refused@sha256:18f38ee5051f554513ab1c062c1d37e90ef3ea48eb8dcfe6af7f1bb287815165, test:rust/crates/urzua-cli/tests/fail_on_integration.rs#an_invalid_config_fail_on_is_a_could_not_run@sha256:0d1e4dc2b6814c65060b1ac658c32d933d950de18ce98801d7b1695d77eb407c
- It is documented in `SPEC-3` and `SPEC-2`. — evidence: spec:docs/specs/SPEC-3-configuration.md, spec:docs/specs/SPEC-2-urzua-check.md
- Turning it on exposed `BUG-180`, a false warning every `--base` run produced, fixed in the same change. — evidence: spec:docs/bugs/BUG-180-a-base-run-reports-unchanged-source-files-as-out-of-scope-so-config-scope-matches-nothing-warns-on-every-pull-request.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed and done. **Why:** keep this repository at zero warnings by making them fail its own CI. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/config.rs@sha256:18818ee3b226 Reason: Pinned the files as they are now after checking each against the record it evidences: status_model.rs and rules/status.rs still declare and judge the per-type state machine (MILE-132), acceptance.rs and the corpus-h gate expectation still hold the held-source case (MILE-133), rules/index.rs still compares generated indexes with the records (MILE-139), config.rs still reads field_levels and gate.rs still derives the report status (MILE-140), refs_snapshot.rs and commands/refs.rs still keep and refresh the cited-number snapshot (MILE-143), the corpus-j expectations still plant an unlisted edge and an unreachable status (MILE-144), config.rs still reads fail_on (MILE-145), claim_field_integration.rs still carries a Verifies back-pointer comment of the kind the milestone's own-corpus item names (MILE-146), main.rs and discovery.rs still read the full back-pointer id list and the decision-gate sources (MILE-147), and embodiment.rs still reads sticky_states (MILE-148). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/config.rs` 18818ee3b226 -> 79b8ccfb0a6d. Reason: config.rs: parse_with_document_models, validate_relation_pairs, claim_expiry_problems and revision_log_problem split into named validation passes called in the original order; messages and order unchanged, guarded by validation_order_tests. Behaviour unchanged, outputs byte-identical (check, check --base and fix --rebaseline JSON compared before and after). **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
