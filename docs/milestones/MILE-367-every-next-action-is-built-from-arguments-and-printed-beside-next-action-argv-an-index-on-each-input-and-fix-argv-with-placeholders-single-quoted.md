---
Stable-Id: 01M85PWATEYZV2X9G3W58M1Q0N
Status: Planned
Phase: '1'
Track: integrations
Depends-on: MILE-359
Blocked-on: —
---
# 367 — Every next_action is built from arguments and printed beside next_action_argv, an index on each input and fix_argv, with placeholders single-quoted

## What

Every `next_action` is built from a program and its arguments, printed as a string joined and shell-quoted for POSIX sh, and as `next_action_argv` (null exactly when `next_action` is null); each entry of `next_action_inputs` carries an `index` of the element it replaces, that element being exactly its placeholder; every finding with a `fix` carries `fix_argv`; and a placeholder in the string is single-quoted in every report (`'<record>'`).

## Why

Contract 1.1 (the published agent CLI contract's RFC-4) is taken whole: urzua prints `contract_version` `"1.1"` only when every piece below is delivered (`MILE-370`). `NextAction` (`urzua-core/src/report.rs`) holds a `command: String`, and about 36 construction sites in `rust/crates/*/src` (`runs_as_printed`, `with_next_action` and the placeholder variants) build strings by hand with three quoting styles (single, double and bare). The contract says a reader of the array never parses quoting and the two forms must agree under POSIX splitting. A day's work was the estimate the contract's text records for building every action from arguments, and it is the largest piece, so it is its own milestone. `hooks.rs` reads `next_action` with `as_str()`, so the string stays.

## Done means

- `NextAction` holds the argument vector and derives the string from it, and no construction site passes a pre-joined command string, shown by a grep and by the type. — evidence: code:rust/crates/urzua-core/src/report.rs, test:rust/crates/urzua-cli/tests/runnable_actions_integration.rs#a_placeholder_is_a_single_quoted_word_at_its_index
- A test runs each reporting command with a planted case that yields a `next_action` and checks, for every report, `next_action_argv` is present, not empty when not null, and equals the string split as a POSIX shell would (the contract's runner does the same: `MILE-369`). — evidence: test:rust/crates/urzua-cli/tests/runnable_actions_integration.rs#every_suggested_action_and_every_next_action_runs_as_printed, test:rust/crates/urzua-cli/tests/golden_output_integration.rs#every_command_prints_the_output_it_always_printed, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#a_hand_added_first_pin_is_reported_and_fix_pin_writes_the_row_that_clears_it
- Every `next_action_inputs` entry has an `index`, and the element at it equals the entry's `placeholder`; the string carries it single-quoted. The test fails against the current bare and double-quoted forms. — evidence: test:rust/crates/urzua-cli/tests/runnable_actions_integration.rs#a_placeholder_is_a_single_quoted_word_at_its_index, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#fix_pin_lists_a_log_it_would_create_and_refuses_a_record_with_no_body
- Every finding that carries a `fix` also carries `fix_argv`, shown by a test over the rules that print one (`doctor`, `hooks`, `fix`). — evidence: test:rust/crates/urzua-cli/tests/runnable_actions_integration.rs#a_hook_fix_is_one_command_whose_words_are_printed_beside_it, test:rust/crates/urzua-cli/tests/hooks_reach_integration.rs#the_chmod_fix_quotes_the_path
- The report and finding schemas of the release candidate `MILE-369` vendors accept the new keys and the reports validate against them. — evidence: test:rust/crates/urzua-cli/tests/agent_cli_contract_integration.rs#a_report_with_placeholders_validates_against_the_base_schema
- `SPEC-2` and the reference pages that show a `next_action` show the quoted form, each spec with a revision row. — evidence: spec:docs/specs/SPEC-2-urzua-check.md, spec:docs/specs/SPEC-15-urzua-doctor.md, spec:docs/reference/commands.md, spec:docs/reference/check.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Filed as planned work: nothing built, no evidence yet. It is one piece of conforming to the published agent CLI contract 1.1, whose RFC-4 the maintainer accepted: "I accept RFC, and yes allow option can be outside rule" (the contract's RFC-4, revision row of 2026-11-24). | **substantive** |
