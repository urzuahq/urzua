---
Stable-Id: 01M6R3NK4WA4F3ZX0G8W2S81CK
Status: Done
Phase: '1'
Track: schema-governance
Implements: ADR-23
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/main.rs#emit@sha256:29d071bbb8232c8cd5678d8c5d104830ec9b083471d1575ab768050e59704a71, test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_emits_json_not_plain_text_lines@sha256:f1206a0ad9ab58de9de017375f35f62be7cca473c3fca30d6ed17d4e15d9f9bf, test:rust/crates/urzua-cli/tests/check_integration.rs#fix_could_not_run_emits_json@sha256:3c19eb5997529c40ff9d2f64f5f34bdf0f2db19dcc43ff5b97f670bd2c566b83, test:rust/crates/urzua-cli/tests/check_integration.rs#explain_could_not_run_emits_json@sha256:1d06a1d80aa3059a75170cf25f732a714eafef07169d382b1943031675546a62, test:rust/crates/urzua-cli/tests/check_integration.rs#graph_could_not_run_emits_json@sha256:7557a40f07ae8c953456434234ba8fc3a88b71e61c161b1e4c7a19c126885aa3
---
# 82 — One shared emit_json helper instead of seven inline copies

## What

Collapse the seven call sites in `urzua-cli/src/main.rs` that each independently do
`println!("{}", serde_json::to_string_pretty(&X).unwrap())` into one shared function (e.g.
`emit_json<T: Serialize>(value: &T) -> `). Today only `check`'s version has a name
(`print_report`); the other six are copy-pasted inline, each printing a different ad hoc struct.

## Why

Found live while fixing BUG-4 (`doctor` printing plain text instead of JSON): the fix needed a JSON
emission path for `doctor`, and checking what already existed found the exact same one-line pattern
duplicated seven times across the command surface, with no single reusable function most commands
actually call. Fixing `doctor` in isolation would have made it an eighth copy rather than closing
the underlying gap. Scoped out of BUG-4 itself (kept that fix narrow, per SPEC-9's own retrospective
scope) and tracked here instead, since it's a real, separate piece of generic tooling work, not part
of what was actually broken in `doctor`.

## Done means

- Every command prints its JSON through the one shared `emit` function in `main.rs`. — evidence: code:rust/crates/urzua-cli/src/main.rs#emit@sha256:29d071bbb8232c8cd5678d8c5d104830ec9b083471d1575ab768050e59704a71
- No inline `println!` of `serde_json::to_string_pretty` remains in any command. — evidence: code:rust/crates/urzua-cli/src/main.rs#emit@sha256:29d071bbb8232c8cd5678d8c5d104830ec9b083471d1575ab768050e59704a71
- `doctor` prints JSON. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_emits_json_not_plain_text_lines@sha256:f1206a0ad9ab58de9de017375f35f62be7cca473c3fca30d6ed17d4e15d9f9bf
- `check_integration.rs` asserts each command's stdout parses as one JSON value. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#fix_could_not_run_emits_json@sha256:3c19eb5997529c40ff9d2f64f5f34bdf0f2db19dcc43ff5b97f670bd2c566b83, test:rust/crates/urzua-cli/tests/check_integration.rs#explain_could_not_run_emits_json@sha256:1d06a1d80aa3059a75170cf25f732a714eafef07169d382b1943031675546a62, test:rust/crates/urzua-cli/tests/check_integration.rs#graph_could_not_run_emits_json@sha256:7557a40f07ae8c953456434234ba8fc3a88b71e61c161b1e4c7a19c126885aa3

> **Revision log**
>
> | Date | Change | Class |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged on `main`: every command prints through the one `emit` function in `main.rs`; no inline `to_string_pretty` copy remains in a command. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> |---|---|---|
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
