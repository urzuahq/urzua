---
Stable-Id: 01M7FW6543EQ20WQ97EB59PGA0
Status: Done
Phase: '1'
Track: write-path
Blocked-on: null
---
# 267 — Built-in templates carry a revision log the tools can append to

## What

Ship a revision log with a first row in every built-in template, and let `fix --apply` and `reverify --apply` append to an empty log table.

## Why

A walkthrough of 2026-11-15 followed only the user documentation with the released binary. A record made by `urzua new` from the built-in decision template could not take `fix --apply` because it has no revision log, and adding an empty table failed for having no row to append after.

## Done means

- A record created by `urzua new` from each built-in profile accepts `fix --apply` with no hand edit, shown by a test per profile. — evidence: test:rust/crates/urzua-cli/tests/runnable_actions_integration.rs#an_adr_from_the_built_in_profile_takes_fix_apply_with_no_hand_edit@sha256:db5459de86e92dedfdd6b7bc2a0a5253740d59503d05555512b020e54e2d7546, test:rust/crates/urzua-cli/tests/runnable_actions_integration.rs#an_rfc_from_the_built_in_profile_takes_fix_apply_with_no_hand_edit@sha256:421ad380f07f74ea55bbb238c4a39ef51bcbd880fe042b3022274dc17a28de92, test:rust/crates/urzua-cli/tests/runnable_actions_integration.rs#a_spec_from_the_built_in_profile_takes_fix_apply_with_no_hand_edit@sha256:85fc2be8866199947d16af48fd159787ce335391e8ec9e1dbfadd3e6927e5567, test:rust/crates/urzua-cli/tests/runnable_actions_integration.rs#a_prd_from_the_built_in_profile_takes_fix_apply_with_no_hand_edit@sha256:431c4893bd9d34852d3ebd6e996473490e7f4942c138992f37f14a33ed991fb3
- An empty revision-log table receives its first row. — evidence: test:rust/crates/urzua-cli/tests/runnable_actions_integration.rs#an_empty_revision_log_table_receives_its_first_row@sha256:652ec6db9c6d8b17914fedcc5f8bd8eb3b485c21e71e24ccb64bb94541abc1f0

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (2 of 2 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
