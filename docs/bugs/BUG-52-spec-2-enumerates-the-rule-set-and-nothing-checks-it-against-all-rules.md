---
Stable-Id: 01M2W0QAD0PBVS3XD05RQQTJ4W
Status: Open
Found-in: 'A code review of PR #74 -- SPEC-2 called itself the complete, current rule set while understating it by four'
Regression-test: 'not yet written -- a rule in `ALL_RULES` absent from the spec that enumerates them must be a finding, and the reverse must be too'
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
