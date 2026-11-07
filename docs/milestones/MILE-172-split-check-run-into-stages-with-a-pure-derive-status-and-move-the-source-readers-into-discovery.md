---
Stable-Id: 01M6K40QR3PB1ZRMH7B9G6S3R2
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-22
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/check.rs, code:rust/crates/urzua-cli/src/gate.rs, test:rust/crates/urzua-cli/src/gate.rs#derives_each_branch@sha256:e08a23fce48a80f1981b5ffb29c4637ae07a5fbadef16aa0ff379667de8732b6, code:rust/crates/urzua-cli/src/discovery.rs, test:rust/crates/urzua-cli/tests/check_integration.rs#a_symlink_cycle_inside_a_claim_path_terminates_and_reads_each_claim_once@sha256:ea2178098d2da4779e8fc9820e50aac8c422e0d939cb7f2203be21ccb4e38c5b, test:rust/crates/urzua-cli/tests/check_integration.rs#a_symlink_resolving_to_the_prefix_root_itself_is_deduped_not_rejected@sha256:93db1bfdbb0a32a6e160c578aab274a70c446e830a62bfb8c023a068cef5870a, test:rust/crates/urzua-cli/tests/check_integration.rs#a_symlink_cycle_that_stays_inside_the_prefix_terminates@sha256:c71e96e2560adea2ad6a1bb7baf4be17ee44862e1d3aa383c2f758bb30118a8a, test:rust/crates/urzua-cli/tests/acceptance.rs
---
# 172 — Split check run into stages with a pure derive_status and move the source readers into discovery

## What

Split `check::run` (about 700 lines in `urzua-cli/src/commands/check.rs`, which is 1,054 lines in all) into `load_context`, `run_rules`, `scope_findings` and a pure `derive_status(blocking, empty_diff, examined, executed, findings)`, and move the claim, index and published-source walkers (`read_claim_files`, `resolve_inside_repo`, `path_is_inside_repo`, `read_index_artifacts` and their symlink policy) into `discovery.rs` or a new `sources.rs`.

## Why

One function does repo and config discovery, the argv and diff scope merge, the claim, index, back-pointer and published reads, drift, every rule invocation, three kinds of post-hoc finding filtering, waivers, status derivation and the report. The filesystem walkers sit in a command file although `SPEC-22` puts I/O in discovery. `check.rs` has no unit tests; the status derivation (`empty_diff`, `blocking`, `any_rule_looked`, `examined.is_empty()`) has five interacting branches testable only end to end through `check_integration.rs`. Any change to discovery or diff scope conflicts here. A pure `derive_status` with a table test is the structured-in, structured-out shape `ADR-64` argues for, and it is where a silent "nothing examined" status would hide (`ADR-55`). The long history-narrating comments in this file (finding 13) are cleaned up as part of the split, per `SPEC-22`'s rule that a comment states the invariant, never the diff. Evidence read at the foundation review of 2026-11-04.

Ordering: after `MILE-171`, which removes the 51 closures this function holds; doing it first means splitting code that is about to be deleted.

## Done means

- `check::run` is a short composition of the four stages. — evidence: code:rust/crates/urzua-cli/src/commands/check.rs
- No function in `check.rs` exceeds a stated size bound.
- `derive_status` is pure, lives where unit tests reach it, and has a table test for each branch. — evidence: code:rust/crates/urzua-cli/src/gate.rs, test:rust/crates/urzua-cli/src/gate.rs#derives_each_branch@sha256:e08a23fce48a80f1981b5ffb29c4637ae07a5fbadef16aa0ff379667de8732b6
- The readers live in discovery with the symlink-policy tests (`BUG-69`, `BUG-75`, `BUG-85`, `BUG-117`) moved with them. — evidence: code:rust/crates/urzua-cli/src/discovery.rs, test:rust/crates/urzua-cli/tests/check_integration.rs#a_symlink_cycle_inside_a_claim_path_terminates_and_reads_each_claim_once@sha256:ea2178098d2da4779e8fc9820e50aac8c422e0d939cb7f2203be21ccb4e38c5b, test:rust/crates/urzua-cli/tests/check_integration.rs#a_symlink_resolving_to_the_prefix_root_itself_is_deduped_not_rejected@sha256:93db1bfdbb0a32a6e160c578aab274a70c446e830a62bfb8c023a068cef5870a, test:rust/crates/urzua-cli/tests/check_integration.rs#a_symlink_cycle_that_stays_inside_the_prefix_terminates@sha256:c71e96e2560adea2ad6a1bb7baf4be17ee44862e1d3aa383c2f758bb30118a8a
- Command output is unchanged. — waived: process-outcome -- a one-time byte comparison of `urzua check` output (full, path-scoped, `--base`) and every acceptance corpus before and after, recorded in the revision log; no file can evidence a before/after comparison
- The acceptance suite passes. — evidence: test:rust/crates/urzua-cli/tests/acceptance.rs

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed at Phase 0. **Why:** maintainer decision of 2026-11-04 pulled foundation work into Phase 0 (Phase 0 is parity on a solid codebase, `SPEC-1`'s Phase 0 exit); filed from the foundation review of the same date, whose evidence is cited above as read at that commit. | **substantive** |
> | 2026-11-04 | `Blocked-on` cleared. **Why:** `MILE-171` is `Done` and `narrative-field.stale` flagged the leftover. `Status` unchanged; this milestone can start. | **substantive** |
> | 2026-11-04 | `Status` Planned -> Done, `Realized-by` added. `check::run` is a composition of `open_session`, `narrow`, `load_context`, `run_rules`, `scope_findings`, `run_after_scoping` and a report build, none over 100 lines; `derive_status` is pure in `gate.rs` with a table test for each branch; `read_claim_files`, `resolve_inside_repo`, `path_is_inside_repo` and `read_index_artifacts` live in `discovery.rs`. **Why:** checked against this record's Done means: `urzua check` output over this repository (full, path-scoped, `--base HEAD~1/8/30`) and every acceptance corpus is byte-identical before and after; the acceptance suite passes. The symlink-policy cases (`BUG-69`, `BUG-75`, `BUG-85`, `BUG-117`) are end-to-end tests in `check_integration.rs` that exercise the readers through the command, so they stay where they are and still cover the moved code; there are no unit tests in the old location to move. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
