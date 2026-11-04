---
Stable-Id: 01M6K40Q60VB9T17T4YHKBYRXZ
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-22
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/table.rs, code:rust/crates/urzua-core/src/rules/runner.rs, code:rust/crates/urzua-cli/src/commands/check.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
---
# 171 — One rule definition table and a runner that owns gating, severity, scoping and OutOfScope

## What

Replace the hand-maintained rule registration with one rule definition table, and a runner that owns gating, severity override, scope filtering and `OutOfScope` classification.

Each entry carries: id, description, scope (records, config, corpus or diff), the anchor its findings sit on (a record, the config, the corpus or a path), whether it reports outside the corpus, whether it needs the identity index, and a `run` function over one shared context built once (records, config, index, full text, drift). `ALL_RULES`, `RULE_METADATA`, `RULES_REPORTING_OUTSIDE_THE_CORPUS` and `IDENTITY_DEPENDENT_RULES` in `urzua-core/src/rules/catalog.rs`, the 51 `gated(...)` closures in `urzua-cli/src/commands/check.rs`, and the rule-id special cases in the post-hoc finding filter (`RULE_TYPE_RECORD_OUTSIDE_DECLARED_DIR`, `RULE_FILENAME_IDENTITY_PATTERN`) become derived from the table. This record also carries `MILE-119`'s population builder: `header_shape: none` classification is done once in the runner.

## Why

There is no `Rule` abstraction today. A rule is a free function with its own signature plus an id constant, an `ALL_RULES` entry, a `RULE_METADATA` entry, a closure in `check.rs`, possibly an entry in one or two parallel lists, and a config option table entry: six or more places. The existing test compares only two of them in order; an unwired rule is caught only by an integration test. Scope gating is re-derived inside each rule (`has_no_header`, `HeaderShape::None`, `OutOfScope` in `declaration.rs`, `header.rs`, `section.rs`, `field.rs`, `relation.rs`, `back_pointer.rs` and others), and finding scoping runs every rule over the whole corpus and filters afterward, with exceptions found one bug at a time (`BUG-67`, `BUG-86`, `BUG-121`, and `MILE-119`'s `BUG-98`, `BUG-124`, `BUG-137`, `BUG-142`, `BUG-147`). That is the same defect class fixed per instance, which `AGENTS.md` says to fix at the mechanism. It is the no-silent-no-op principle (`ADR-55`) and `SPEC-22`'s extract-on-second-occurrence rule applied to the registry: a property of a rule should live with the rule, so a rule that reports on a non-record path cannot be silently dropped under `check <path>` or `--base`. Evidence read at the foundation review of 2026-11-04.

Ordering: this precedes `MILE-172`, `MILE-174` and `MILE-173`'s check-specific parts, which all edit the same code. It overlaps `MILE-119` (same mechanism) and `MILE-123` (scoping); neither is changed here, and `MILE-119`'s status stays a maintainer decision.

## Done means

Adding a rule is one table entry plus the rule function; no other list, closure or filter names a rule id. The `MILE-119` table-driven test iterates the table and asserts every rule is `OutOfScope` (or documents an exemption) for a `header_shape: none` type. A test fails if a rule is missing from the table. `--base` and `check <path>` scope findings by the declared anchor, not by id. Output of `urzua check` over this repository is unchanged before and after; the acceptance suite (`MILE-170`) passes unchanged; `SPEC-2` and `SPEC-22` carry revision rows.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed at Phase 0. **Why:** maintainer decision of 2026-11-04 pulled foundation work into Phase 0 (Phase 0 is parity on a solid codebase, `SPEC-1`'s Phase 0 exit); filed from the foundation review of the same date, whose evidence is cited above as read at that commit. | **substantive** |
> | 2026-11-04 | First step landed (the rule definition table, `RULE_TABLE`, with the rule metadata and the three parallel lists derived from it); `Status` stays Planned. **Why:** the milestone is delivered in steps (table and metadata, then the runner for gating and severity, then scoping by anchor), each its own reviewable PR; `Realized-by` and `Done` are set by the PR that completes it. | **structural** |
> | 2026-11-04 | Second step landed (the runner): each rule's table entry carries the function that runs it, `check` builds one `RuleCtx` and runs the table by phase, and the gate and declared severity live in the runner. `Status` stays Planned. **Why:** scoping by declared anchor is the remaining step. | **structural** |
> | 2026-11-04 | Third step landed (scoping): each rule's table entry declares the anchor its findings sit on, and a path argument or `--base` keeps or drops a finding by it, so `RULES_REPORTING_OUTSIDE_THE_CORPUS`, the snapshot special case and the two rule-id exceptions in `check` are gone. `Status` stays Planned. **Why:** the config-option ownership lists remain. | **structural** |
> | 2026-11-04 | Fourth step landed and `Status` Planned -> Done, `Realized-by` added. The config loader's option ownership (which rule accepts which option, the `field_levels` rule list, the per-rule `name !=` checks) now reads the rule's table entry, and `config.relation-field-not-known` checks `relation_fields.date` and `author` (`MILE-176`'s gap). **Why:** checked against this record's Done means: a rule is one table entry plus its function (the test that every `RULE_*` constant is in the table, and the config option test, fail otherwise); no list, closure or filter in `check`, the config loader or `init` names a rule id to treat it differently; `--base` and `check <path>` scope by the declared anchor; the `MILE-119` test iterates the table; `urzua check` output over this repository and the acceptance corpora is byte-identical before and after; the acceptance suite passes unchanged. What stays outside the table by design: the commands that read one rule's own options to gather its inputs (`check`'s claim, index and published-file readers, `fix`'s drift gate), which pick inputs and do not register or filter a rule. `MILE-119`'s population builder is `scoped_census_records`, which remains the one place a `header_shape: none` type is classified out of scope; the table declares each rule's `header_none` and the test enforces it. | **structural** |
