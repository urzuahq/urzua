# 74 — Add a spec template so urzua new spec works

> Status: Planned
> Stable-Id: 01M1YSM7BNRXXTYMV4TF93EAG4
> Phase: 0
> Track: roadmap-tracking
> Implements: SPEC-1

## What

Add `.urzua/templates/spec.md` (or declare `header_shape = "yaml-frontmatter"` on `spec` in
`.urzua/config.toml`) so `urzua new spec "..."` actually works. `adr`, `rfc`, `bug`, and `milestone`
all have a template; `spec` has neither a template nor a synthesizable shape declared, so `urzua new
spec` fails outright with exit 2, "no template ... and no synthesizable shape declared." Every one
of this repo's 7 specs (SPEC-1 through SPEC-7) was hand-authored -- never created through the tool.

## Why

Found live, immediately, trying to scaffold SPEC-7 with `urzua new spec` and hitting the exit-2
error. This is a real gap in bootstrapping the tool with itself: `milestone` and `bug` were both
built specifically to be dogfooded this way, but `spec` -- one of the three original record types
this project is built around -- can't be created through `urzua new` at all.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
