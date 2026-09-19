---
Stable-Id: 01M2W0QA0YMGXD5V5XBXN80PKB
Status: Open
Found-in: 'A code review of PR #74 -- SPEC-1 was rewritten by roughly 60%% with two substantive entries and its Version left at 0.3'
Regression-test: 'not yet written -- a spec gaining a `substantive` revision entry without a `Version` change must be a finding; a `structural` entry, or an unchanged spec, must not'
---
# 51 — A substantive revision-log entry does not require the `Version` bump `SPEC-18` mandates

## What is wrong

`SPEC-18` states the rule plainly:

> a substantive or structural change is a `Version` bump plus a Why-bearing revision-log entry on the
> same spec

`revision-log.change-class-required` checks the entry has a real class. **Nothing checks the other
half.** A spec can be rewritten, gain a `substantive` entry, and keep its `Version`.

It happened in the change that found it: `SPEC-1` was narrowed by roughly 60%, its `Subject` rewritten
and two `substantive` entries added, with `Version: '0.3'` untouched. Caught by review.

## Why it matters

`Version` is what a reader uses to know whether the spec they remember is the spec in front of them.
A spec whose version has not moved through several substantive rewrites is worse than one with no
version at all, because it asserts stability it does not have.

## Fix

A rule pairing the two halves `ADR-14` and `SPEC-18` already decided: a record gaining a `substantive`
revision entry in a commit that does not change its `Version` is a finding.

Needs the git-history access `embodiment.consistency` already has (`ADR-32`), since "gained an entry
in this change" is a diff question, not a document question. That is the same precomputed-by-the-caller
shape, not new I/O in `urzua-core`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** `SPEC-18` requires a `Version` bump alongside every substantive revision entry and only the entry half is checked. `SPEC-1` was rewritten by ~60% with two substantive entries and `Version` left at `0.3`; review caught it, the engine did not. | **substantive** |
