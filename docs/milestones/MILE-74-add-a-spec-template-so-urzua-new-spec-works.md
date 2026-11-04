---
Status: Done
Stable-Id: 01M1YSM7BNRXXTYMV4TF93EAG4
Phase: '0'
Track: roadmap-tracking
Implements: SPEC-1
Blocked-on: —
Realized-by: code:.urzua/templates/spec.md, test:rust/crates/urzua-cli/tests/new_integration.rs
---
# 74 — Add a spec template so urzua new spec works

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

## Done means

`.urzua/templates/spec.md` exists and `urzua new spec` writes a record that passes `check` (it exits 0 rather than failing for want of a template); the canonical spec field set the template carries is decided, with `Author` required, and `new_integration.rs` covers it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | `Status: Done`. The mechanical failure was already resolved as a side effect of ADR-33's yaml-frontmatter migration (`spec`'s `header_shape` got declared, so `urzua new spec` stopped erroring). The real remaining scope -- the canonical field set -- decided: `Author` required (MILE-78's accountability argument, backfilled onto all 16 pre-existing specs lacking it); `Embodiment`/`Derives-from` a project-wide optional convention, not `SPEC-1`-specific baggage; `Implements`/`Parent` declared in `known_fields` since every spec already carries them. `.urzua/templates/spec.md` built so `urzua new spec` produces real section scaffolding, not a bare skeleton. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
