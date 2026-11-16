---
Stable-Id: 01M7FVRQKX17381R1VXZWAKYJ7
Status: InProgress
Phase: '1'
Track: integrations
Blocked-on: null
---
# 220 — urzua installs the git hooks that doctor asks for

## What

Add a command that wires the pre-commit and pre-push hooks `urzua doctor` reports as missing, and ship a hook definition for the pre-commit framework.

## Why

the audit of 2026-11-15 found that `doctor` warns when git hooks are missing but no command installs them; this repository wires its own with a Makefile target an adopter does not have.

## Done means

- One command installs the pre-commit and pre-push hooks in a repository that has none, and `urzua doctor` then reports the hook checks as passing. — evidence: code:rust/crates/urzua-cli/src/commands/hooks.rs@sha256:8c5997f9cc84bf2b83bb57aa568bb78b5cabb105733cbe136f8637ed564da718, test:rust/crates/urzua-cli/tests/hooks_install_integration.rs#default_layout_gets_both_hooks_and_doctor_then_passes@sha256:5e1df08ad8026374ee93621c67a1e5d45028eecdb9a00c610c9af188c8c4dd46
- The command refuses to overwrite an existing hook or a `core.hooksPath` it did not set, and says how to call urzua from it.
- A repository using the pre-commit framework can add urzua with a documented stanza, checked by a test that runs it.
- The session-start warning about missing hooks names the command. — evidence: code:rust/crates/urzua-cli/src/commands/hooks.rs#file_wiring@sha256:54a858ba6518c5ff14f1b9a2bf50e741d1feba79a1913ce826d8c75e53a1f607, test:rust/crates/urzua-cli/tests/hooks_install_integration.rs#default_layout_gets_both_hooks_and_doctor_then_passes@sha256:5e1df08ad8026374ee93621c67a1e5d45028eecdb9a00c610c9af188c8c4dd46

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (2 of 4 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `InProgress`. | **structural** |
