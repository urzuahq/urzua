---
Status: Fixed
Stable-Id: 01M1Y4J5XTVNAA89BBJYT4VX95
Found-in: manual testing while building the `milestone` record type -- ran `check docs/milestones/` expecting a narrow result and got the whole 55-file corpus back, identical to `check docs/`
Realized-by: code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-cli/tests/check_integration.rs, test:rust/crates/urzua-cli/tests/check_integration.rs#check_scopes_to_the_requested_path_not_the_whole_corpus@sha256:f97b9312777eea2fb204110c2d0b3f37c648ecfde4f663c1c694f55570e94ab8
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
