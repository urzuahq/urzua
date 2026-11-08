---
Status: Fixed
Stable-Id: 01M1Z5YTQ1FVF20SGM9JM6C0FF
Found-in: writing SPEC-15 (the doctor spec) and finding the actual implementation printed `[OK]`/`[WARN]`/`[ERROR]` text lines, directly contradicting ADR-23's "stdout is always JSON, no format flag" contract every other command follows
Realized-by: code:rust/crates/urzua-cli/src/commands/doctor.rs#DoctorReport@sha256:7b0d6b1c81464ef03d8bdfdc3a7c2d2f9b6a5339ff411793a1e4f3927ef2b73d, code:rust/crates/urzua-cli/src/main.rs@sha256:628a08c34658d3a1c038a2814844405aa50aeb738da95e84120d8701f282d718, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:5ab61b95fd5864a41566dee0d902fac000dca3552f90316331ecf90617c3f9b0, test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_emits_json_not_plain_text_lines@sha256:f1206a0ad9ab58de9de017375f35f62be7cca473c3fca30d6ed17d4e15d9f9bf
---
# 4 — urzua doctor outputs plain text instead of JSON

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
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#doctor_emits_json_not_plain_text_lines` (lines 394-431) b8b8680e7dcc -> f1206a0ad9ab. Reason: Read the diff of each pinned span since the old pin and the claim still holds. check_integration.rs#doctor_emits_json_not_plain_text_lines: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/main.rs@sha256:9fae4c3edf84, rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b833 Reason: Pinned the files as they are now after checking each against the record it evidences: publish-release.yml still builds and uploads the archives itself (BUG-28), new_record.rs still renders only the configured YAML header and new_integration.rs still holds the test that a blockquote template does not override it (BUG-3), waiver.rs is_active still parses Expires as a date (BUG-31), prepare-release.yml still hands knope's nothing-to-release exit to release-guard.sh instead of failing (BUG-33), ci.yml's changeset job still reads the label live (BUG-34), knope.toml still releases from changesets only (BUG-35), main.rs and check_integration.rs still emit doctor as JSON (BUG-4), release-guard.test.sh still plants an unmatched-package fragment and asserts a nonzero exit (BUG-48), field_state.rs still carries the retired placeholder tokens (BUG-5), and generate-rule-table.py and the Makefile rule-table-check target still compare SPEC-2's rule table with the shipped rule set (BUG-52). BUG-29 and BUG-30 are not in this batch because the filename.title-consistency code they cite moved out of rules.rs. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 0 file-level, 1 scoped. Evidence: rust/crates/urzua-cli/src/commands/doctor.rs#DoctorReport@sha256:7b0d6b1c8146 Reason: Pinned the locators re-pointed in this review of PR 433 against the code as it is now: record_id in rules/shared.rs still accepts any non-empty numeric prefix, values.rs normalize still strips leading zeros so ADR-0034 and ADR-34 are one id (BUG-2), filename_title_consistency in rules/filename.rs still separates an unnumbered H1 from an absent one and reports the H1's own line (BUG-23, BUG-29), first_h1 in rules/filename.rs still ignores an H1 inside a fenced block (BUG-30), discovery.rs scope_to_requested_paths still narrows check to the requested paths (BUG-1), and doctor.rs DoctorReport is still the JSON doctor emits (BUG-4). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` 9fae4c3edf84 -> f317c199a33d. Reason: Read the diff of each pinned file since the old pin and the claim still holds. main.rs: only the doc comment on the Init command changed, so its --help text now says --dry-run prints the config the real run would write instead of calling it byte-identical to the real run; no flag, argument or behaviour changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` f893cb99b833 -> 8538aedb7492. Reason: the tests in check_integration.rs now name real tracked files; the behaviour this record relies on is unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` 8538aedb7492 -> 5ab61b95fd58. Reason: check_integration.rs: the only change is in pre_commit_hook_stops_a_planted_bad_header_with_the_real_binary, which now symlinks the urzua binary into the fixture instead of copying it (a copy raced a forking test thread and failed with ETXTBSY under load); the test's setup, planted defect and assertions are otherwise unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` f317c199a33d -> 628a08c34658. Reason: Read the diff of each re-pinned file against main. main.rs: the --help about text now carries the one-line description stated in RFC-27; no command or option changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | `Realized-by` gained `code:rust/crates/urzua-cli/src/commands/doctor.rs#DoctorReport`, checked against the code it names. **Why:** review of PR 433: `run_doctor` moved out of `main.rs` into `commands/doctor.rs`, where `DoctorReport` is the JSON `doctor` emits; `main.rs` keeps the shared `emit`, so it stays as evidence beside it. | **substantive** |
