---
Status: Done
Stable-Id: 01M1Z2T44DHPRE5C4DVENWXY58
Phase: '0'
Track: schema-governance
Implements: —
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#the_declared_level_overrides_the_rule_body_s_own_severity@sha256:8f67f94ecf327ba3c3ae071f21b729f97fe1637d2b178f8f8ab6adb1cb8b95d8, test:rust/crates/urzua-cli/tests/check_integration.rs#an_undeclared_rule_is_reported_as_not_enabled_never_omitted@sha256:6517650418cd37bce81e8fc012fdecdc11c176e43f3d1167fb1e4bc1f1248ff7, code:rust/crates/urzua-core/src/rules/table.rs@sha256:40b9283c07e59e462ca70047f0baa7263a3442d1e4b099d18aa478f8b55bc041, code:rust/crates/urzua-core/src/rules/pointer.rs@sha256:e4d8db46f2e32fd5b80d2a62fe9938bf7ef057e18de0e92d2f22c66987b057d9, code:scripts/generate-rule-table.py@sha256:f5c69a4acb163e956b54dc91daccb2f0f75731e9d74926e9cbe69c3c32d36923, code:rust/crates/urzua-cli/src/commands/rules.rs@sha256:314b3277bd031399ed2b15af322bf9c6ccb14f6d1c178c8ada0734e9a52fc3c9, code:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d, code:.urzua/config.yaml@sha256:151830e5926d1887be39a723d110744ff5e2ae8c408bb114a90ba3e2b8316a2e
---
# 80 — Configurable rule severity, per SPEC-1's own unbuilt promise

## What

`Severity` is a hardcoded two-value enum (`Error`/`Warning`) baked directly into ~13 call sites
across `rules.rs`, with zero config surface -- despite SPEC-1 explicitly listing "which checks are
errors vs. warnings" as configurable from v0. This milestone owns *making a rule's severity
configurable per repo* in `.urzua/config.toml` (a config-surface question); whether `Severity`
itself gains more values (e.g. `Info` — MILE-46, found independently, same day) is a separate
schema/enum question, kept apart deliberately rather than conflated. Also separate: whether the CLI
should gain a general log-verbosity control (debug/info-as-diagnostic-noise while a command runs) —
orthogonal to findings entirely, since a finding is either a real problem or it isn't, while
verbosity is about how much the tool narrates while deciding that.

## Why

Raised live: rules today only distinguish two severities, hardcoded in source, with no way for a
repo to downgrade or upgrade a given rule's default -- exactly the gap SPEC-1 named as a v0
configuration surface and never built. Worth resolving the findings-severity-vs-log-verbosity
distinction explicitly before building either, rather than conflating "how bad is this finding"
with "how much should the tool print" into one enum.

## A worked instance

`ADR-45` carried `Embodiment: Verified` from its first commit (`3b0fe66`, 2026-09-10) while every
`Realized-by` locator was a `code:` one and nothing tested the flow. The computed tier was
`Implemented`. `embodiment.consistency` reported the disagreement correctly on every run for six
days, and nothing happened, because:

- the rule emits `Warning`, hardcoded at its call site;
- `check` computes `blocking` from `Error`-severity findings only;
- the corpus carries **204 findings, all warnings, zero errors**, so `blocking` is `false` and
  `urzua check docs/` exits 0.

It was found only because someone opened that record for an unrelated reason. This is the concrete
cost of the gap: a record asserting a higher evidence tier than it holds is an untrue claim in the
corpus -- the class this project exists to catch -- and it is currently indistinguishable from 203
routine observations.

Note what this milestone is *not*: the rule is correct and its default of `Warning` is defensible,
since drift is usually a signal to review. What is missing is any way for this repo to say that
*this* rule, here, should block.

## Done means

- A `rules` table in config declares each rule's `level`, and that level replaces whatever the rule chose. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#the_declared_level_overrides_the_rule_body_s_own_severity@sha256:8f67f94ecf327ba3c3ae071f21b729f97fe1637d2b178f8f8ab6adb1cb8b95d8
- Every rule is opt-in, and an undeclared rule is reported `not-enabled` rather than omitted. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#an_undeclared_rule_is_reported_as_not_enabled_never_omitted@sha256:6517650418cd37bce81e8fc012fdecdc11c176e43f3d1167fb1e4bc1f1248ff7
- `pointer.resolution` was split so one id carries one question. — evidence: code:rust/crates/urzua-core/src/rules/table.rs@sha256:40b9283c07e59e462ca70047f0baa7263a3442d1e4b099d18aa478f8b55bc041, code:rust/crates/urzua-core/src/rules/pointer.rs@sha256:e4d8db46f2e32fd5b80d2a62fe9938bf7ef057e18de0e92d2f22c66987b057d9
- The rule list in `SPEC-2` is generated from the shipped set (`scripts/generate-rule-table.py`, `urzua rules`). — evidence: code:scripts/generate-rule-table.py@sha256:f5c69a4acb163e956b54dc91daccb2f0f75731e9d74926e9cbe69c3c32d36923, code:rust/crates/urzua-cli/src/commands/rules.rs@sha256:314b3277bd031399ed2b15af322bf9c6ccb14f6d1c178c8ada0734e9a52fc3c9, code:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d
- Urzua's own `.urzua/config.yaml` declares every rule's level. — evidence: code:.urzua/config.yaml@sha256:151830e5926d1887be39a723d110744ff5e2ae8c408bb114a90ba3e2b8316a2e

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Phase `1` -> `0`. **Why:** this is the root blocker for Phase 0, not work that follows it. A `[rules]` table is a parse error today (`deny_unknown_fields`, verified by running it), so nothing can be made opt-in and `RFC-28` is unbuildable as written. `MILE-4` and `MILE-51` both depend on it. Phase 0 could not close while its prerequisite was scheduled after it -- the mechanical reason this work kept producing blockers behind blockers (`RFC-33`). | **structural** |
> | 2026-09-16 | Added a worked instance: `ADR-45`'s false `Embodiment: Verified`, reported correctly and non-blockingly for six days among 204 warnings. **Why:** this milestone's motivation was a principle ("rules only distinguish two severities"); it now has a dated case where the missing configurability let an untrue claim survive in the corpus, which is a stronger argument for building it than the principle was. | **substantive** |
> | 2026-09-17 | `Status: Planned` → `Done`. **Why:** the `rules` table ships, every rule is opt-in and carries a declared `level` that replaces whatever `rules.rs` chose, and an undeclared rule is reported `not-enabled` rather than omitted (`ADR-7`). `pointer.resolution` was split on the way: it emitted a finding for every reference that *resolved* -- 172 of 213 on this corpus -- and one rule id cannot carry two severities, so the dangling-reference error and the target-status policy could never be levelled apart. Measured after: 215 findings → 47. The findings-severity vs log-verbosity distinction this record insisted on keeping separate is untouched; `MILE-46` and the CLI verbosity question remain open. Presets are `MILE-95`. | **substantive** |
> | 2026-09-17 | Recorded `BUG-38`: the `field.quality` flattening this change accepted is not acceptable. **Why:** `Blank` and `Placeholder` mean someone forgot; `Pending` means someone declared work unfinished. Two questions, one rule id, so no declared level is correct for both -- `error` blocks a deliberate marker and `warn` unblocks a genuinely empty required field. Same defect as `pointer.resolution`, which this change split; missed on the first pass because the grading read as one question's degrees until `Pending`'s own definition was checked. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 6 locators pinned, 6 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/rules/table.rs@sha256:40b9283c07e5, rust/crates/urzua-core/src/rules/pointer.rs@sha256:e4d8db46f2e3, scripts/generate-rule-table.py@sha256:f5c69a4acb16, rust/crates/urzua-cli/src/commands/rules.rs@sha256:314b3277bd03, .github/workflows/checks.yml@sha256:4afa5fe0db59, .urzua/config.yaml@sha256:151830e5926d Reason: Pinned the files as they are now after checking each against the record it evidences: section.rs, document_model.rs, table.rs and config.yaml still hold the decisionless-status exemption (MILE-7), config.yaml and the milestone template still carry the WontDo terminal status (MILE-71), the spec template and config.yaml still make urzua new spec work (MILE-74), field_state.rs, config.yaml, checks.yml and the Makefile still report a returning placeholder (MILE-78), config.yaml still declares the milestone type (MILE-8), table.rs, pointer.rs, commands/rules.rs, generate-rule-table.py and config.yaml still give every rule a configurable level (MILE-80), config.yaml still declares the per-type field sets (MILE-81), config.yaml, pointer.rs and narrative.rs still hold the opt-in narrative-field.stale rule (MILE-83), schema_version.rs, the corpus-a expectation and config.yaml still show the removed shapes refused (MILE-85), and new.rs, new_refs.rs, open_pr.rs, report.rs, config.rs, discovery.rs, check.rs, config.yaml and table.rs still hold the open pull request numbering check (MILE-89). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
