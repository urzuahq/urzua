---
Stable-Id: 01M2Q4SRD96G32D4S66VNA3X9S
Status: Planned
Phase: '1'
Date: 2026-09-17
Author: beauwilliams
Derives-from: ADR-53
---
# 95 — Rule presets: `extends`, so an adopter does not hand-write every rule

## What

A named starting set a config can inherit -- `extends: recommended` -- instead of naming every rule
individually. `ESLint` and `Spectral` both settled on this shape, with `off`/`all`/`recommended`
modifiers layered over it.

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

Note the failure mode a preset reintroduces: a named set is governance the engine ships. It stays
honest only while `extends` is a *starting point a config can override*, never a floor it cannot get
below -- `off` has to be reachable for every rule in any preset.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Filed. **Why:** `ADR-53`'s opt-in decision leaves every adopter writing the full rule list, and `MILE-80` shipped `init`-proposes-all-at-`warn` as a stand-in rather than leaving that unaddressed. Naming the stand-in as a stand-in, so it is not mistaken for the answer. | **substantive** |
