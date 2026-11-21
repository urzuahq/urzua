---
Stable-Id: 01M7W8EZ84B0CNEPGQY72HFJQZ
Status: Fixed
Found-in: review of pull request #616 (section.decided-body-changed), 2026-11-20: a prefix-lines record's rewrite went unexamined; a check of the decider gate on a prefix-lines fixture reproduced the same read.
Realized-by: test:rust/crates/urzua-cli/tests/authority_integration.rs#a_prefix_lines_records_base_status_is_read_as_the_loader_reads_it@sha256:307afbfbb87f016af2571b86a33cd4f9c2817685688e26f437cd74e317eecf89, test:rust/crates/urzua-cli/tests/version_bump_integration.rs#a_prefix_lines_records_base_version_is_read_as_the_loader_reads_it@sha256:34c0917627806e4da6c600915c4ca5314a310d11d0a1943c91c7a98d60a4fe21, test:rust/crates/urzua-cli/tests/diff_scope_integration.rs#a_deleted_prefix_lines_records_locators_pull_in_the_record_sharing_one@sha256:51b99d0de3b225ea89b108f6fdb4448430bc143bf3885dc4059812b2acf9c021, test:rust/crates/urzua-cli/tests/diff_scope_integration.rs#a_prefix_lines_waivers_base_scope_is_read_so_a_rescope_reports_the_finding@sha256:b28cfda3f81f6d26f89e16c47a441c08b4ad2c3f5ba39e8610db3b53598adea2, test:rust/crates/urzua-cli/tests/children_agreement_integration.rs#a_prefix_lines_childs_base_status_is_read_so_a_body_change_does_not_pull_in_the_parent@sha256:9fbe582bba8cb7b5f50a04a73c91b6282bc63532ed853b8e25f2e48d7d24a0b8
---
# 213 — A record's merge-base copy is read with header_shape alone, so a prefix-lines type's base header reads as unreadable

## What was wrong

A record type that declares `fields: {from: prefix-lines}` has no `---` front matter, but three places that read the merge base's copy of a record parse it with `header_shape` alone, which defaults to YAML front matter. The base header then has no region and reads as unreadable, so the check that needed it silently does less:

- `rust/crates/urzua-core/src/rules/decided_body.rs`, `judge` (`section.decided-body-changed`): the base status was unreadable, so a rewrite of an Accepted prefix-lines record was counted unexamined and not reported. Fixed in the pull request that found it, by reading the copy through `record_in_text`.
- `rust/crates/urzua-cli/src/diff_scope.rs`, `committed_statuses` (the decider gate, `diff.decider-required`): reproduced on a prefix-lines `adr` type with a gate on `Accepted`. A move from Proposed to Accepted is reported as "the merge base's copy could not be read, so it is treated as a move", not as a move from Proposed.
- `rust/crates/urzua-core/src/rules/revision_log.rs`, `judge_version_bump` (`revision-log.version-bump-required`): reads the base copy with `parse_with_shape_and_prefix` and compares the declared version field, so on a prefix-lines type the base value reads as absent. Read from the code; not reproduced.

Other sites that read a base or older copy of a record were not audited.

## Sites audited

Every place in `urzua-core` and `urzua-cli` that rebuilds a `Record` from a merge-base or older commit's text now reads it through `Record::parse_declared` (or `Record::parse_as_type` with the merge base's own type configuration), which reads it as the loader does:

- `urzua-cli/src/diff_scope.rs`: `committed_statuses` (the decider gate), `deleted_record_facts` (a deleted record's cited locators were lost, so a record sharing one was not a dependent), `touched_waiver_scopes` (a changed waiver's base `Scope` was lost, so the finding it stopped waiving was not reported), `parents_of_moved_children` (a child's base status read absent, so any change to it pulled its parent in as moved) and `deleted_records` (already read the model; routed through the shared function).
- `urzua-core/src/rules/revision_log.rs`, `judge_version_bump`.
- `decided_body.rs`, `claim_expiry.rs` and `pin_attestation.rs`, which used the private `record_in_text`; it is removed.

Ruled out: `init.rs` and `new_record.rs` parse YAML front matter on purpose (detecting a shape, reading a scaffold `urzua new` wrote); the loader (`discovery.rs`) reads with the declared model and reports a model that does not resolve as a load error; the remaining `parse_with_shape*` and `Record::parse` calls are in unit tests.

## Why nothing caught it

Each site rebuilt a `Record` from text itself instead of through one function that reads a record as the loader does (`Record::parse_with_model` with the type's declared model). `record_in_text` in `claim_expiry.rs` is that function but is private to the rules module. No test used a prefix-lines type for a base-copy read.

## References

`record_in_text` (`rust/crates/urzua-core/src/rules/claim_expiry.rs`) and `resolved_model` (`rust/crates/urzua-cli/src/discovery.rs`) read a record as the loader does; the shared fix is to route every base-copy read through one such function and add a prefix-lines case for each site.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-21 | Fixed: every base-copy read routes through `Record::parse_declared`; a prefix-lines test per site, each observed failing before the fix; the audited sites listed under Sites audited. | **structural** |
> | 2026-11-21 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 5 locators pinned, 0 file-level, 5 scoped. Evidence: rust/crates/urzua-cli/tests/authority_integration.rs#a_prefix_lines_records_base_status_is_read_as_the_loader_reads_it@sha256:307afbfbb87f, rust/crates/urzua-cli/tests/version_bump_integration.rs#a_prefix_lines_records_base_version_is_read_as_the_loader_reads_it@sha256:34c091762780, rust/crates/urzua-cli/tests/diff_scope_integration.rs#a_deleted_prefix_lines_records_locators_pull_in_the_record_sharing_one@sha256:51b99d0de3b2, rust/crates/urzua-cli/tests/diff_scope_integration.rs#a_prefix_lines_waivers_base_scope_is_read_so_a_rescope_reports_the_finding@sha256:b28cfda3f81f, rust/crates/urzua-cli/tests/children_agreement_integration.rs#a_prefix_lines_childs_base_status_is_read_so_a_body_change_does_not_pull_in_the_parent@sha256:9fbe582bba8c Reason: First pins for the prefix-lines tests that prove BUG-213 fixed, each observed failing before the fix **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
