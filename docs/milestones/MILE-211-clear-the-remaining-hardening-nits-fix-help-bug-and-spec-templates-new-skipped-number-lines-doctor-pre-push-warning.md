---
Stable-Id: 01M6SSWJYRSZGQ0QWJYPMXXTF9
Status: Planned
Phase: '1'
Track: user-docs
Blocked-on: null
---
# 211 — Clear the remaining hardening nits: fix help, bug and spec templates, new skipped-number lines, doctor pre-push warning

## What

Clear four small leftovers of Phase 0 hardening: the stale `fix` help text, the `bug` and `spec` templates that show the wrong statuses and a stray toml block, the one stderr line per skipped number in `urzua new`, and the `doctor` warning for a missing pre-push hook where only a pre-commit hook is wired.

## Why

They are nits from the adopter end-to-end second pass (N8, M5 rest) and the first-pass cli and config reviews (`fix --help`, `templates/bug.md`, `templates/spec.md`), still open on main at `4c5de6f1` with no record. `new` prints 91 lines on a clone that holds 91 waivers and is killed by SIGPIPE when piped through `head`.

## Done means

- `fix --help` describes what `fix` applies today.
- `templates/bug.md` lists the statuses the `bug` type declares.
- `templates/spec.md` has no toml block.
- The harness lock is refreshed.
- `urzua new` prints one line naming the skipped range, not one per number; a test fails before the change.
- `doctor` warns only for a hook the repository's configuration says it wires, and a repository wiring only pre-commit gets no pre-push warning.
- A patch changeset names the CLI changes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Initial record. **Why:** deferred during Phase 0 hardening and tracked by no record. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
