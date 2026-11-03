---
Stable-Id: 01M37C018CSWK45571WHKAWKVZ
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 27"
Regression-test: "none -- no planted-violation test is practical: root is built entirely from plain strings/sequences/mappings, so serialization cannot actually fail on any reachable input. The fix is correctness-by-construction (loud failure over silent wrong success), not a behavior change for any input this function can be called with."
---
# 146 — init silently swallows a YAML serialization error into an empty config body

## What was wrong

`render_config_yaml` (`init.rs`) rendered the generated config's YAML body with
`yaml_serde::to_string(&Value::Mapping(root)).unwrap_or_default()`. If serialization ever failed, `body`
would silently become an empty string -- `urzua init` would then write a config containing only the
leading comment line and report `written: true` with no error or notice (`--dry-run` would print the
same comment-only config with `written: false`). Because `schema_version` and `record_types` are
required, `check`/`audit` would then refuse to load that config (`not-run`, "missing field
`schema_version`") -- a failure surfaced only later, by a different command, not at `init` where it
happened.

## Why nothing caught it

`root` is built entirely from plain strings, sequences, and mappings constructed from Rust `String`s
already known to be valid UTF-8 -- there is no reachable input today that makes `yaml_serde::to_string`
actually fail here, so nothing in the test suite could have observed the swallowed-error path firing.
The defect is in the failure mode chosen for a case that hasn't happened yet, not in behavior any test
exercises.

## Fix

Changed to `.unwrap()`, matching the established precedent for "this should structurally never fail"
serialization already used by `emit()` (`serde_json::to_string_pretty(..).unwrap()`) -- a real, loud
failure rather than a silent one, which the process's own panic hook (`ADR-46`) turns into structured
JSON on stdout rather than raw stderr text. No signature change, no call sites touched: `render_config_yaml`
still returns a plain `String`.

## References

- `rust/crates/urzua-cli/src/main.rs` -- `emit()`'s own `.unwrap()` on `serde_json::to_string_pretty`,
  the precedent this fix matches exactly.
- `ADR-46` -- the panic hook that turns an unwrap failure into structured JSON rather than a stderr leak.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-24 | Filed and fixed in one pass, found by a full-release code review. **Why:** `unwrap_or_default()` on a fallible serialization call is the exact silent-wrong-success shape this project has repeatedly closed elsewhere (`ADR-55`); matched to the existing `emit()` precedent for the correct failure mode instead of inventing a new one. | **substantive** |
