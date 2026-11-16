---
Stable-Id: 01M7HW08E0S177BKC2D09NFNC9
Status: Planned
Phase: '1'
Track: accountability-identity
Blocked-on: null
---
# 289 — fix, reverify and waive never write a record under a path gate

## What

Make every command that writes records on someone's behalf refuse to write a record the authority table protects by path, as the edit hook does.

## Why

Found on 2026-11-16 while urzua's own builders and reviewers used the tool to land a week of pull requests. A builder reproduced it: with a path gate on a record, `urzua fix --apply --actor agent` rewrote its Embodiment, because the write path checks status gates and never path gates. The edit hook refuses the same write, so an agent can change a protected record by running a command.

## Done means

- `urzua fix --apply`, `fix --rebaseline --apply`, `reverify --apply` and `waive --apply` each refuse a record under a path gate, with a notice naming it, shown by a test per command.
- The refusal says who can make the change.
- A repository with no path gate behaves as before.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
