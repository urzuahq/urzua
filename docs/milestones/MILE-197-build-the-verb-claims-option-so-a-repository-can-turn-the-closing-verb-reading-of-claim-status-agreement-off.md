---
Stable-Id: 01M6NW0AXBH6FNEBXF1DQT25T2
Status: Done
Realized-by: code:rust/crates/urzua-core/src/rules/claim.rs, code:rust/crates/urzua-core/src/rules/options.rs, code:rust/crates/urzua-core/src/config.rs, code:.urzua/config.yaml, spec:docs/specs/SPEC-3-configuration.md, spec:docs/specs/SPEC-2-urzua-check.md, test:rust/crates/urzua-cli/tests/claim_field_integration.rs
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

- `verb_claims: false` on `claim.status-agreement` loads when `closes` is declared. Without `closes` it
  is refused at load with a `ConfigError::InvalidClaimField` naming `verb_claims` and `closes`, whether
  the value is `true` or `false`; on any other rule it is refused as `OptionNotApplicable`. Tests pin all
  three and fail before the change.
- With the option off, a fragment whose body says `Fixes BUG-n` for an open record raises no finding and
  counts no verb claim, while the same fragment's `closes: [BUG-n]` is `claim-closes-open`; with it
  absent or `true` the result is byte-identical to before (a test pins both). The notice states that the
  reading is off rather than printing a count.
- `.urzua/config.yaml` sets `verb_claims: false`, and `urzua check` and `urzua check --base origin/main`
  report 0 unwaived errors or warnings, with the `claim.status-agreement` notice showing the reading off.
- `SPEC-3` documents the option, default and load error, and `SPEC-2` the rule description and notice,
  each with a revision row; `make rule-table-check` passes. A CLI changeset is not needed if no
  user-facing CLI behaviour outside the config changes; otherwise a minor changeset with `closes: []`.
- With `verb_claims: false` and `strict` off, the run carries a notice that fragments without the key carry no claim, with a test.
- Removing the reading from the engine is not part of this milestone; it stays a separate later decision.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-88` (Proposed) decides the option on a maintainer decision of 2026-11-05 and no milestone builds it; blocked on the maintainer accepting it. | **substantive** |
> | 2026-11-05 | `Phase` 1 -> 0, and Done means gains the notice when `verb_claims: false` is set without `strict`. **Why:** maintainer decision, 2026-11-05, on accepting `ADR-88`: it sets this repository's own config, which Phase 0 covers. | **substantive** |
> | 2026-11-05 | `Status` Planned -> Done; `Realized-by` added. **Why:** Done means is met and `ADR-88` is Accepted. `verb_claims: false` on `claim.status-agreement` loads with `closes`; without `closes` it is an `InvalidClaimField` load error naming `verb_claims` and `closes` for `true` as well as `false`, and on another rule `OptionNotApplicable`; with it off a body saying `Fixes BUG-n` raises no finding and counts nothing while the same fragment's `closes: [BUG-n]` is `claim-closes-open`, and the notice says the reading is off; absent or `true` is byte-identical (a test compares the reports); set without `strict` a second notice says a fragment without the key carries no claim. This repository sets `verb_claims: false`. `SPEC-2` and `SPEC-3` have revision rows and the rule table is regenerated. Removing the reading from the engine stays a separate decision. | **substantive** |
