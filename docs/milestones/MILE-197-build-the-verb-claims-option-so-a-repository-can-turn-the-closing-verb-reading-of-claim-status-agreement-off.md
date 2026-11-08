---
Stable-Id: 01M6NW0AXBH6FNEBXF1DQT25T2
Status: Done
Realized-by: test:rust/crates/urzua-cli/tests/claim_field_integration.rs#verb_claims_false_turns_the_prose_reading_off_and_keeps_the_field_observed_failing@sha256:86872e7977f42be0a61d3f86d310d0fbeb11bd1944a3e088667cdc8837903bb4, test:rust/crates/urzua-cli/tests/claim_field_integration.rs#verb_claims_needs_closes_whether_true_or_false_observed_failing@sha256:731bfb82bc43e7299e25f5f96f1b9f279a09ad5038bbc4b03e7ecdd8663ed508, code:rust/crates/urzua-core/src/config.rs@sha256:18818ee3b2262e4f5dd6b5960d7f874a5662afaabf772f15c82441d7cc7c2df0, test:rust/crates/urzua-cli/tests/claim_field_integration.rs#verb_claims_on_another_rule_is_not_applicable_observed_failing@sha256:f109b5b270401cd1c805ce25a8ee7b4c039f9c5cae37bf4c2dbce17e43f0d1a7, code:rust/crates/urzua-core/src/rules/options.rs@sha256:3ab929cf735279ad174257739b5d3119925067bc5797c859adbfa728a051147e, test:rust/crates/urzua-cli/tests/claim_field_integration.rs#verb_claims_absent_or_true_is_the_reading_as_it_was_observed_failing@sha256:93d41cae1113d83eae873f2a581b39e370d823212ea1fcacb1ed99aea3f9d214, code:.urzua/config.yaml@sha256:151830e5926d1887be39a723d110744ff5e2ae8c408bb114a90ba3e2b8316a2e, code:Makefile@sha256:fb9fda4c85083447873cd372df238eae66f507575c2257c8f275f2253e4ae436, code:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d, spec:docs/specs/SPEC-3-configuration.md, spec:docs/specs/SPEC-2-urzua-check.md, code:scripts/generate-rule-table.py@sha256:f5c69a4acb163e956b54dc91daccb2f0f75731e9d74926e9cbe69c3c32d36923, code:.changeset/claim-verb-claims-option.md@sha256:6691ff9c284ddd7c319a4fcde8c956534c4273081126086a7a6b389b86ac167c, test:rust/crates/urzua-cli/tests/claim_field_integration.rs#verb_claims_false_without_strict_carries_a_notice_observed_failing@sha256:df4b1a04cdebe2cd2656719a5fb325dce6a88719ec4bdb9f57636c6643b8e08b
Phase: '0'
Track: governance-process
Implements: ADR-88
Blocked-on: null
---
# 197 — Build the verb_claims option so a repository can turn the closing-verb reading of claim.status-agreement off

## What

Build the `verb_claims` option on `claim.status-agreement` (`ADR-88`): `false` turns the closing-verb
reading off, so only the structured `closes` field is a claim; it is refused at load unless `closes` is
declared; an undeclared option behaves as today. Set it in this repository's `.urzua/config.yaml`. It
is blocked until the maintainer accepts `ADR-88`.

## Why

`ADR-85` kept the prose reading as a deprecated fallback "until a later decision removes it", and this
repository now has `closes` and `strict: true` with all 184 fragments carrying `closes:`. Measured over
them, the verb reading finds 69 claims, 67 of them already in the fragment's own `closes:` and 2 not
claims at all (`BUG-60` and `BUG-61` in an example at `.changeset/claim-closes-field.md:8`). The
maintainer decided on 2026-11-05 for strict mode and structured data in and out.

**Phase 1, not 0.** Phase 0's single exit is urzua governing its own repository with zero standing
warnings and reproducing the governance functionality of a real adopter environment (`SPEC-1`, Exit bar);
turning a deprecated fallback off in an opt-in configuration is neither, and nothing in the Phase 0 bar
waits on it. It is the step by which a repository that has moved to the structured field stops paying
for prose, which is what the Phase 1 adopters do, so it sits with them.

## Done means

- `verb_claims: false` on `claim.status-agreement` loads when `closes` is declared. — evidence: test:rust/crates/urzua-cli/tests/claim_field_integration.rs#verb_claims_false_turns_the_prose_reading_off_and_keeps_the_field_observed_failing@sha256:86872e7977f42be0a61d3f86d310d0fbeb11bd1944a3e088667cdc8837903bb4
- Without `closes`, `verb_claims` is refused at load with a `ConfigError::InvalidClaimField` naming `verb_claims` and `closes`, whether the value is `true` or `false`. — evidence: test:rust/crates/urzua-cli/tests/claim_field_integration.rs#verb_claims_needs_closes_whether_true_or_false_observed_failing@sha256:731bfb82bc43e7299e25f5f96f1b9f279a09ad5038bbc4b03e7ecdd8663ed508, code:rust/crates/urzua-core/src/config.rs@sha256:18818ee3b2262e4f5dd6b5960d7f874a5662afaabf772f15c82441d7cc7c2df0
- On any other rule `verb_claims` is refused as `OptionNotApplicable`. — evidence: test:rust/crates/urzua-cli/tests/claim_field_integration.rs#verb_claims_on_another_rule_is_not_applicable_observed_failing@sha256:f109b5b270401cd1c805ce25a8ee7b4c039f9c5cae37bf4c2dbce17e43f0d1a7, code:rust/crates/urzua-core/src/rules/options.rs@sha256:3ab929cf735279ad174257739b5d3119925067bc5797c859adbfa728a051147e
- With the option off, a fragment whose body says `Fixes BUG-n` for an open record raises no finding and counts no verb claim, while the same fragment's `closes: [BUG-n]` is `claim-closes-open`. — evidence: test:rust/crates/urzua-cli/tests/claim_field_integration.rs#verb_claims_false_turns_the_prose_reading_off_and_keeps_the_field_observed_failing@sha256:86872e7977f42be0a61d3f86d310d0fbeb11bd1944a3e088667cdc8837903bb4
- With the option absent or `true` the result is byte-identical to before (a test pins both). — evidence: test:rust/crates/urzua-cli/tests/claim_field_integration.rs#verb_claims_absent_or_true_is_the_reading_as_it_was_observed_failing@sha256:93d41cae1113d83eae873f2a581b39e370d823212ea1fcacb1ed99aea3f9d214
- The notice states that the reading is off rather than printing a count. — evidence: test:rust/crates/urzua-cli/tests/claim_field_integration.rs#verb_claims_false_turns_the_prose_reading_off_and_keeps_the_field_observed_failing@sha256:86872e7977f42be0a61d3f86d310d0fbeb11bd1944a3e088667cdc8837903bb4
- `.urzua/config.yaml` sets `verb_claims: false`. — evidence: code:.urzua/config.yaml@sha256:151830e5926d1887be39a723d110744ff5e2ae8c408bb114a90ba3e2b8316a2e
- `urzua check` and `urzua check --base origin/main` report 0 unwaived errors or warnings. — evidence: code:Makefile@sha256:fb9fda4c85083447873cd372df238eae66f507575c2257c8f275f2253e4ae436, code:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d
- `SPEC-3` documents the option, default and load error, and `SPEC-2` the rule description and notice, each with a revision row. — evidence: spec:docs/specs/SPEC-3-configuration.md, spec:docs/specs/SPEC-2-urzua-check.md
- `make rule-table-check` passes. — evidence: code:scripts/generate-rule-table.py@sha256:f5c69a4acb163e956b54dc91daccb2f0f75731e9d74926e9cbe69c3c32d36923, spec:docs/specs/SPEC-2-urzua-check.md
- A minor changeset is added. — evidence: code:.changeset/claim-verb-claims-option.md@sha256:6691ff9c284ddd7c319a4fcde8c956534c4273081126086a7a6b389b86ac167c
- With `verb_claims: false` and `strict` off, the run carries a notice that fragments without the key carry no claim, with a test. — evidence: test:rust/crates/urzua-cli/tests/claim_field_integration.rs#verb_claims_false_without_strict_carries_a_notice_observed_failing@sha256:df4b1a04cdebe2cd2656719a5fb325dce6a88719ec4bdb9f57636c6643b8e08b

Removing the reading from the engine is not part of this milestone; it stays a separate later decision.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-88` (Proposed) decides the option on a maintainer decision of 2026-11-05 and no milestone builds it; blocked on the maintainer accepting it. | **substantive** |
> | 2026-11-05 | `Phase` 1 -> 0, and Done means gains the notice when `verb_claims: false` is set without `strict`. **Why:** maintainer decision, 2026-11-05, on accepting `ADR-88`: it sets this repository's own config, which Phase 0 covers. | **substantive** |
> | 2026-11-05 | `Status` Planned -> Done; `Realized-by` added. **Why:** Done means is met and `ADR-88` is Accepted. `verb_claims: false` on `claim.status-agreement` loads with `closes`; without `closes` it is an `InvalidClaimField` load error naming `verb_claims` and `closes` for `true` as well as `false`, and on another rule `OptionNotApplicable`; with it off a body saying `Fixes BUG-n` raises no finding and counts nothing while the same fragment's `closes: [BUG-n]` is `claim-closes-open`, and the notice says the reading is off; absent or `true` is byte-identical (a test compares the reports); set without `strict` a second notice says a fragment without the key carries no claim. This repository sets `verb_claims: false`. `SPEC-2` and `SPEC-3` have revision rows and the rule table is regenerated. Removing the reading from the engine stays a separate decision. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 7 locators pinned, 7 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/config.rs@sha256:18818ee3b226, rust/crates/urzua-core/src/rules/options.rs@sha256:3ab929cf7352, .urzua/config.yaml@sha256:151830e5926d, Makefile@sha256:fb9fda4c8508, .github/workflows/checks.yml@sha256:4afa5fe0db59, scripts/generate-rule-table.py@sha256:f5c69a4acb16, .changeset/claim-verb-claims-option.md@sha256:6691ff9c284d Reason: Pinned the files as they are now after checking each against the record it evidences: sequence.rs still reads roadmap.pairs as one graph (MILE-192), waiver.rs and config.rs still read the permanent declaration and the sixteen WAIVER records and the waiver template still exist as the migration the milestone describes (MILE-193), config.rs, config.yaml, the Makefile, checks.yml, MILE-189 and generate-rule-table.py still carry the info level (MILE-194), harness/run.rs and config.rs still offer block_on all (MILE-195), config.rs and rules/roadmap.rs still read work_types (MILE-196), config.rs and options.rs still read verb_claims (MILE-197), the harness modules and the changeset still install and run the engine hooks (MILE-198), bundle.rs, the bundled and repo-local command files and urzua-record.sh still draft every record type and the lifecycle commands (MILE-199), init.rs still proposes yaml-frontmatter (MILE-2), and authority.rs, harness/gate.rs, rules/decider.rs and the urzua-gate action scripts still enforce who may accept a record (MILE-200). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
