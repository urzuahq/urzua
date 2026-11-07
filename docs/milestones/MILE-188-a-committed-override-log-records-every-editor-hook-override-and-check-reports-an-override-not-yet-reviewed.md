---
Stable-Id: 01M6MP8QCAARFPZ420Z71Z8QPM
Status: Done
Phase: '0'
Track: governance-process
Implements: ADR-78
Blocked-on: —
Realized-by: code:.urzua/config.yaml#override_log@sha256:5a586a85c059739cf449b56bb7b6814a3ae6f20d03c8e834bde69571e3d0aaf1, test:rust/crates/urzua-cli/tests/override_log_integration.rs#enabling_the_rule_without_declaring_a_log_is_a_could_not_run@sha256:1247c1bd40351630e86eb3d7a9e09e09f53d61fb87ed573a6f1fbc21500ca9ba, spec:docs/specs/SPEC-3-configuration.md, code:rust/crates/urzua-cli/src/harness/run.rs#persist_override@sha256:25bb90a542f95d7152cd61af50a3ea0bfe9060da4c0d0b5f6b9b11325f68cbf2, test:rust/crates/urzua-cli/tests/record_edit_hook_integration.rs#an_override_is_recorded_in_the_declared_log_and_check_then_reports_it_unreviewed@sha256:97c7d56f4f9a7031fc2f20ab1c69c116f8d781525cd802bc44cead236c2b04bd, test:rust/crates/urzua-cli/tests/record_edit_hook_integration.rs#with_a_log_declared_an_override_needs_a_reason_and_nothing_is_written_without_one@sha256:f36c60291383a6f90fc16c16ec1634b4be809a4f2fb4de4e0dcee8e69984c5ab, code:rust/crates/urzua-core/src/rules/override_log.rs, test:rust/crates/urzua-cli/tests/override_log_integration.rs#an_unreviewed_override_is_a_warning_on_the_log_and_a_reviewed_one_is_not@sha256:458c9fc8ab7a77162f4dff4b63bab8df3cc8f921903e6a11d25a8df02c5d5d53, test:rust/crates/urzua-cli/tests/override_log_integration.rs#a_placeholder_or_pending_reviewer_does_not_clear_the_finding@sha256:8c4071d4a00dd26f6a749d19f066ffd2068b9de236dd357e4709915ee0ca1eca, test:rust/crates/urzua-cli/tests/override_log_integration.rs#a_declared_log_that_does_not_exist_is_reported_never_read_as_empty@sha256:6606e1ed17c0e29807a03215a5a06b4e7ea3817b55e7cdeb7799561b07d00334, test:rust/crates/urzua-cli/tests/override_log_integration.rs#a_row_that_is_not_an_entry_is_reported@sha256:8cf567937a9707efb8db8c0d0da90147bca57873a6e69d8d035afc0cab35cb7d, code:.urzua/override-log.md, code:rust/crates/urzua-core/src/rules/table.rs, spec:docs/specs/SPEC-2-urzua-check.md, code:scripts/generate-rule-table.py
---
# 188 — A committed override log records every editor-hook override, and check reports an override not yet reviewed

## What

A committed log of editor-hook overrides, declared in `.urzua/config.yaml`, with an entry per override carrying its name, the field, the record, the reason and the date. The hook appends to it when a block is passed by a named override, and `check` reports an override that has not yet been reviewed, so it surfaces in the pull request that carries it. There is no notification or routing system: the pull request is the channel.

## Why

`ADR-78` item 1 answers `ADR-69`'s open questions on where an override is persisted and whom an unacknowledged block routes to. Today `persist_override` in the hook is an empty function, so an override is an audit trail only for as long as the transcript lives. `ADR-78` notes that no milestone yet builds this.

## Done means

- The override log's location and shape are declared in `.urzua/config.yaml`, not hardcoded in the hook or the engine. — evidence: code:.urzua/config.yaml#override_log@sha256:5a586a85c059739cf449b56bb7b6814a3ae6f20d03c8e834bde69571e3d0aaf1, test:rust/crates/urzua-cli/tests/override_log_integration.rs#enabling_the_rule_without_declaring_a_log_is_a_could_not_run@sha256:1247c1bd40351630e86eb3d7a9e09e09f53d61fb87ed573a6f1fbc21500ca9ba
- `SPEC-3` documents the override log declaration. — evidence: spec:docs/specs/SPEC-3-configuration.md
- A hook run that passes a block with `--override NAME` appends one entry with the name, the field, the record, the reason and the date, as a committed file change. — evidence: code:rust/crates/urzua-cli/src/harness/run.rs#persist_override@sha256:25bb90a542f95d7152cd61af50a3ea0bfe9060da4c0d0b5f6b9b11325f68cbf2, test:rust/crates/urzua-cli/tests/record_edit_hook_integration.rs#an_override_is_recorded_in_the_declared_log_and_check_then_reports_it_unreviewed@sha256:97c7d56f4f9a7031fc2f20ab1c69c116f8d781525cd802bc44cead236c2b04bd
- An override with no reason is refused like an empty name. — evidence: test:rust/crates/urzua-cli/tests/record_edit_hook_integration.rs#with_a_log_declared_an_override_needs_a_reason_and_nothing_is_written_without_one@sha256:f36c60291383a6f90fc16c16ec1634b4be809a4f2fb4de4e0dcee8e69984c5ab
- `check` reports a warning for each log entry not yet reviewed, naming the entry and the record, and no finding for a reviewed one, exercised by a planted-violation test. — evidence: code:rust/crates/urzua-core/src/rules/override_log.rs, test:rust/crates/urzua-cli/tests/override_log_integration.rs#an_unreviewed_override_is_a_warning_on_the_log_and_a_reviewed_one_is_not@sha256:458c9fc8ab7a77162f4dff4b63bab8df3cc8f921903e6a11d25a8df02c5d5d53
- "Reviewed" is defined in `SPEC-3` (a person has named themselves in `Reviewed-by`), and a blank or placeholder reviewer does not count. — evidence: spec:docs/specs/SPEC-3-configuration.md, test:rust/crates/urzua-cli/tests/override_log_integration.rs#a_placeholder_or_pending_reviewer_does_not_clear_the_finding@sha256:8c4071d4a00dd26f6a749d19f066ffd2068b9de236dd357e4709915ee0ca1eca
- A declared log that is missing or unparsable is reported, never read as empty. — evidence: test:rust/crates/urzua-cli/tests/override_log_integration.rs#a_declared_log_that_does_not_exist_is_reported_never_read_as_empty@sha256:6606e1ed17c0e29807a03215a5a06b4e7ea3817b55e7cdeb7799561b07d00334, test:rust/crates/urzua-cli/tests/override_log_integration.rs#a_row_that_is_not_an_entry_is_reported@sha256:8cf567937a9707efb8db8c0d0da90147bca57873a6e69d8d035afc0cab35cb7d
- The rule is enabled in this repository's own `.urzua/config.yaml` with a log that exists. — evidence: code:.urzua/config.yaml#override_log@sha256:5a586a85c059739cf449b56bb7b6814a3ae6f20d03c8e834bde69571e3d0aaf1, code:.urzua/override-log.md
- `make rule-table-check` passes with the new rule in the table. — evidence: code:rust/crates/urzua-core/src/rules/table.rs, spec:docs/specs/SPEC-2-urzua-check.md, code:scripts/generate-rule-table.py

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-78` Accepted 2026-11-05 decides a committed override log read by `check` and notes that no milestone exists for it. | **substantive** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Status` from `Realized-by computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
> | 2026-11-05 | Built in #309: top-level `override_log` and the opt-in `override.unreviewed` rule (`SPEC-2`, `SPEC-3`), the hook recording a named override and requiring a reason (`SPEC-8`), and both enabled in this repository's configuration with an empty log. `Realized-by` added and `Status` set to `Done`. **Why:** every item of Done means is met and exercised in this repository's configuration. | **substantive** |
> | 2026-11-05 | Fixed after review of #309: `Reviewed-by` counts as a review only when it names a person (a blank cell, a placeholder such as `TBD` or `-`, and a value the record types declare as pending are not a review, using the field-state vocabulary of `field.quality` and `field.pending`), and a declared log that does not exist is reported with the variant `override-log-missing` instead of read as empty, which is what the third item of Done means asks for. **Why:** the reviewer showed the warning cleared by `TBD`, `-` and `Pending`, and found the missing-log item of Done means unmet while the milestone read `Done`; both are now met and exercised by tests. | **substantive** |
> | 2026-11-06 | `Realized-by` now names `rust/crates/urzua-cli/src/harness/run.rs`, the engine core that appends the override-log rows, in place of the deleted script. **Why:** `MILE-204` deleted the script this record cited; the engine core and the installed translator are the code that now carries the behavior. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
