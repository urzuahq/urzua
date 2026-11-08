---
Stable-Id: 01M6W6WSD9FV1NNZT9TBYMKJDM
Status: Accepted
Accepted-on: 2026-11-08
Date: 2026-11-07
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
---
# 98 — Test CI is sharded across parallel jobs and one gate job aggregates them

## Context

The `rust` job of `checks.yml` ran fmt, clippy, the supply-chain gates, the release build, the records
gate and every test in one job on one runner, in sequence. It took 14 to 15 minutes against a
15-minute timeout and was being cancelled; `PR #428` raised the timeout to 30 minutes as a stopgap
(this ADR's change supersedes that line, since the job it limits no longer exists).

Measured on CI (run 42420633044) and locally:

- The test files are tiny except for a handful: `acceptance` about 397 s, `drafting_commands_integration`
  149 s, and four more binaries at 38 to 45 s. Everything else is under 3 s per binary.
- `cargo test` runs the 86 test binaries one after another, so the many fast ones wait behind each
  other and the heavy ones never overlap. Run through `cargo nextest` (one process per test, all
  binaries scheduled together) the 2,088 non-acceptance tests took 3 m 45 s locally on four cores. Measured on CI after the change (run 42429110304, cold cache): 4 m 24 s end to end, against 14 to 15 minutes before; the nextest shard union is 2,111 tests and the acceptance shards run all 151 cases, checked against an unsharded run by the independent review on `#430`.
- The `acceptance` runner is a custom harness (`harness = false`): one process, one suite-wide
  coverage gate (SPEC-4). nextest cannot list or partition it.
- Fmt, clippy, the release build and the tests do not depend on each other, only on the checkout.

`make ci` and the `ci` aggregate must stay as they are for the ruleset (`ADR-95`, `docs/reference/release-app-setup.md`
requires the single check `ci`).

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Raise the timeout only (`PR #428`) | One line; no new moving parts | The job keeps getting slower with the suite; every push waits 14+ minutes; no feedback on what is slow |
| B. Keep one job, switch `cargo test` to `cargo nextest run` | Small; most of the test-time win (no per-binary serial wait) | fmt, clippy, build and tests stay in series; the 397 s acceptance runner is still one process; no parallel runners |
| C. Split into parallel jobs and shard the tests with `cargo nextest --partition count:i/n`, shard the acceptance runner by its own switch, merge its coverage proof, keep `ci` as the only required name | Wall-clock is set by the slowest job; a shard count is one number; failure names the shard; the cache is per job type | More jobs and runner-minutes; each test shard compiles the tests (warm dependency cache keeps this short); the acceptance runner needs a small, tested shard/merge protocol |
| D. Build tests once (`nextest archive`), upload, run shards from the archive | No repeated test compilation | Artifact transfer and remap complexity for a build that is short with a warm cache; revisit if the compile dominates |
| E. `--partition hash:i/n` instead of `count:i/n` | A test stays in its shard when others are added | Uneven shards: the few heavy tests can land together, and they are what sets the time here |
| F. A matrix of whole `cargo test -p <crate>` jobs | No new tool | Almost all the time is in one crate; it balances nothing |
| G. Per-job concurrency groups, so a new commit cancels work job by job | A finished job survives a newer push | `ci.yml` already has one workflow-level group that cancels a superseded run whole; per-job groups only help when the gate waits for the newer run, which this design has no need of |
| H. A label-gated full matrix: the expensive jobs run only when a PR carries a label | Cheap pushes while iterating | The whole pipeline now takes about 4.5 minutes, so there is no expensive tier to gate, and a label is one more way for a required job to skip |
| I. A workflow that re-runs the checks of every layer of a stacked PR when a layer above goes green | Spares lower layers their own full run | This repository does not stack pull requests; the extra workflow would have nothing to act on |

## Decision

*In the context of a single 15-minute `rust` job dominated by a few heavy tests run serially, facing
a timeout the job kept hitting, we decided to split `checks.yml` into parallel jobs (lint, release build and
records gate, three nextest test shards, four acceptance shards and an acceptance coverage-gate merge),
shard with `cargo nextest --partition count:i/n`, and join everything in a `result` job and then the
`ci` job (same name, now a script-checked gate), both of which fail unless every job they need succeeded (a skip is a failure,
except `changeset` outside a pull request), to achieve a wall-clock set by the slowest job rather than
the sum, accepting more runner-minutes and a shard protocol for the acceptance runner to maintain.*

Details that the decision fixes:

- **nextest** is pinned and checksummed in one composite action, `.github/actions/install-nextest`,
  with a pin test, the way knope is. `-E 'not binary(acceptance)'` keeps the custom harness out of it.
- **One definition of the test commands**, `.github/scripts/ci-tests.sh`, used by the workflow and by
  `make ci`, always under `test-no-leak.sh`. Without nextest, `make ci` falls back to `cargo test` on a
  laptop and says so; with `CI` set it fails instead.
- **Acceptance sharding**: `URZUA_ACCEPTANCE_SHARD=i/n` deals the cases round-robin in discovery order;
  each shard writes what it proved, and `URZUA_ACCEPTANCE_MERGE=<dir>` judges the coverage gate over the
  union. The merge fails when a shard file is missing, repeated, or from a different discovery, or
  when the shards ran fewer cases than were discovered. Per-case isolation (corpus plus case keyed
  scratch directories) is unchanged.
- **Gate**: `.github/scripts/ci-gate.sh` (planted-violation cases in `ci-gate.test.sh`, run by CI and
  `make ci`) treats failure, cancellation, a skip and a missing result as red.
- **Cache**: one `rust-cache` key per kind of work (lint, release, test), written only by runs on
  `main`; pull requests restore it and never write, so branches cannot evict it. Debug info is cut to
  line tables for test builds.

## Reversibility

High. It is workflow, script and one test-harness switch; reverting the PR restores the single job.
The `ci` name, which the ruleset requires, does not change. The only external effect is the runner-minutes.

## Consequences

- Wall-clock time is set by the slowest shard plus the acceptance merge; the numbers are recorded in the
  PR that implements this. The shard counts (3 and 4) are numbers to raise when a shard nears its timeout;
  `if: matrix.shard == '1/3'` for the doctests names the first test shard.
- More jobs means more runner-minutes per push and several duplicated checkouts (full history, because
  drift and blame tests read it).
- A new `harness = false` test target would need the same treatment as `acceptance`, or nextest fails
  at list time with an unambiguous error.
- nextest does not run doctests; `ci-tests.sh doctest` does, in the first test shard.
- A test that depended on running in the same process as another (shared statics) now runs alone;
  none did here (the full suite passed under nextest).
- Options G to I are patterns seen in other CI setups and deliberately not adopted; the table gives the
  reason for each, and each can be revisited if the pipeline grows an expensive tier or stacked PRs.

## References

`ADR-47` (one checks definition), `ADR-95` (the `ci` check is required), `SPEC-20` (the pipeline),
`SPEC-4` (the acceptance suite and its coverage gate), `BUG-193` (the leak guard), `PR #428`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Filed as Proposed. **Why:** maintainer request of 2026-11-07 to shard CI tests; the rust job took 14 to 15 minutes against a 15-minute timeout. | **substantive** |
> | 2026-11-08 | Status changed from Proposed to Accepted; the measured CI time and the current test counts are recorded in the Context. **Why:** Maintainer decision, 2026-11-08. | **substantive** |
