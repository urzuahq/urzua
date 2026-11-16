---
Stable-Id: 01M7FW5X1S92KDD4FAABC33847
Status: Done
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 266 — Every next action and every documented command runs as printed

## What

Make each `next_action` and each command in the documentation run unchanged: include `--actor` where an apply needs it, suggest staging an untracked record, and suggest a re-baseline for drift.

## Why

A walkthrough of 2026-11-15 followed only the user documentation with the released binary. `fix --apply` as printed by `next_action` and by three documentation pages fails for want of `--actor`; after `urzua new`, `check` is ok having examined no file; and for a changed pinned file the suggested action wrote a drift state and turned the check green instead of pointing at the re-baseline.

## Done means

- A test runs the `next_action` of every finding variant in a fixture and each command exits without a usage error. — evidence: test:rust/crates/urzua-cli/tests/runnable_actions_integration.rs#every_suggested_action_and_every_next_action_runs_as_printed@sha256:01cc795070fcf29b2a6d70576626977ad921fe82e6719788a757fada5dd4faff, test:rust/crates/urzua-cli/tests/runnable_actions_integration.rs#every_run_command_variant_is_covered_by_a_fixture_or_exempt_with_a_reason@sha256:97c8ba5befe84ea42ff3cdd242d9be7012f043d54639c526a5db4c162eff80e3
- A test extracts the commands in the reference pages and runs them against a fixture. — evidence: test:rust/crates/urzua-cli/tests/doc_commands_integration.rs#every_documented_urzua_command_runs_against_a_fixture@sha256:fa251038b804cefb6d3f83be87228338a2279c6d0cfcf36508063afe4586e938
- `check` on a repository whose only record is untracked says so and suggests staging it. — evidence: test:rust/crates/urzua-cli/tests/runnable_actions_integration.rs#a_new_record_is_reported_untracked_until_staged_then_examined@sha256:046946515ce7ddd5b8bde7886c204b85c40a8fa152adb60e4850019fb5f010a8
- The next action for a changed pinned file is the re-baseline listing. — evidence: test:rust/crates/urzua-cli/tests/runnable_actions_integration.rs#a_changed_pinned_file_leads_to_the_repin_and_cannot_go_green_without_one@sha256:2f86aab29e2710726ce07489de82355cbb86c5fc7615c359ff05031f0d305a2e

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (4 of 4 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
> | 2026-11-16 | Delivered in #548: each Done-means item cites the test that shows it, and `urzua fix --apply` computed `Status: Done` from that evidence. **Why:** the PR that delivers a milestone records it. | **structural** |
