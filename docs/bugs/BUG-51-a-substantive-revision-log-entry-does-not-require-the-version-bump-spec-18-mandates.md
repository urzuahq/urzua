---
Stable-Id: 01M2W0QA0YMGXD5V5XBXN80PKB
Status: Fixed
Found-in: 'A code review of PR #74 -- SPEC-1 was rewritten by roughly 60%% with two substantive entries and its Version left at 0.3'
Realized-by: test:rust/crates/urzua-cli/tests/version_bump_integration.rs#a_substantive_row_without_a_version_bump_is_reported_observed_failing@sha256:2d0ca8554bb975ac43fb3b9c36ee9615b6dbaffbc5d5875f11c97b73cb72bb14, test:rust/crates/urzua-cli/tests/version_bump_integration.rs#a_structural_row_without_a_version_bump_is_reported@sha256:497c6022b15ac15e21d150034ac2f029c4d28bcb6be0d515751125f01b3ff78d
---
# 51 — A substantive revision-log entry does not require the `Version` bump `SPEC-18` mandates

## What is wrong

`SPEC-18` states the rule plainly:

> a substantive or structural change is a `Version` bump plus a Why-bearing revision-log entry on the
> same spec

`revision-log.change-class-required` checks the entry has a real class. **Nothing checks the other
half.** A spec can be rewritten, gain a `substantive` **or `structural`** entry, and keep its `Version`.

It happened in the change that found it: `SPEC-1` was narrowed by roughly 60%, its `Subject` rewritten
and two `substantive` entries added, with `Version: '0.3'` untouched. Caught by review.

## Why it matters

`Version` is what a reader uses to know whether the spec they remember is the spec in front of them.
A spec whose version has not moved through several substantive rewrites is worse than one with no
version at all, because it asserts stability it does not have.

## Fix

A rule pairing the two halves `ADR-14` and `SPEC-18` already decided: a record gaining a `substantive` **or
`structural`** revision entry in a commit that does not change its `Version` is a finding.

Needs the git-history access `embodiment.consistency` already has (`ADR-32`), since "gained an entry
in this change" is a diff question, not a document question. That is the same precomputed-by-the-caller
shape, not new I/O in `urzua-core`.

## Fixed

Maintainer decision of 2026-11-04. `revision-log.version-bump-required` is a `--base` rule: for each changed record of a type that declares `revision_log: {version_bump: {field, classes}}`, a revision-log row present now and absent from the merge base's copy, whose class is one of `classes`, requires the record's `field` to differ from the merge base's. The merge base's text is read by the CLI through `urzua-io` (`read_at_commit`) and handed to the rules as data, so `urzua-core` stays pure. No type declares it by default; this repository declares it on `spec` for `substantive` and `structural`, and `urzua check --base origin/main` is clean. A record new in the diff has nothing to bump from and is left unexamined, a whole-repo run reports `not-applicable`, and enabling the rule with no declaring type fails the config load. `SPEC-2`, `SPEC-3` and `SPEC-18` are updated.

`MILE-44`'s suggested actions (`ADR-73`) have not landed, so the finding carries none yet; it is the rule's message that says to bump the field.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** `SPEC-18` requires a `Version` bump alongside every substantive revision entry and only the entry half is checked. `SPEC-1` was rewritten by ~60% with two substantive entries and `Version` left at `0.3`; review caught it, the engine did not. | **substantive** |
> | 2026-09-19 | Deferred behind `MILE-98`. **Why:** fixable today as another hardcoded comparison in `rules.rs`, and that is the mistake this family *is* -- `PLACEHOLDER_TOKENS` transcribed by hand, `config.pointer-declaration-missing` hardcoded to one pair, `revision-log.change-class-required` keyed to a literal string. Each is a comparison written as a constant. Under the declared document model they are declarations, so building them now means building them twice and teaching the second version nothing. The gap stays open for the duration, deliberately. | **substantive** |
> | 2026-09-19 | `Blocked-on: MILE-98` declared as a field rather than described in prose. **Why:** the `bug` type did not declare `Blocked-on`, so the deferral was written into this log where no rule could see it. That is a configuration gap, not a missing rule -- `ADR-53` makes the field set a repository's declaration, and declaring it took one line. `narrative-field.stale` now reports all six of these when `MILE-98` reaches a terminal status. | **structural** |
> | 2026-09-19 | Scope corrected to include `structural` entries. **Why:** `SPEC-18` reads *"a substantive **or structural** change is a `Version` bump plus a Why-bearing revision-log entry"*, and this record named only the substantive half -- which would have built a rule enforcing half a decision, the same shape as `MILE-7` being filed as an exemption when the requirement runs both ways. | **substantive** |
> | 2026-11-04 | `Blocked-on` cleared (`MILE-98` is Done); `Status` stays Open, and the fix needs a design decision. **Why:** the declared model supplies the fields (`Version`) and the revision log's identity (`revision_log`, `MILE-167`), but the rule is a diff question: it must compare a record's revision rows and `Version` against the base, and `check --base` hands rules only the changed paths (`diff_gate.rs`), not the base text. Building it means a new diff-content input from the caller, as `embodiment.consistency` has for history, and a choice of which change classes require the bump and how that is declared. That is a rule-vocabulary decision (`ADR-53` left the function set open) for the maintainer. | **substantive** |
> | 2026-11-04 | `Status: Open` -> `Fixed`; `Blocked-on` removed; `Regression-test` filled. **Why:** maintainer decision of 2026-11-04 chose a config-driven, diff-aware rule over a hardcoded comparison; it is built, declared on `spec` in this repository's config and exercised by `check --base origin/main`. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
