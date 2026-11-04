---
Stable-Id: 01M6D2KJ958J4M6MZB75BSP80N
Status: Open
Found-in: "A /code-review v0.3.0...main pass, round 29"
Regression-test: "planned: a check test run outside any git repository, asserting the documented behavior"
---
# 170 — SPEC-2 promises a no-git fallback that walks the directory, but the binary refuses to run outside a git repository

## What was wrong

SPEC-2 (Discovery) says that without git, `check` falls back to walking the directory and reports it in
`scope.source`. `find_repo_root` instead fails with "no .git directory found above the given path".

Repro: copy a corpus and `.urzua/config.yaml` outside any git repository and run `urzua check`:
`{"status":"not-run","error":"no .git directory found above the given path"}`, exit 2. The `scope.source`
table in SPEC-2 has no value for a directory walk, so the fallback was never given a shape either.

## What the docs say

SPEC-2's Discovery section, against `discovery.rs` (`find_repo_root`). The sweep is tracked-only by
design (`ADR-6`: never a raw directory walk), which is in tension with the fallback.

## Decision needed

- Build the fallback (and define its `scope.source` value and how it squares with ADR-6), or amend SPEC-2
  to say `check` requires a git repository and fails loud, which is defensible.
- Either way the spec and the binary must agree; today the spec promises behavior that does not exist.

## References

- `SPEC-2` -- Discovery and the `scope.source` table.
- `ADR-6` -- tracked files only.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** found by a full-release code review and reproduced with the release binary on a disposable fixture. Left `Status: Open` -- the fix needs a decision, not a quick edit. | **substantive** |
> | 2026-11-04 | Triage: still reproduces on main; `Status` stays `Open`. `urzua check` in a directory with a config and records but no `.git` returns `{"status":"not-run","error":"no .git directory found above the given path"}`, while `SPEC-2` still promises a directory-walk fallback. **Why:** build-the-fallback versus amend-the-spec is the decision the record names. | **substantive** |
