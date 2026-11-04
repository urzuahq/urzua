---
Stable-Id: 01M6K40S3H6Y3WN6REJQC88Y4P
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-3
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/config.rs, code:rust/crates/urzua-core/src/rules/options.rs, test:rust/crates/urzua-core/src/config.rs, test:rust/crates/urzua-cli/tests/rule_option_reads.rs
---
# 174 — One RuleSetting option-set definition and typed option access instead of unwrap_or_default re-reads

## What

Define the `RuleSetting` option set once, and give each rule a typed options accessor so `check.rs` stops plumbing options by hand.

Today the set is defined four times in `urzua-core/src/config.rs`: the struct (about 18 `Option` fields), a shadow `Table` struct plus two field-by-field constructions, and the `RuleOption` validation table with `option_field`. The CLI then reads options back with `.get(RULE).and_then(|s| s.x.clone()).unwrap_or_default()` 14 times in `check.rs`. Replace with a single definition (derive `Default`, `deny_unknown_fields`, a small custom visitor for the bare-level shorthand) and `config.rule_opts::<ClaimOpts>(RULE)` returning a typed value.

## Why

Each new rule option means editing four or more places by hand, and a missed spot is a silently ignored option, which `ADR-55` and `ADR-64` forbid. `accepted_statuses` already lacks the `#[serde(default)]` its siblings carry and works only because it is an `Option`. Options for every rule sit in one flat struct, so nothing says which options belong to which rule except the separate table. `unwrap_or_default()` at the call sites erases the difference between "option absent" and "option empty", which is exactly the confusion `BUG-44` (a rule inverting into all-errors when an option was missing) came from. Typed access puts the option with the rule, in step with `MILE-171`. Evidence read at the foundation review of 2026-11-04.

Ordering: after `MILE-171`, because the rule table is where the typed accessor is registered. Parallel option additions (`MILE-34`, the field-rule options) conflict in `config.rs`, so land this before further option work.

## Done means

A new rule option is added in one place; a test fails if an option is declared in the table but not accepted by the struct or the reverse; `check.rs` contains no `unwrap_or_default()` re-read of a rule option; absent and empty are distinguishable in the typed value; config load still refuses an option on a rule that does not take it; `SPEC-3` carries a revision row and its option reference is unchanged.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed at Phase 0. **Why:** maintainer decision of 2026-11-04 pulled foundation work into Phase 0 (Phase 0 is parity on a solid codebase, `SPEC-1`'s Phase 0 exit); filed from the foundation review of the same date, whose evidence is cited above as read at that commit. | **substantive** |
> | 2026-11-04 | `Blocked-on` cleared. **Why:** `MILE-171` is `Done` and `narrative-field.stale` flagged the leftover. `Status` unchanged; this milestone can start. | **substantive** |
> | 2026-11-04 | `Status` Planned -> Done, `Realized-by` added. `RuleSetting` is the one definition of the option set (derived `Default`, `Deserialize` with `deny_unknown_fields`; the hand-written `Deserialize` only adds the bare-level shorthand), `option_field` reads the serialized setting instead of a 17-arm match, and rules read typed options (`Config::rule_opts`, `Config::enabled_rule_opts`, the `*Opts` types in `rules/options.rs`). **Why:** checked against Done means: a new option is a struct field plus, for a list option, one `RULE_OPTIONS` entry, and `the_struct_and_the_option_tables_name_the_same_options` fails when they disagree; `check.rs` has no option re-read (`rule_option_reads.rs`); required options are `Vec` (load guarantees them) while `fields` stays `Option`, so absent and empty differ; `an_option_on_a_rule_that_does_not_take_it_is_refused` and the existing config tests show load still refuses an option on a rule that does not take it, with unchanged messages; `SPEC-3` has a revision row. Not converted: the readers in `discovery.rs` and `embodiment.rs`/`citation.rs`/`dates.rs` that keep `Option` to tell absent from empty or read a scalar; they sit outside the `check.rs` criterion. | **structural** |
