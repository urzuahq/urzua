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

- `urzua fix --apply`, `fix --rebaseline --apply`, `reverify --apply` and `waive --apply` each refuse a record under a path gate, with a notice naming it, shown by a test per command. — evidence: test:rust/crates/urzua-cli/tests/fix_paths_integration.rs#a_record_under_a_path_gate_is_refused_to_an_agent_and_written_for_a_person_in_its_group@sha256:d7212b9122c8691490e4af73376f8b54fdde809bde9787bb2bdbf55f835ad89b, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#a_record_under_a_path_gate_is_not_re_pinned_for_an_agent@sha256:ae96ce6413f04e040b7bfdc316b165cec457baf35e81a59d4ea6731bacc16c83, test:rust/crates/urzua-cli/tests/reverify_integration.rs#a_record_under_a_path_gate_is_not_renewed@sha256:cf655f9edf84111f1cf1484f1c64fdd5cbb5db2b750d7afb01ac556afae8d285, test:rust/crates/urzua-cli/tests/waive_apply_integration.rs#drafts_into_a_directory_under_a_path_gate_are_refused@sha256:8ef36fded35a4810cdef682d58c15962b5e3e700a7e3a048ac10be0876314ef0, code:rust/crates/urzua-core/src/authority.rs#path_gate_refusal@sha256:20193936e77ed99cc94396a01c0582be4b3f5472b72a7e69078274c5becb9704
- The refusal says who can make the change. — evidence: test:rust/crates/urzua-cli/tests/fix_paths_integration.rs#a_record_under_a_path_gate_is_refused_to_an_agent_and_written_for_a_person_in_its_group@sha256:d7212b9122c8691490e4af73376f8b54fdde809bde9787bb2bdbf55f835ad89b, code:rust/crates/urzua-core/src/authority.rs#path_gate_refusal@sha256:20193936e77ed99cc94396a01c0582be4b3f5472b72a7e69078274c5becb9704
- A repository with no path gate behaves as before. — evidence: test:rust/crates/urzua-cli/tests/fix_paths_integration.rs#without_a_path_gate_an_agent_fix_applies_as_before@sha256:8522f5bfe85a07cb754ac52bab6a3a1a245c48e0a7df34ac02d16c8e6794bc9c, code:rust/crates/urzua-core/src/authority.rs#path_gate_refusal@sha256:20193936e77ed99cc94396a01c0582be4b3f5472b72a7e69078274c5becb9704

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (3 of 3 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
> | 2026-11-16 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 6 locators pinned, 0 file-level, 6 scoped. Evidence: rust/crates/urzua-cli/tests/fix_paths_integration.rs#a_record_under_a_path_gate_is_refused_to_an_agent_and_written_for_a_person_in_its_group@sha256:d7212b9122c8, rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#a_record_under_a_path_gate_is_not_re_pinned_for_an_agent@sha256:ae96ce6413f0, rust/crates/urzua-cli/tests/reverify_integration.rs#a_record_under_a_path_gate_is_not_renewed@sha256:cf655f9edf84, rust/crates/urzua-cli/tests/waive_apply_integration.rs#drafts_into_a_directory_under_a_path_gate_are_refused@sha256:8ef36fded35a, rust/crates/urzua-core/src/authority.rs#path_gate_refusal@sha256:20193936e77e, rust/crates/urzua-cli/tests/fix_paths_integration.rs#without_a_path_gate_an_agent_fix_applies_as_before@sha256:8522f5bfe85a Reason: Pinned the tests and code each delivered Done-means item cites, as they stand in this pull request. **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
