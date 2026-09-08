# 82 — One shared emit_json helper instead of seven inline copies

> Status: Planned
> Phase: 1
> Track: schema-governance
> Implements: ADR-23

## What

Collapse the seven call sites in `urzua-cli/src/main.rs` that each independently do
`println!("{}", serde_json::to_string_pretty(&X).unwrap())` into one shared function (e.g.
`emit_json<T: Serialize>(value: &T) -> `). Today only `check`'s version has a name
(`print_report`); the other six are copy-pasted inline, each printing a different ad hoc struct.

## Why

Found live while fixing BUG-4 (`doctor` printing plain text instead of JSON): the fix needed a JSON
emission path for `doctor`, and checking what already existed found the exact same one-line pattern
duplicated seven times across the command surface, with no single reusable function most commands
actually call. Fixing `doctor` in isolation would have made it an eighth copy rather than closing
the underlying gap. Scoped out of BUG-4 itself (kept that fix narrow, per SPEC-9's own retrospective
scope) and tracked here instead, since it's a real, separate piece of generic tooling work, not part
of what was actually broken in `doctor`.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
