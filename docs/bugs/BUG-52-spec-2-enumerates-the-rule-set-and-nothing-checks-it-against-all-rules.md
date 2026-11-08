---
Stable-Id: 01M2W0QAD0PBVS3XD05RQQTJ4W
Status: Fixed
Found-in: 'A code review of PR #74 -- SPEC-2 called itself the complete, current rule set while understating it by four'
Realized-by: code:scripts/generate-rule-table.py@sha256:f5c69a4acb163e956b54dc91daccb2f0f75731e9d74926e9cbe69c3c32d36923, code:Makefile@sha256:ccc04e61c2e32aad9ccd10166a12df24d31e8ccb1b24233f01c566d7058f382a, test:rust/crates/urzua-core/src/rules/table.rs#every_rule_id_constant_has_a_table_entry@sha256:d8805b858389e7ccda9bd33e1d8227c020c617761ed0d44a86632b39ef89f9e7
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
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-07 | `Realized-by` now cites `scripts/generate-rule-table.py` and the Makefile's `rule-table-check` target (which runs it with `--check`), the check that fails when `SPEC-2`'s rule table differs from the shipped rule set. **Why:** the one locator before, `table.rs#every_rule_id_constant_has_a_table_entry`, compares rule-id constants with the metadata table, a different drift; it stays as supporting evidence. No cargo test compares `SPEC-2`'s table; `make rule-table-check` runs in CI. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: scripts/generate-rule-table.py@sha256:f5c69a4acb16, Makefile@sha256:fb9fda4c8508 Reason: Pinned the files as they are now after checking each against the record it evidences: publish-release.yml still builds and uploads the archives itself (BUG-28), new_record.rs still renders only the configured YAML header and new_integration.rs still holds the test that a blockquote template does not override it (BUG-3), waiver.rs is_active still parses Expires as a date (BUG-31), prepare-release.yml still hands knope's nothing-to-release exit to release-guard.sh instead of failing (BUG-33), ci.yml's changeset job still reads the label live (BUG-34), knope.toml still releases from changesets only (BUG-35), main.rs and check_integration.rs still emit doctor as JSON (BUG-4), release-guard.test.sh still plants an unmatched-package fragment and asserts a nonzero exit (BUG-48), field_state.rs still carries the retired placeholder tokens (BUG-5), and generate-rule-table.py and the Makefile rule-table-check target still compare SPEC-2's rule table with the shipped rule set (BUG-52). BUG-29 and BUG-30 are not in this batch because the filename.title-consistency code they cite moved out of rules.rs. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `Makefile` fb9fda4c8508 -> ccc04e61c2e3. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. Makefile: the rust-test target now runs the tests through .github/scripts/ci-tests.sh (nextest, doctests and the acceptance runner under the leak guard) and the Makefile gained the ci-gate-test and nextest-pin targets, and the targets the record relies on are unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
