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

- One command installs the pre-commit and pre-push hooks in a repository that has none, and `urzua doctor` then reports the hook checks as passing. — evidence: code:rust/crates/urzua-cli/src/commands/hooks.rs@sha256:e7362fcf9c7ac9597be85246bb678535518bfd8d335268baa171591ab873168b, test:rust/crates/urzua-cli/tests/hooks_install_integration.rs#default_layout_gets_both_hooks_and_doctor_then_passes@sha256:c26231da38077512339dcad799d571a9bed53ce2fd33a246e1e2e80f1b8e5f25
- The command refuses to overwrite an existing hook or a `core.hooksPath` it did not set, and says how to call urzua from it.
- A repository using the pre-commit framework can add urzua with a documented stanza, checked by a test that runs it.
- The session-start warning about missing hooks names the command. — evidence: code:rust/crates/urzua-cli/src/commands/hooks.rs#file_wiring@sha256:cfa4ac5c661ce09dec34304b2f891a461744be4cf2c223dd36a6ece5115395df, test:rust/crates/urzua-cli/tests/hooks_install_integration.rs#default_layout_gets_both_hooks_and_doctor_then_passes@sha256:c26231da38077512339dcad799d571a9bed53ce2fd33a246e1e2e80f1b8e5f25

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (2 of 4 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `InProgress`. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/hooks.rs` 8c5997f9cc84 -> 5982768ca5cf, `rust/crates/urzua-cli/tests/hooks_install_integration.rs#default_layout_gets_both_hooks_and_doctor_then_passes` (lines 101-152) 5e1df08ad802 -> c26231da3807, `rust/crates/urzua-cli/src/commands/hooks.rs#file_wiring` (lines 235-299) 54a858ba6518 -> 079857d5c405. Reason: hooks.rs: the per-hook repair command field is named fix instead of next_action (the top-level key is the output wrapper's); behaviour and messages are unchanged. hooks_install_integration.rs: the assertions read the per-check repair command from the field fix instead of next_action; the scenarios are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/hooks.rs` 5982768ca5cf -> 44def0cc7b34, `rust/crates/urzua-cli/src/commands/hooks.rs#file_wiring` (lines 339-396) 079857d5c405 -> 49de3a6d4b27. Reason: hooks.rs places the block before a final exit or exec and refuses a hook that could end first, reports an unreachable, stale-policy or duplicated block, refuses symlinked hooks and directories outside the repository, and treats a binary without hooks run as not run; the wiring check still names the command that fixes an unwired hook. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/hooks.rs` 44def0cc7b34 -> 99570c335007. Reason: hooks.rs puts the managed block at the top of a hook (after the shebang, comments and a husky v8 source line) instead of reading the hook to find its end, reports a hand-written call as wired only when it is the first command, applies install's guards in uninstall, and writes atomically; the wiring check still names the command that fixes an unwired hook. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/hooks.rs` 99570c335007 -> a24e1fa6bc88. Reason: hooks.rs puts the managed block at the top of a hook (after the shebang, comments and a husky v8 source line) instead of reading the hook to find its end, reports a hand-written call as wired only when it is the first command, applies install's guards in uninstall, and writes atomically; the wiring check still names the command that fixes an unwired hook. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/hooks.rs` a24e1fa6bc88 -> d04b45959787, `rust/crates/urzua-cli/src/commands/hooks.rs#file_wiring` (lines 372-429) 49de3a6d4b27 -> cfa4ac5c661c. Reason: hooks.rs reads only the managed block (for its own hook) as wired, reports any other mention of urzua as unmanaged-call, supports declared hooks, places the block after a husky v8 source line, refuses hard links, and writes through an exclusive temporary file; the wiring check still names the command that fixes an unwired hook. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/hooks.rs` d04b45959787 -> e7362fcf9c7a. Reason: hooks.rs reads only the managed block (for its own hook) as wired, reports any other mention of urzua as unmanaged-call, supports declared hooks, places the block after a husky v8 source line, refuses hard links, and writes through an exclusive temporary file, and names 0.4.6, the release after 0.4.5, as the first with hooks run; the wiring check still names the command that fixes an unwired hook. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
