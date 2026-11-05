---
Stable-Id: 01M6D2KJ958J4M6MZB75BSP80N
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 29"
Realized-by: test:rust/crates/urzua-cli/tests/could_not_run_integration.rs#every_command_that_reads_a_repository_exits_2_outside_one_observed_failing@sha256:8b3362d317c4fb9b5063b83f918fa80b2d8ee559c7c1e7570e697704f5efcbf5, test:rust/crates/urzua-cli/tests/could_not_run_integration.rs#rules_reads_no_repository_and_runs_outside_one@sha256:7991558a3d30e97a48282b2a11f5b0a4f7978a59b1d226d8cb2e9c5b8b0dd5ec
---
# 170 — SPEC-2 promises a no-git fallback that walks the directory, but the binary refuses to run outside a git repository

## What was wrong

SPEC-2 (Discovery) says that without git, `check` falls back to walking the directory and reports it in
`scope.source`. `find_repo_root` instead fails with "no .git directory found above the given path".

Repro: copy a corpus and `.urzua/config.yaml` outside any git repository and run `urzua check`:
`{"status":"not-run","error":"no .git directory found above the given path"}`, exit 2. The `scope.source`
table in SPEC-2 has no value for a directory walk, so the fallback was never given a shape either.

## Decision (2026-11-05)

The maintainer decided to amend `SPEC-2` and remove the fallback. Outside a git repository every command exits 2 with a clear "needs a git repository" error. Checked against the binary on a config and corpus outside any repository: every command that reads a repository (`check`, `audit`, `explain`, `graph`, `export`, `fix`, `migrate`, `index`, `refs refresh`, `init`, `doctor`, `new`, and `check` given a path) already exited 2; only the message changed, from `no .git directory found above the given path` to `urzua needs a git repository: no .git directory found above the given path`. `rules` reads no repository and still runs; `import` is not implemented and exits 2 with its own message.

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
> | 2026-11-05 | Fixed. **Why:** maintainer decision 2026-11-05: amend `SPEC-2` to remove the no-git fallback; every command that reads a repository exits 2 with a "needs a git repository" error, now tested for each. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
