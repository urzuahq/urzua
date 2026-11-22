---
Stable-Id: 01M7W0B0GHBSAKN5YPJ179YXHW
Status: Done
Phase: '1'
Track: harness
Blocked-on:
---
# 354 — harness status reports a bundle hook entry missing from the lock as not wired

## What

The session-start check (`hook_entry_problems` in `rust/crates/urzua-cli/src/harness/run.rs`) reports a hook entry the
current bundle wants that the lock does not list, as it already reports a locked entry that is not wired. Today it walks
only the lock's entries, so a repository installed before a release that adds an entry (the two AskUserQuestion entries
of `ADR-105`) hears nothing at session start; `urzua harness status` does report it.

## Why

Found by an independent review of #612 on 2026-11-20. The failure is safe (without the question hooks the gated move
stays refused), but the person is not told why the question route never records an answer until they run
`urzua harness status`.

## Done means

- In a repository whose lock predates a bundle hook entry, the session-start check names that entry as not wired and says to run `urzua harness install`; shown by a binary test. — evidence: test:rust/crates/urzua-cli/tests/harness_integration.rs#session_start_names_a_bundle_hook_entry_the_lock_predates@sha256:dd8f9e9918c84f49e726c4f964292822cb2014008229034c3f6c6a6b61fe08ff
- A repository whose lock and settings match the bundle reports nothing new; shown by the existing session-start tests passing unchanged. — evidence: test:rust/crates/urzua-cli/tests/harness_integration.rs#session_start_is_quiet_when_ready_and_names_each_thing_that_would_fail_open@sha256:7c58b4d9df735fc9f137cd30d3414f22ab9966f8ada3fe38b162c8a2607144bc, test:rust/crates/urzua-cli/tests/drafting_commands_integration.rs#this_repository_runs_on_the_installed_bundle_with_no_drift_and_no_hook_scripts@sha256:275c976f4290072d9663d9358c2eb6ea652ab9b01b1a58feab72b85b6e556a5a

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-20 | Filed. **Why:** an independent review of #612 found that the session-start check walks only the lock's entries. | **substantive** |
> | 2026-11-22 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 0 file-level, 3 scoped. Evidence: rust/crates/urzua-cli/tests/harness_integration.rs#session_start_names_a_bundle_hook_entry_the_lock_predates@sha256:dd8f9e9918c8, rust/crates/urzua-cli/tests/harness_integration.rs#session_start_is_quiet_when_ready_and_names_each_thing_that_would_fail_open@sha256:7c58b4d9df73, rust/crates/urzua-cli/tests/drafting_commands_integration.rs#this_repository_runs_on_the_installed_bundle_with_no_drift_and_no_hook_scripts@sha256:275c976f4290 Reason: harness_integration.rs drives the built binary: the new test removes the AskUserQuestion entries from lock and settings and observes two hook-entry findings (failed before the change), the existing quiet-session test is unchanged, and the drift test shows this repository's own bundle matches **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-22 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (2 of 2 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
