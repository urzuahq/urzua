---
Stable-Id: 01M6MP8QCAARFPZ420Z71Z8QPM
Status: Done
Phase: '0'
Track: governance-process
Implements: ADR-78
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/override_log.rs, code:rust/crates/urzua-core/src/rules/table.rs, code:scripts/hooks/record-edit-hook.sh, test:rust/crates/urzua-cli/tests/override_log_integration.rs, test:rust/crates/urzua-cli/tests/record_edit_hook_integration.rs
---
# 188 — A committed override log records every editor-hook override, and check reports an override not yet reviewed

## What

A committed log of editor-hook overrides, declared in `.urzua/config.yaml`, with an entry per override carrying its name, the field, the record, the reason and the date. The hook appends to it when a block is passed by a named override, and `check` reports an override that has not yet been reviewed, so it surfaces in the pull request that carries it. There is no notification or routing system: the pull request is the channel.

## Why

`ADR-78` item 1 answers `ADR-69`'s open questions on where an override is persisted and whom an unacknowledged block routes to. Today `persist_override` in the hook is an empty function, so an override is an audit trail only for as long as the transcript lives. `ADR-78` notes that no milestone yet builds this.

## Done means

- The log's location and shape are declared in `.urzua/config.yaml` (not hardcoded in the hook or the engine), and `SPEC-3` documents the declaration.
- A hook run that passes a block with `--override NAME` appends one entry with the name, the field, the record, the reason and the date, and an override with no reason is refused like an empty name; the entry is a committed file change that appears in the pull request.
- `check` reports a warning for each log entry not yet reviewed, naming the entry and the record, and no finding for a reviewed one; "reviewed" is defined in `SPEC-3` (a person has named themselves in `Reviewed-by`) and exercised by a planted-violation test that fails before the rule exists; a declared log that is missing or unparsable is reported, never read as empty.
- The rule is enabled in this repository's own `.urzua/config.yaml` with a log that exists, and `make rule-table-check` passes with the new rule in the table.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-78` Accepted 2026-11-05 decides a committed override log read by `check` and notes that no milestone exists for it. | **substantive** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Status` from `Realized-by computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
