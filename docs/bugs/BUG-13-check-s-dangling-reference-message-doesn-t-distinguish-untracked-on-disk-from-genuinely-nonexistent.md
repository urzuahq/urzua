---
Stable-Id: 01M260GGTATCK7PSKRQY2K7T0M
Status: Fixed
Found-in: 'hit live drafting ADR-45 while SPEC-20 existed on disk but was not yet `git add`ed -- ADR-45''s own Implements/References pointer to SPEC-20 reported `relation.supersession-reciprocity`/`pointer.resolution` findings identical in wording to a reference to a record that does not exist anywhere, even though the file was sitting right there in the working tree'
Realized-by: test:rust/crates/urzua-core/src/rules/pointer.rs#a_reference_to_an_untracked_file_on_disk_is_its_own_variant_observed_failing@sha256:337fc9daaa606146097d2f6be441af7c559b64c9771c2ff308c4f49373e38746, test:rust/crates/urzua-core/src/rules/relation.rs#a_supersession_to_an_untracked_file_on_disk_is_its_own_variant_observed_failing@sha256:9c255d4945cd926b5231ad68bd4d5fe5ca9ed422445e5b39563d3e7652e5f10a, test:rust/crates/urzua-core/src/rules/embodiment.rs#a_locator_naming_an_untracked_file_on_disk_is_its_own_variant_observed_failing@sha256:7b6cf41bea7f90523738a94b0a46dd295b1049ce2b511b54aa04f924f505edf6, test:rust/crates/urzua-cli/tests/check_integration.rs#a_reference_to_an_untracked_file_is_told_apart_from_a_missing_one_observed_failing@sha256:26fc3b877fa8a878186592b97b8367ca55f68f67358b6bba713ad41a549ca4e3
---
# 13 — check's dangling-reference message doesn't distinguish untracked-on-disk from genuinely nonexistent

## What was wrong

`urzua check` (correctly, by design -- ADR-6's git-tracked-only discovery) only examines git-tracked
and staged files, never a raw filesystem walk. A reference to a record that exists on disk but isn't
yet staged reports the exact same message as a reference to a record that doesn't exist anywhere:
`"<field>: <reference> does not resolve to any discovered record"` /
`"does not resolve to any discovered record"`. A person (or agent) actively drafting two
cross-referencing records has no way to tell, from the message alone, whether they made a real typo
or just forgot `git add` on the target -- both read identically as "this doesn't exist."

## Why nothing caught it

No test exercises this specific distinction -- every existing `pointer_resolution`/
`supersession_reciprocity` test constructs its fixture records directly in memory (never touching a
real filesystem or git index at all), so there was never a scenario where "discovered" and "exists
on disk" could actually diverge in a test. The gap is architectural, not just a missing test case:
`urzua-core`'s rule functions are deliberately pure, no filesystem or git access (the same purity
boundary `ADR-5`/`ADR-6` enforce, checked by `purity.rs`'s own compile-time dependency guard) --
they only ever see the `records` slice `urzua-cli`'s discovery layer hands them. Distinguishing
"genuinely doesn't exist" from "exists on disk but wasn't discovered" needs information (a real
filesystem stat, or knowledge of what git considers untracked) that the pure rule layer structurally
cannot have. A fix has to live at the `urzua-cli` boundary -- either enriching a dangling finding's
message after the fact by checking whether any candidate path matching the reference exists on disk,
or making `discover_tracked_files` itself distinguish "not tracked" from "doesn't exist" and passing
that through -- not a change to any pure rule function.

## Decision (2026-11-05)

The maintainer decided: add a separate finding variant for a cited path that exists on disk but is not tracked by git, distinct from one that does not exist, each with its own `ADR-73` action; the CLI supplies the on-disk fact so core stays pure.

Fixed: `urzua-cli` (`discovery::untracked_records`) reads the files directly in each record type's directories that git does not track, loads them with the loader's own identity rules and hands their ids to the rules (`RuleCtx::untracked_records`, a parameter of the four reference-resolving rules). `pointer.resolution`, `relation.supersession-reciprocity`, `relation.pair-reciprocity` and `sequence.unresolved-blocker` emit `pointer-untracked`, `supersession-untracked`, `pair-untracked` and `blocker-untracked` with a message naming the file; `embodiment.locator-exists` emits `locator-untracked` for a cited path that is on disk and untracked. Actions: a missing target keeps `edit-field` `set`; an untracked one is exempt with a reason, because the repair is `git add` and neither `edit-config` nor `run-command` fits (`run-command`'s `argv[0]` is `urzua`, `ADR-73`). `SPEC-2` states it, with the regenerated action table.

## References

- `rust/crates/urzua-core/src/rules.rs` -- `pointer_resolution`, `supersession_reciprocity`, whose
  dangling-reference messages this bug is about.
- `rust/crates/urzua-io/src/lib.rs` -- `discover_tracked_files`, the git-tracked-only discovery
  boundary this fix would need to extend past, without violating it for the actual check itself.
- ADR-6 -- the git-tracked-only discovery decision this bug does not propose changing, only wants a
  clearer message about.
- ADR-5 -- the `urzua-core` purity boundary explaining why this can't be a pure-rule-layer fix.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-10 | Initial bug record, `Status: Open`. Not yet fixed -- exact mechanism (post-hoc message enrichment in `urzua-cli` vs. a richer discovery-layer signal threaded through) not yet decided. | **structural** |
> | 2026-11-04 | Triage: still reproduces on main; `Status` stays `Open`. In a repository where `ADR-3` has `Derives-from: ADR-9` and `docs/adr/ADR-9-untracked.md` exists but is not `git add`ed, `urzua check` reports `Derives-from: ADR-9 does not resolve to any discovered record`, identical to a reference to a record that exists nowhere; after `git add` it reports nothing. **Why:** the fix is at the `urzua-io` discovery boundary and the record leaves its shape (enrich the message, or make discovery distinguish untracked from absent) undecided. | **substantive** |
> | 2026-11-05 | Fixed. **Why:** maintainer decision 2026-11-05: an untracked-on-disk target is its own finding variant with its own `ADR-73` action, the on-disk fact supplied by the CLI. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 4 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/embodiment.rs#a_locator_naming_an_untracked_file_on_disk_is_its_own_variant_observed_failing` (lines 1168-1204) 116d0d13f401 -> 7b6cf41bea7f, `rust/crates/urzua-cli/tests/check_integration.rs#a_reference_to_an_untracked_file_is_told_apart_from_a_missing_one_observed_failing` (lines 769-844) ae0f4584daa3 -> 26fc3b877fa8. Reason: Read the diff of each pinned span since the old pin and the claim still holds. embodiment.rs#a_locator_naming_an_untracked_file_on_disk_is_its_own_variant_observed_failing: the call to the locator checker gained one more empty-slice argument, assertions unchanged; check_integration.rs#a_reference_to_an_untracked_file_is_told_apart_from_a_missing_one_observed_failing: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
