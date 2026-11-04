---
Stable-Id: 01M6K40QR3PB1ZRMH7B9G6S3R2
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-22
Blocked-on: MILE-171
---
# 172 — Split check run into stages with a pure derive_status and move the source readers into discovery

## What

Split `check::run` (about 700 lines in `urzua-cli/src/commands/check.rs`, which is 1,054 lines in all) into `load_context`, `run_rules`, `scope_findings` and a pure `derive_status(blocking, empty_diff, examined, executed, findings)`, and move the claim, index and published-source walkers (`read_claim_files`, `resolve_inside_repo`, `path_is_inside_repo`, `read_index_artifacts` and their symlink policy) into `discovery.rs` or a new `sources.rs`.

## Why

One function does repo and config discovery, the argv and diff scope merge, the claim, index, back-pointer and published reads, drift, every rule invocation, three kinds of post-hoc finding filtering, waivers, status derivation and the report. The filesystem walkers sit in a command file although `SPEC-22` puts I/O in discovery. `check.rs` has no unit tests; the status derivation (`empty_diff`, `blocking`, `any_rule_looked`, `examined.is_empty()`) has five interacting branches testable only end to end through `check_integration.rs`. Any change to discovery or diff scope conflicts here. A pure `derive_status` with a table test is the structured-in, structured-out shape `ADR-64` argues for, and it is where a silent "nothing examined" status would hide (`ADR-55`). The long history-narrating comments in this file (finding 13) are cleaned up as part of the split, per `SPEC-22`'s rule that a comment states the invariant, never the diff. Evidence read at the foundation review of 2026-11-04.

Ordering: after `MILE-171`, which removes the 51 closures this function holds; doing it first means splitting code that is about to be deleted.

## Done means

`check::run` is a short composition of the four stages and no function in `check.rs` exceeds a stated size bound; `derive_status` is pure, lives where unit tests reach it, and has a table test for each branch; the readers live in discovery with the symlink-policy tests (`BUG-69`, `BUG-75`, `BUG-85`, `BUG-117`) moved with them; command output is unchanged and the acceptance suite passes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed at Phase 0. **Why:** maintainer decision of 2026-11-04 pulled foundation work into Phase 0 (Phase 0 is parity on a solid codebase, `SPEC-1`'s Phase 0 exit); filed from the foundation review of the same date, whose evidence is cited above as read at that commit. | **substantive** |
