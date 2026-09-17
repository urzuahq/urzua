---
Stable-Id: 01M2Q4SRD96G32D4S66VNA3X9S
Status: Planned
Phase: '1'
Track: schema-governance
Implements: —
Blocked-on: —
---
# 95 — Rule presets: `init --preset`, so an adopter does not hand-write every rule

## What

Follows `ADR-53`.

A named starting set `urzua init` can **generate a config from** -- `urzua init --preset recommended`
-- instead of an adopter naming every rule by hand.

**A preset is an init-time generator, not a runtime inheritance.** `ESLint` and `Spectral` both resolve
an `extends` chain while linting; this deliberately does not. The preset's only job is to write a
config file, and once written that file is ordinary editable text with no upstream. Nothing reads a
preset at `check` time, and `check` has no notion that one was ever used.

## Why

`ADR-53` made every rule opt-in, which is right and is not free: a config that names nothing runs
nothing, so **every adopter hand-writes the full rule list before the tool does anything.** This
repo's own `.urzua/config.yaml` now carries 18 lines that are almost entirely "yes, the obvious
ones".

`MILE-80` shipped a deliberate stand-in rather than leaving the gap open: `urzua init` proposes every
rule at `warn`. That is discoverable and cannot hand a corpus blocking errors it never asked for
(`MILE-51`), but it is a preset by another name -- one value, hardcoded, chosen by `init` rather than
declared anywhere. Exactly the shape `ADR-53` argues against, at one remove.

## The open question this milestone owns

Whether the shipped set is **minimal** (a few checks nothing could object to), **recommended** (the
ones this project would defend), or **all-advisory** (today's `init` behaviour, made explicit). The
research behind `ADR-53` argued presets should land *in the same release as* opt-in, precisely because
opt-in without them pushes the full list onto every adopter. They did not, and this milestone is that
debt, named rather than left implicit.

Generating rather than inheriting is what keeps this consistent with `ADR-53`. A runtime `extends`
would be governance the engine supplies on every run, and would need care to stay a starting point
rather than a floor. A generator has no such problem: the output is a file the adopter owns, edits and
can delete lines from, and the engine never sees where it came from.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Filed. **Why:** `ADR-53`'s opt-in decision leaves every adopter writing the full rule list, and `MILE-80` shipped `init`-proposes-all-at-`warn` as a stand-in rather than leaving that unaddressed. Naming the stand-in as a stand-in, so it is not mistaken for the answer. | **substantive** |
