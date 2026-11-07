---
Status: Done
Stable-Id: 01M1YNNG9RQEZYDDHBQDN5AJDW
Phase: '0'
Track: governance-process
Implements: ADR-23
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/main.rs#emit@sha256:29d071bbb8232c8cd5678d8c5d104830ec9b083471d1575ab768050e59704a71, test:rust/crates/urzua-cli/tests/could_not_run_integration.rs#every_command_that_reads_a_repository_exits_2_outside_one_observed_failing@sha256:8b3362d317c4fb9b5063b83f918fa80b2d8ee559c7c1e7570e697704f5efcbf5, test:rust/crates/urzua-cli/tests/check_integration.rs#a_bad_flag_emits_json_not_a_panic@sha256:eb8bfb4d74748038835831064ab072943e8c32648f99c1202940cb169993747a
---
# 69 — Stdout is always JSON; remove the --format flag

## What

Removed `--format`: `check`, `fix`, `explain`, `graph`, and `migrate schema --report` now emit exactly one JSON object on stdout, unconditionally, with no human-readable rendering anywhere.

## Why

Two output shapes mean two things that can silently drift apart, and an agent piping stdout, a script, and a person reading a terminal should all see exactly the same thing -- one shape is a stronger guarantee than a flag that happens to default correctly.

## Done means

- `check`, `fix`, `explain`, `graph` and `migrate schema --report` each emit exactly one JSON object on stdout, unconditionally. — evidence: code:rust/crates/urzua-cli/src/main.rs#emit@sha256:29d071bbb8232c8cd5678d8c5d104830ec9b083471d1575ab768050e59704a71, test:rust/crates/urzua-cli/tests/could_not_run_integration.rs#every_command_that_reads_a_repository_exits_2_outside_one_observed_failing@sha256:8b3362d317c4fb9b5063b83f918fa80b2d8ee559c7c1e7570e697704f5efcbf5
- There is no `--format` flag and no human-readable rendering. — evidence: code:rust/crates/urzua-cli/src/main.rs#emit@sha256:29d071bbb8232c8cd5678d8c5d104830ec9b083471d1575ab768050e59704a71
- A test asserts the flag is rejected and stdout parses as one JSON value. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#a_bad_flag_emits_json_not_a_panic@sha256:eb8bfb4d74748038835831064ab072943e8c32648f99c1202940cb169993747a

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-06 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
