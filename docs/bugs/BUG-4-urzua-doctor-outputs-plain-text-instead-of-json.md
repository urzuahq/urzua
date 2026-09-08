# 4 — urzua doctor outputs plain text instead of JSON

> Status: Fixed
> Stable-Id: 01M1Z5YTQ1FVF20SGM9JM6C0FF
> Found-in: writing SPEC-15 (the doctor spec) and finding the actual implementation printed `[OK]`/`[WARN]`/`[ERROR]` text lines, directly contradicting ADR-23's "stdout is always JSON, no format flag" contract every other command follows
> Regression-test: doctor_emits_json_not_plain_text_lines (rust/crates/urzua-cli/tests/check_integration.rs)
> Realized-by: code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-cli/tests/check_integration.rs

## What was wrong

`urzua doctor` printed human-readable `[OK]`/`[WARN]`/`[ERROR]` lines to stdout and nothing else —
no JSON at all. Every other implemented command (`check`, `audit`, `new`, `fix`, `explain`, `graph`,
`migrate ids`, `migrate schema --report`) emits its report as JSON on stdout, unconditionally, per
ADR-23. `doctor` was the one command that never got the memo, even though ADR-23's own text already
named it (alongside `init`) as a known, tracked exception -- "predate the JSON report" -- rather
than something anyone had actually gone back to fix.

## Why nothing caught it

`doctor`'s only existing test (`doctor_reports_missing_config_as_exit_2`) asserted the exit code
only, never inspected stdout's shape. A test asserting an exit code and a test asserting an output
contract are different claims, and only the first existed -- so a command entirely off the JSON
contract could still pass its own test suite indefinitely.

## References

- Fixed by converting `run_doctor` to build a `DoctorReport { status, checks: Vec<DoctorCheck> }`
  and emit it via `serde_json::to_string_pretty`, same shape convention as every other command's
  report. Exit-code semantics are unchanged (2 = no config, 1 = config error or any `error`-level
  check, 0 otherwise) -- only the output format changed.
- ADR-23 — the stdout-JSON-always contract this brings `doctor` into compliance with.
- SPEC-15 — `doctor`'s full current spec, written the same day this bug was found.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Fixed and shipped: `doctor` now emits `DoctorReport` JSON instead of plain-text lines; exit-code semantics unchanged. | **substantive** |

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
