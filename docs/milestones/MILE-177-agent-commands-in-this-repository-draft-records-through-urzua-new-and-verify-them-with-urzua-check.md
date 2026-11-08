---
Stable-Id: 01M6K4FD9GX9YA39H8S02ZRY1Z
Status: Done
Phase: '0'
Track: governance-process
Implements: RFC-7, ADR-70
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/drafting_commands_integration.rs#each_command_drafts_a_record_that_check_examined_and_passes@sha256:06558493d7951a03308c9c8ea74e1c4f302ed7d2ecf6739c5e04a82aadbaeec0, code:.claude/hooks/urzua-record.sh, test:rust/crates/urzua-cli/tests/drafting_commands_integration.rs#a_decision_status_written_into_the_draft_is_refused_and_nothing_is_staged@sha256:7ea4c611c23ad004c1e85fe0dd69a5cc5bec47684c72c72e56304761fac73675, test:rust/crates/urzua-cli/tests/drafting_commands_integration.rs#commands_carry_no_schema_knowledge_of_their_own@sha256:d8856d08d39bb1d3e24c7371841d57828087a906f2d80b44d89423f26cfa4fa5, test:rust/crates/urzua-cli/tests/drafting_commands_integration.rs#with_include_untracked_a_sweep_examines_a_scaffold_before_it_is_staged@sha256:72a6a58d19ad423cb4c08a3a555399584891f0d7c03b5666d7f12ede30b51e17, code:AGENTS.md, code:.github/PULL_REQUEST_TEMPLATE.md, code:.urzua/config.yaml
---
# 177 — Agent commands in this repository draft records through urzua new and verify them with urzua check

## What

Agent commands in this repository, under `.claude/commands/`, for drafting an RFC, an ADR and a milestone. Each one scaffolds with `urzua new`, fills the record's content from the conversation, stages the file so `check` governs it, runs `urzua check` and reports the findings and the number of records examined, and never sets a decision status. `AGENTS.md` gains the matching rule: when a proposal or a made decision is noticed, offer the command rather than carry on silently. The pull request template gains a prompt for whether the change embodies a decision.

## Why

A real adopter environment drafts its records through agent commands that wrap its authoring path, so an agent reaches for the command instead of freehanding a header. This repository has `urzua new` (`ADR-27`) and nothing that wraps it: `AGENTS.md` asks agents to read the relevant spec before hand-writing a record, which is an instruction and not a path. `MILE-103` records the cost: several records here were hand-written and two were rejected by `check` for headers `urzua new` would have produced. `MILE-99` records two records pushed with headers `check` rejected. `ADR-56` records that `urzua new` leaves a file untracked, so a reference to it is reported dangling while it sits on disk, and that this was misdiagnosed as a stale build twice; a command that stages before it checks removes that window for agents without changing what is governed. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

`RFC-7` section 3 puts drafting-assistance commands in scope alongside the enforcement hooks, on the condition that their output is still verified by `urzua check` and never trusted for having come from the command. `ADR-70` drafts that condition.

## Done means

- An RFC, an ADR and a milestone are each drafted in this repository through its command, ending in a `urzua check` run that examined the new record and reports zero unwaived findings. — evidence: test:rust/crates/urzua-cli/tests/drafting_commands_integration.rs#each_command_drafts_a_record_that_check_examined_and_passes@sha256:06558493d7951a03308c9c8ea74e1c4f302ed7d2ecf6739c5e04a82aadbaeec0, code:.claude/hooks/urzua-record.sh
- A command refuses to write `Accepted`, `Done` or `Fixed`. — evidence: test:rust/crates/urzua-cli/tests/drafting_commands_integration.rs#a_decision_status_written_into_the_draft_is_refused_and_nothing_is_staged@sha256:7ea4c611c23ad004c1e85fe0dd69a5cc5bec47684c72c72e56304761fac73675
- No command restates a header field, status value or template (they read `urzua new`, `urzua rules` and `.urzua/config.yaml`). — evidence: test:rust/crates/urzua-cli/tests/drafting_commands_integration.rs#commands_carry_no_schema_knowledge_of_their_own@sha256:d8856d08d39bb1d3e24c7371841d57828087a906f2d80b44d89423f26cfa4fa5
- A test walks each command's steps against a fixture corpus and fails when the record is not examined. — evidence: test:rust/crates/urzua-cli/tests/drafting_commands_integration.rs#each_command_drafts_a_record_that_check_examined_and_passes@sha256:06558493d7951a03308c9c8ea74e1c4f302ed7d2ecf6739c5e04a82aadbaeec0, test:rust/crates/urzua-cli/tests/drafting_commands_integration.rs#with_include_untracked_a_sweep_examines_a_scaffold_before_it_is_staged@sha256:72a6a58d19ad423cb4c08a3a555399584891f0d7c03b5666d7f12ede30b51e17
- `AGENTS.md` carries the suggestion rule. — evidence: code:AGENTS.md
- `.github/PULL_REQUEST_TEMPLATE.md` carries the decision prompt. — evidence: code:.github/PULL_REQUEST_TEMPLATE.md
- `.urzua/config.yaml` and the records in this repository are what the commands run against, not only a fixture. — evidence: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/drafting_commands_integration.rs#each_command_drafts_a_record_that_check_examined_and_passes@sha256:06558493d7951a03308c9c8ea74e1c4f302ed7d2ecf6739c5e04a82aadbaeec0

Sibling to `MILE-103` (preference under choice) and `MILE-151` (`urzua new` follows the declared pattern), which it does not replace.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed that environment's authoring commands have no counterpart here; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Built `/rfc`, `/adr` and `/milestone` in `.claude/commands/`, their shared `scripts/draft-record.sh`, the `AGENTS.md` suggestion rule, the pull request template's decision prompt and a test that extracts and runs each command's shell steps (`drafting_commands_integration.rs`). `Realized-by` names the script, the test and `SPEC-19`; `Status` is `Done` in the same change (the maintainer reviewing the PR is the decision). **Why:** `MILE-183` holds `Status` to delivery evidence. | **substantive** |
> | 2026-11-06 | `Realized-by` now names `.claude/hooks/urzua-record.sh`, the bundle's helper that replaced `scripts/draft-record.sh`. **Why:** `MILE-204` deleted the script this record cited; the engine core and the installed translator are the code that now carries the behavior. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/drafting_commands_integration.rs#each_command_drafts_a_record_that_check_examined_and_passes` (lines 185-230) bb07ec43d504 -> 06558493d795. Reason: The test's clean-run assertion counts only unwaived findings now, because this repository's configuration carries one waived finding that every scope keeps (WAIVER-0052). It still requires one examined record and no unwaived finding. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
