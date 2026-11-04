---
Stable-Id: 01M6K4FD9GX9YA39H8S02ZRY1Z
Status: Done
Phase: '0'
Track: governance-process
Implements: RFC-7, ADR-70
Blocked-on: —
Realized-by: code:scripts/draft-record.sh, test:rust/crates/urzua-cli/tests/drafting_commands_integration.rs, spec:docs/specs/SPEC-19-repository-agent-guardrails.md
---
# 177 — Agent commands in this repository draft records through urzua new and verify them with urzua check

## What

Agent commands in this repository, under `.claude/commands/`, for drafting an RFC, an ADR and a milestone. Each one scaffolds with `urzua new`, fills the record's content from the conversation, stages the file so `check` governs it, runs `urzua check` and reports the findings and the number of records examined, and never sets a decision status. `AGENTS.md` gains the matching rule: when a proposal or a made decision is noticed, offer the command rather than carry on silently. The pull request template gains a prompt for whether the change embodies a decision.

## Why

A real adopter environment drafts its records through agent commands that wrap its authoring path, so an agent reaches for the command instead of freehanding a header. This repository has `urzua new` (`ADR-27`) and nothing that wraps it: `AGENTS.md` asks agents to read the relevant spec before hand-writing a record, which is an instruction and not a path. `MILE-103` records the cost: several records here were hand-written and two were rejected by `check` for headers `urzua new` would have produced. `MILE-99` records two records pushed with headers `check` rejected. `ADR-56` records that `urzua new` leaves a file untracked, so a reference to it is reported dangling while it sits on disk, and that this was misdiagnosed as a stale build twice; a command that stages before it checks removes that window for agents without changing what is governed. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

`RFC-7` section 3 puts drafting-assistance commands in scope alongside the enforcement hooks, on the condition that their output is still verified by `urzua check` and never trusted for having come from the command. `ADR-70` drafts that condition.

## Done means

An RFC, an ADR and a milestone are each drafted in this repository through its command, ending in a `urzua check` run that examined the new record and reports zero unwaived findings; a command refuses to write `Accepted`, `Done` or `Fixed`; no command restates a header field, status value or template (they read `urzua new`, `urzua rules` and `.urzua/config.yaml`); a test walks each command's steps against a fixture corpus and fails when the record is not examined; `AGENTS.md` carries the suggestion rule and `.github/PULL_REQUEST_TEMPLATE.md` the decision prompt; and `.urzua/config.yaml` and the records in this repository are what the commands run against, not only a fixture.

Sibling to `MILE-103` (preference under choice) and `MILE-151` (`urzua new` follows the declared pattern), which it does not replace.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed that environment's authoring commands have no counterpart here; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Built `/rfc`, `/adr` and `/milestone` in `.claude/commands/`, their shared `scripts/draft-record.sh`, the `AGENTS.md` suggestion rule, the pull request template's decision prompt and a test that extracts and runs each command's shell steps (`drafting_commands_integration.rs`). `Realized-by` names the script, the test and `SPEC-19`; `Status` is `Done` in the same change (the maintainer reviewing the PR is the decision). **Why:** `MILE-183` holds `Status` to delivery evidence. | **substantive** |
