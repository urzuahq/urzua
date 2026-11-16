---
Stable-Id: 01M7HW08E0S177BKC2D09NFNC9
Status: Done
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

- `urzua fix --apply`, `fix --rebaseline --apply`, `reverify --apply` and `waive --apply` each refuse a record under a path gate, with a notice naming it, shown by a test per command. — evidence: test:rust/crates/urzua-cli/tests/fix_paths_integration.rs#a_record_under_a_path_gate_is_refused_to_an_agent_and_written_for_a_person_in_its_group, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#a_record_under_a_path_gate_is_not_re_pinned_for_an_agent, test:rust/crates/urzua-cli/tests/reverify_integration.rs#a_record_under_a_path_gate_is_not_renewed, test:rust/crates/urzua-cli/tests/waive_apply_integration.rs#drafts_into_a_directory_under_a_path_gate_are_refused, code:rust/crates/urzua-core/src/authority.rs#path_gate_refusal
- The refusal says who can make the change. — evidence: test:rust/crates/urzua-cli/tests/fix_paths_integration.rs#a_record_under_a_path_gate_is_refused_to_an_agent_and_written_for_a_person_in_its_group, code:rust/crates/urzua-core/src/authority.rs#path_gate_refusal
- A repository with no path gate behaves as before. — evidence: test:rust/crates/urzua-cli/tests/fix_paths_integration.rs#without_a_path_gate_an_agent_fix_applies_as_before, code:rust/crates/urzua-core/src/authority.rs#path_gate_refusal

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (3 of 3 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
