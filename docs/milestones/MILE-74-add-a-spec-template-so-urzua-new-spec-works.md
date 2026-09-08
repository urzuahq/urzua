# 74 — Add a spec template so urzua new spec works

> Status: Planned
> Stable-Id: 01M1YSM7BNRXXTYMV4TF93EAG4
> Phase: 0
> Track: roadmap-tracking
> Implements: SPEC-1
> Blocked-on: —

## What

Add `.urzua/templates/spec.md` (or declare `header_shape = "yaml-frontmatter"` on `spec` in
`.urzua/config.toml`) so `urzua new spec "..."` actually works. `adr`, `rfc`, `bug`, and `milestone`
all have a template; `spec` has neither a template nor a synthesizable shape declared, so `urzua new
spec` fails outright with exit 2, "no template ... and no synthesizable shape declared." Every one
of this repo's specs (SPEC-1 through SPEC-6) was hand-authored -- never created through the tool.

Building the template forces a decision this repo's own corpus has ducked so far: the canonical
field set for `spec`. SPEC-1 alone carries `Embodiment`/`Author`/`Derives-from`; SPEC-2 through
SPEC-6 have none of those, just `Version`/`Date`/`Status`/`Implements`/`Parent` -- neither
`header.required-fields` (only checks for *missing* required fields; `Author` was never required
for `spec`) nor `header.layout-consistency` (checks arrangement, not which fields exist) catches
this. Decide whether `Author` becomes a required `spec` field (the same accountability argument
MILE-78 already made for ADR/RFC) and whether SPEC-1's `Embodiment`/`Derives-from` are worth keeping
project-wide or are SPEC-1-specific baggage from being hand-authored first, before this project's
own `spec` config existed.

## Why

Found live, immediately, trying to scaffold SPEC-7 with `urzua new spec` and hitting the exit-2
error. This is a real gap in bootstrapping the tool with itself: `milestone` and `bug` were both
built specifically to be dogfooded this way, but `spec` -- one of the three original record types
this project is built around -- can't be created through `urzua new` at all.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
