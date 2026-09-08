---
Status: Fixed
Stable-Id: 01M1Y4J5XTVNAA89BBJYT4VX95
Found-in: manual testing while building the `milestone` record type -- ran `check docs/milestones/` expecting a narrow result and got the whole 55-file corpus back, identical to `check docs/`
Regression-test: check_scopes_to_the_requested_path_not_the_whole_corpus (rust/crates/urzua-cli/tests/check_integration.rs)
Realized-by: code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
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
