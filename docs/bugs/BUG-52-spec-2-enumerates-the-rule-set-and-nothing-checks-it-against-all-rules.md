---
Stable-Id: 01M2W0QAD0PBVS3XD05RQQTJ4W
Status: Fixed
Found-in: 'A code review of PR #74 -- SPEC-2 called itself the complete, current rule set while understating it by four'
Regression-test: 'every_rule_in_all_rules_has_metadata_in_the_same_order, rules.rs; make rule-table-check (wired into make ci)'
---
# 52 — `SPEC-2` enumerates the rule set and nothing checks it against `ALL_RULES`

## What is wrong

`SPEC-2` is `check`'s own spec and says of its rule table: *"This list is the complete, current set."*

It said **seventeen** while `ALL_RULES` shipped **twenty-one**. `pointer.target-status`,
`field.pending`, `claim.status-agreement` and `embodiment.locator-exists` were added over two days,
each in a change that touched `rules.rs` and none that touched the spec -- and one of them is declared
`error` in this repository's own config.

## Why it is this project's problem specifically

The whole premise is that a record and the code it describes should not be able to disagree silently.
`embodiment.consistency` enforces that for *locators*; nothing enforces it for a *list*. A spec that
names itself complete is making a checkable claim, and it is the one claim in the corpus that maps
exactly onto a constant in the source.

`MILE-94` records what this costs: specs asserting what the code contradicts. `ADR-52` deliberately
left thirteen specs describing TOML rather than rewrite them ahead of the code, on the same reasoning.
That was a decision to defer; this is no mechanism at all.

## Fix

A rule comparing a declared list against a shipped one. The general form is the interesting part: this
is not *"check `SPEC-2`"* but *"a record claiming to enumerate a set the binary owns must match it"*,
which needs the set named in config rather than a spec number hardcoded in Rust.

Closest existing shape is `config.pointer-declaration-missing` -- a config-level, schema-inventory
check rather than a per-record one.

Related: `BUG-41` (companion pairs checked for one pair only) and `BUG-40` (a rule's scope
unreadable). All three are the same family: the engine knows something and does not compare it to what
the corpus says about it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** `SPEC-2` called itself the complete current rule set while understating it by four, across two days and four separate changes to `rules.rs`. The claim maps exactly onto `ALL_RULES`, so it is checkable, and nothing checks it. | **substantive** |
> | 2026-09-19 | Deferred behind `MILE-98`. **Why:** fixable today as another hardcoded comparison in `rules.rs`, and that is the mistake this family *is* -- `PLACEHOLDER_TOKENS` transcribed by hand, `config.pointer-declaration-missing` hardcoded to one pair, `revision-log.change-class-required` keyed to a literal string. Each is a comparison written as a constant. Under the declared document model they are declarations, so building them now means building them twice and teaching the second version nothing. The gap stays open for the duration, deliberately. | **substantive** |
> | 2026-09-19 | `Blocked-on: MILE-98` declared as a field rather than described in prose. **Why:** the `bug` type did not declare `Blocked-on`, so the deferral was written into this log where no rule could see it. That is a configuration gap, not a missing rule -- `ADR-53` makes the field set a repository's declaration, and declaring it took one line. `narrative-field.stale` now reports all six of these when `MILE-98` reaches a terminal status. | **structural** |
> | 2026-09-23 | Fixed, unblocked from `MILE-98`. **Why:** the deferred concern was specifically "another hardcoded comparison in `rules.rs`" -- a rule re-deriving the same fact `ALL_RULES` already states, in a second place, taught nothing by `MILE-98`'s eventual declared document model. The actual fix collapses to one source instead: `urzua_core::rules::RULE_METADATA` (checked against `ALL_RULES` by a compiled test, `every_rule_in_all_rules_has_metadata_in_the_same_order`) is what `SPEC-2`'s table is generated from (`scripts/generate-rule-table.py`), not compared against. Nothing is duplicated for `MILE-98` to later replace, so the original deferral reasoning doesn't apply to this shape of fix. | **substantive** |
