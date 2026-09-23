---
Stable-Id: 01M36E3SNAFFEVE01GZS54K4EY
Status: Not a bug
Found-in: "A /code-review v0.3.0...main pass, round 21"
Regression-test: "none -- see Amendment"
---
# 127 — resolve_argv_overrides silently drops a symlink to a real file since it never follows the link

## What was suspected

`resolve_argv_overrides` (`BUG-24`) checks `std::fs::symlink_metadata(&full)`, which never follows a
symlink -- for a symlink whose target is a real file, `meta.is_file()` is `false` (`is_symlink()` is
`true` instead), so the match falls to `Ok(_) => None`. The reviewer read this as the override being
silently dropped, contradicting the function's own contract: an argv path naming a file precisely is
"used as given" (`SPEC-2`), and a symlink is exactly as explicit an argv path as a direct file.

## Why the diagnosis was wrong

`resolve_argv_overrides` has exactly one caller, and its `scope` argument is never a raw argv path --
it is the output of `relative_scopes` (`discovery.rs:141-157`), which calls `.canonicalize()` on every
requested path before returning it. `Path::canonicalize()` fully resolves all symlink components (the
same as POSIX `realpath`), so by the time `resolve_argv_overrides` runs, a symlink argument has already
been replaced by its target's own canonical path. The `symlink_metadata`/`is_symlink()` branch this
bug describes is dead code reached by no input the real CLI call chain can produce.

This was caught by the project's own verify-before-trusting discipline: reverting the proposed fix
(`metadata` back to `symlink_metadata`) should have made the planted regression test fail, and it did
not -- the test passed identically either way, which is the signal a "fix" addresses nothing reachable.

## What changed

Nothing behavioral. The vacuous regression test
(`an_argv_path_naming_a_symlink_to_an_untracked_file_is_examined`) is removed rather than kept passing
for the wrong reason. `resolve_argv_overrides` keeps `symlink_metadata`, with its comment corrected to
say why the symlink/no-symlink distinction is moot at this call site instead of claiming a policy
("follows a symlink") the code does not implement.

## References

- `BUG-24` -- the function this bug's (non-)diagnosis is about.
- `BUG-85` -- the real prior instance of this shape, for `claim_paths`; not the same function.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass, `Status: Fixed`. **Why:** found by a full-release code review of `BUG-24`'s own fix, the same day it shipped. | **substantive** |
> | 2026-09-23 | Amended same day: the revert-and-observe check that verified every other round-21 fix was run against this one late, and it failed -- the "fix" made no test fail when undone. Re-diagnosed as unreachable via the real call chain (`relative_scopes` already canonicalizes every argv path first) and re-graded `Status: Not a bug`. Vacuous test removed rather than left passing for the wrong reason. **Why:** this project's own "never silently rewrite" rule -- correcting the record in place, in the open, rather than deleting it. | **substantive** |
