---
Stable-Id: 01M6HZ0N6A83BE6C89RP8F46S9
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Realized-by: code:rust/crates/urzua-core/src/rules/index.rs@sha256:f4617f7707e2718418818a8ed0de0f0f2f1e9f80b93b13b661720810cd3925e2, test:rust/crates/urzua-cli/tests/index_freshness_integration.rs#a_missing_record_an_extra_entry_and_a_stale_title_each_block@sha256:6bf98d566e70051bbcb648d3ea75c2c1286b0682c16f56d4ba5451bc2a340d92, test:rust/crates/urzua-cli/tests/index_freshness_integration.rs#a_missing_index_file_and_a_removed_marker_pair_block@sha256:b9cd4217f0bcbc82fdd8ef13539eddec474e24ec6f138a5df4075d612302f467, test:rust/crates/urzua-cli/tests/index_freshness_integration.rs#index_regenerates_a_declared_index_and_a_second_run_is_a_no_op_end_to_end@sha256:7046d1eaf20f875864e3d251792ff2156105132c5865fe4e2d7ff1ba511f003e, test:rust/crates/urzua-cli/tests/index_freshness_integration.rs#a_current_index_passes_and_the_census_counts_it@sha256:4c4df67647a08350cd054f50580dbf02486d706d18891132a7914721059c90eb
---
# 139 — Generated index files and README index lists must match the records

## What

A generated artifact must be current: where a repository keeps generated index files (a structured index file in a record directory) or a list of records between markers in a README, `check` regenerates the expected content from the records and fails when the committed copy differs. A missing file, a stale entry and a removed marker pair are each an error.

## Why

The adopter regenerates two indexes per record directory in its audit and its linter then verifies they are fresh, and for its proposals that freshness check is the only structural check applied. `MILE-42` (an `INDEX.json`) and `MILE-115` (`export`) serve discovery by tools, not a freshness gate, so a repository that commits an index has nothing here that stops it going stale. The alternative, deleting the committed indexes and deriving them with `export`, is available to an adopter but is its choice; the engine must support the checked-in case.

## Done means

- A record added, removed or retitled without regenerating the index fails `check` with an error naming the stale artifact and the differing entry. — evidence: code:rust/crates/urzua-core/src/rules/index.rs@sha256:f4617f7707e2718418818a8ed0de0f0f2f1e9f80b93b13b661720810cd3925e2, test:rust/crates/urzua-cli/tests/index_freshness_integration.rs#a_missing_record_an_extra_entry_and_a_stale_title_each_block@sha256:6bf98d566e70051bbcb648d3ea75c2c1286b0682c16f56d4ba5451bc2a340d92
- A missing artifact and a removed marker pair each fail. — evidence: test:rust/crates/urzua-cli/tests/index_freshness_integration.rs#a_missing_index_file_and_a_removed_marker_pair_block@sha256:b9cd4217f0bcbc82fdd8ef13539eddec474e24ec6f138a5df4075d612302f467
- Regenerating clears the findings. — evidence: test:rust/crates/urzua-cli/tests/index_freshness_integration.rs#index_regenerates_a_declared_index_and_a_second_run_is_a_no_op_end_to_end@sha256:7046d1eaf20f875864e3d251792ff2156105132c5865fe4e2d7ff1ba511f003e
- The rule reports how many generated artifacts it compared. — evidence: test:rust/crates/urzua-cli/tests/index_freshness_integration.rs#a_current_index_passes_and_the_census_counts_it@sha256:4c4df67647a08350cd054f50580dbf02486d706d18891132a7914721059c90eb

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter treats a missing or stale generated index, or a removed README marker pair, as an error, and urzua has no equivalent gate. | **substantive** |
> | 2026-11-04 | Narrowest reading built as the opt-in rule `index.freshness` (`SPEC-2`): each declared artifact is a file, or a marked list inside a README, in one declared line format; the engine compares the committed entries with the records and does not regenerate them or compare order. A structured (JSON) index is not read; only line formats. Status unchanged. **Why:** the milestone asks for the freshness gate, and a line format covers both artifacts without the engine owning an index file schema. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #171: opt-in rule `index.freshness` compares declared index artifacts with the records. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/rules/index.rs@sha256:01dc6447b569 Reason: Pinned the files as they are now after checking each against the record it evidences: status_model.rs and rules/status.rs still declare and judge the per-type state machine (MILE-132), acceptance.rs and the corpus-h gate expectation still hold the held-source case (MILE-133), rules/index.rs still compares generated indexes with the records (MILE-139), config.rs still reads field_levels and gate.rs still derives the report status (MILE-140), refs_snapshot.rs and commands/refs.rs still keep and refresh the cited-number snapshot (MILE-143), the corpus-j expectations still plant an unlisted edge and an unreachable status (MILE-144), config.rs still reads fail_on (MILE-145), claim_field_integration.rs still carries a Verifies back-pointer comment of the kind the milestone's own-corpus item names (MILE-146), main.rs and discovery.rs still read the full back-pointer id list and the decision-gate sources (MILE-147), and embodiment.rs still reads sticky_states (MILE-148). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/index.rs` 01dc6447b569 -> f4617f7707e2. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: index.rs: split index_freshness into check_index_artifact, index_region_bounds, check_region_entries, read_entry and compare_entry_to_record. Behaviour unchanged, outputs byte-identical (check, check --base and fix --rebaseline JSON compared before and after). **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
