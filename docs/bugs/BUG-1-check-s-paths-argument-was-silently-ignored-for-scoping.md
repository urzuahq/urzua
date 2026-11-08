---
Status: Fixed
Stable-Id: 01M1Y4J5XTVNAA89BBJYT4VX95
Found-in: manual testing while building the `milestone` record type -- ran `check docs/milestones/` expecting a narrow result and got the whole 55-file corpus back, identical to `check docs/`
Realized-by: code:rust/crates/urzua-cli/src/discovery.rs#scope_to_requested_paths@sha256:4bca0662cc4a2869a3ce189587af48ae7c0da50b2dde88e4ee7e82a126e932d4, code:rust/crates/urzua-cli/src/main.rs@sha256:628a08c34658d3a1c038a2814844405aa50aeb738da95e84120d8701f282d718, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:8538aedb749292d7641c037df67b6e667d5087b0c8dca95682fef065a8429a00, test:rust/crates/urzua-cli/tests/check_integration.rs#check_scopes_to_the_requested_path_not_the_whole_corpus@sha256:f97b9312777eea2fb204110c2d0b3f37c648ecfde4f663c1c694f55570e94ab8
---
# 1 — check's paths argument was silently ignored for scoping

## What was wrong

`run_check`'s `paths` argument was used only to locate the repository root (`find_repo_root`), then
discarded. `discover_tracked_files` always scanned the entire configured corpus regardless of what
path was actually requested. `urzua check docs/adr/` and `urzua check docs/` returned byte-identical
results, examining every record type every time.

Every invocation of `urzua check docs/` throughout this project's own history examined the whole
corpus by coincidence, not by correct scoping -- `docs/` happens to be where everything lives, so
the bug had nothing to expose itself against until a genuinely narrower path was tried.

## Why nothing caught it

`check_integration.rs`'s tests all called `check docs/` -- none exercised a narrower subpath, so
nothing asserted that scoping actually excluded anything. Exactly the "no test caught it because no
real record ever needed the fallback" defect class this project's own review practice already
names, found here about the CLI's own argument handling rather than a validation rule.

## Fix

Added `scope_to_requested_paths`: canonicalizes each requested path, expresses it relative to the
repo root, and filters `discover_tracked_files`'s output to entries under any requested path. Empty
`paths` (no argument given) means no restriction, preserving the existing default behavior.

## References

- `rust/crates/urzua-cli/src/main.rs` -- `scope_to_requested_paths`, `run_check`.
- `rust/crates/urzua-cli/tests/check_integration.rs` --
  `check_scopes_to_the_requested_path_not_the_whole_corpus`, observed failing on the pre-fix code
  before being confirmed passing on the fix.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial record: defect found, fixed, and verified same-day. | **structural** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#check_scopes_to_the_requested_path_not_the_whole_corpus` (lines 433-478) a28350ca40cd -> f97b9312777e. Reason: Read the diff of each pinned span since the old pin and the claim still holds. check_integration.rs#check_scopes_to_the_requested_path_not_the_whole_corpus: the output assertions now read the JSON field with json_field instead of substring-matching the printed text, with the same expected values. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/main.rs@sha256:9fae4c3edf84, rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b833 Reason: Pinned the files as they are now after checking each against the record it evidences: config.rs still reads the YAML config and init.rs still writes it (ADR-52), discovery.rs still carries no diagnostic for a format the tool no longer reads (ADR-54), header.rs still parses the header as one closed structure and reports duplicate keys (ADR-8), main.rs still hands the paths argument to check (BUG-1), report.rs and main.rs still hold the shared Report and emit contract (BUG-19), the corpus-j renamed-record expectation files still exist (BUG-191), tests/common/mod.rs still owns the TempDir that removes fixture directories and the test-no-leak scripts still fail on a leftover (BUG-193), init.rs and migrate.rs still emit JSON reports (BUG-20), report.rs still declares ScopeSource with the SPEC-2 values and discovery.rs maps to it (BUG-25), and prepare-release.yml still fails loudly with the knope.toml release steps behind it (BUG-27). BUG-2 and BUG-23 are not in this batch because the code they cite moved out of rules.rs. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 0 file-level, 1 scoped. Evidence: rust/crates/urzua-cli/src/discovery.rs#scope_to_requested_paths@sha256:4bca0662cc4a Reason: Pinned the locators re-pointed in this review of PR 433 against the code as it is now: record_id in rules/shared.rs still accepts any non-empty numeric prefix, values.rs normalize still strips leading zeros so ADR-0034 and ADR-34 are one id (BUG-2), filename_title_consistency in rules/filename.rs still separates an unnumbered H1 from an absent one and reports the H1's own line (BUG-23, BUG-29), first_h1 in rules/filename.rs still ignores an H1 inside a fenced block (BUG-30), discovery.rs scope_to_requested_paths still narrows check to the requested paths (BUG-1), and doctor.rs DoctorReport is still the JSON doctor emits (BUG-4). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | `Realized-by` gained `code:rust/crates/urzua-cli/src/discovery.rs#scope_to_requested_paths`, checked against the code it names. **Why:** review of PR 433: the fix this record describes lives in `scope_to_requested_paths`; `main.rs`, which the record cited, now only dispatches `check` with its `paths`, so the pin on it alone did not show the fix. `main.rs` stays as the dispatch evidence. | **substantive** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` 9fae4c3edf84 -> f317c199a33d. Reason: Read the diff of each pinned file since the old pin and the claim still holds. main.rs: only the doc comment on the Init command changed, so its --help text now says --dry-run prints the config the real run would write instead of calling it byte-identical to the real run; no flag, argument or behaviour changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` f893cb99b833 -> 8538aedb7492. Reason: the tests in check_integration.rs now name real tracked files; the behaviour this record relies on is unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` f317c199a33d -> 628a08c34658. Reason: Read the diff of each re-pinned file against main. main.rs: the --help about text now carries the one-line description stated in RFC-27; no command or option changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
