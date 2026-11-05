---
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-io/src/lib.rs@sha256:788d6af3b7181c460ff2dfa60d38e88ce4cedb38339aff74f397adc4481b9389, code:rust/crates/urzua-core/src/rules/embodiment.rs#compute_embodiment@sha256:337ee455b133de479b08a6a8aa5a77596e10120d41610cbf5764e2b5462bb0b3, code:rust/crates/urzua-cli/src/discovery.rs#compute_drifted_records_at@sha256:68d5fe68290101f4e9759e67287017a4cca0a5208cddd75336b2fe795e91de52, test:rust/crates/urzua-io/src/lib.rs@sha256:788d6af3b7181c460ff2dfa60d38e88ce4cedb38339aff74f397adc4481b9389, test:rust/crates/urzua-core/src/rules/embodiment.rs#a_drifted_record_computes_to_drift_detected_regardless_of_tier@sha256:382ca34e713c29eba50cacb1ed4a29cbdecff3658b5dfc542fdc325f5b42f515
Date: 2026-09-07
Accepted-on: 2026-09-07
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: RFC-5
---
# 32 — Drift detection via git history on the `Realized-by` line, not a stored hash

## Context

RFC-5 §2 named tier 1 of its confidence ladder — "pointer only, unchanged since last
verification (via git history on the locator)" — as a real, cheap signal, and ADR-18 explicitly
excluded "per-locator staleness verification against git history" from the Embodiment MVP. The
`Embodiment::DriftDetected` variant has existed in the schema since the start ("alarm state,
reachable from any other state") and has never been computed by anything.

Two real design questions had to be settled before writing any code, not discovered after:

1. **What anchors "last verification"?** A stored hash or commit SHA on the record would be a
   schema change to `Realized-by` — the exact format ADR-18 deliberately kept flat and simple.
   The alternative, staying schema-free: compare the `Realized-by` *line*'s own last-touched commit
   (via `git blame` on that specific line, not the whole file — a record edited for something
   unrelated, e.g. fixing an `Author` typo, must not silently reset the staleness clock) against
   each locator's last commit, via `git merge-base --is-ancestor`. No new field, no stored value.
2. **Where does the git-history computation live?** `embodiment_consistency` is a pure
   `urzua-core` function (no I/O, mechanically enforced by `purity.rs`); git history is I/O by
   definition. The computation has to happen in `urzua-io`/`urzua-cli`, producing a plain
   `HashSet<PathBuf>` of "records with at least one drifted locator" that gets *passed into* the
   pure rule — the same shape already used for `full_text` in `filename_title_consistency`.

A third thing surfaced only by checking the actual CI config, not assumed: `.github/workflows/ci.yml`'s
`rust` job checkout has no `fetch-depth` set, which defaults to a depth-1 shallow clone. In a
depth-1 checkout, `git log`/`git blame` can only ever see the single tip commit — every path would
resolve to the same "last commit," making drift detection silently useless in CI specifically,
while working fine on a real developer clone. This would have been a defect discovered only after
shipping, not before, had the checkout config not been checked directly.

## Decision

In the context of RFC-5 tier 1's staleness signal and ADR-18's schema-simplicity constraint, we
decided: **drift detection compares git history, stores nothing new, and lives in the impure layer.**

- `urzua_io::commit_for_line(repo_root, path, line)` — the commit that last touched a specific
  1-indexed line, via `git blame -L <line>,<line> --porcelain`.
- `urzua_io::last_commit_for_path(repo_root, path)` — the commit that last touched a whole file,
  via `git log -1 --format=%H`. Used for locator paths, which aren't single lines.
- `urzua_io::commit_strictly_before(repo_root, ancestor, descendant)` — `git merge-base
  --is-ancestor`, wrapped to return `false` (never an error) when history can't establish an
  order — an ambiguous or missing history reads as "no drift," never a false positive.
- `run_check` (CLI) computes, per record, whether any `Realized-by` locator's last commit comes
  strictly after the `Realized-by` *line's* last commit, producing a `HashSet<PathBuf>` of drifted
  record paths.
- `rules::compute_embodiment` gains a `drifted: bool` parameter: if true, the computed value is
  unconditionally `Drift detected`, regardless of what tier the categorized locators would
  otherwise compute — matching the schema's own description of `DriftDetected` as an alarm state
  reachable from any other state. `embodiment_consistency` takes the drifted set and reports
  disagreement the same way it already reports tier disagreement.
- **`ci.yml`'s `rust` job checkout gets `fetch-depth: 0`** — the same fix already applied to the
  `changeset` job, for the same underlying reason (git history has to actually be there to read).

**Scoped out of this decision, named rather than silently dropped:** `fix --apply` writing
`Embodiment: Drift detected` automatically. `fix::detect_repairs` has the same purity constraint as
`embodiment_consistency` and would need the same drifted-set plumbing; wiring it in is real,
separate follow-up work, not implied by this ADR. `check` reports drift; only a human (or a future,
separately-decided `fix` extension) acts on it.

## Reversibility

Additive: new `urzua-io` functions, a new parameter on an existing pure function (an internal
signature change, not a public CLI contract change), and a CI checkout depth change. Nothing
existing is removed or reshaped. `fetch-depth: 0` costs a slightly slower checkout in CI in
exchange for correct history — cheap at this repo's size, revisit only if that changes.

## Consequences

- A record whose `Realized-by` line hasn't been touched since a cited locator changed now surfaces
  a `Drift detected` disagreement from `check`, using nothing but git history already present.
- The staleness clock is scoped to the `Realized-by` line specifically, not the whole file — an
  unrelated edit elsewhere in the record (fixing a typo, updating `Deciders`) does not silently
  mask real drift.
- CI now needs full git history to run `check` correctly; a future contributor adding a job that
  runs `check` must remember this, same as `changeset`'s job already does — worth checking if this
  pattern needs its own rule later (a job running `check` with a shallow checkout produces a
  passing-but-meaningless drift result, never an outright error) rather than repeating the
  discovery cost per job.
- `fix`-writability of `Drift detected` remains a real, named gap.

## Amendment (2026-11-04): missing history is an error, `fix` writes the drift, and pins refine it

*Status of this amendment: accepted 2026-11-04 by the maintainer (drafted 2026-11-04 from the drift review the maintainer approved). The Decision text above is unchanged.*

Three statements in the Decision no longer match the code. Detection by `git blame` on the
`Realized-by` line, nothing stored on the record and the computation living in the impure layer all
stand, and that blame comparison remains the path for an unpinned locator (`urzua-cli/src/discovery.rs`).

- **Missing history is an error, not "no drift".** The Decision says `commit_strictly_before` is
  "wrapped to return `false` (never an error)" so that "an ambiguous or missing history reads as 'no
  drift'". `#214` reversed this on purpose: it returns a `Result`, and a git failure (a commit the
  repository does not have, as in a shallow clone) is an `Err` that the check reports, "never a quiet
  no drift" (`MILE-159`, `#229`). `is_shallow_repository` and `commit_for_line_in` were added with it.
  The Consequences line that a shallow checkout "produces a passing-but-meaningless drift result,
  never an outright error" is likewise no longer true.
- **`fix` writes `Drift detected`.** The Decision scopes out `fix --apply` writing `Embodiment: Drift
  detected`. `urzua fix` now writes it, as the revision rows of `ADR-45` show, and the Consequences
  line "`fix`-writability of `Drift detected` remains a real, named gap" is closed.
- **Drift is no longer unconditional.** `ADR-67` refines it: a pinned locator compares a content hash
  and does not consult git history, `drift_grace_days` holds back a recent change, and a stated
  `Drift detected` is accepted inside that window (`#182`).

## References

- RFC-5 §2, tier 1 — the staleness signal this ADR implements.
- ADR-18 — the Embodiment MVP this extends without changing its schema.
- `rust/crates/urzua-core/src/rules.rs` — `compute_embodiment`, `embodiment_consistency`.
- `rust/crates/urzua-io/src/lib.rs` — `commit_for_line`, `last_commit_for_path`,
  `commit_strictly_before`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | `Realized-by` locators pinned by content hash (`@sha256:`) after re-verifying each cited file against the Decision. **Why:** Re-verified against current code; maintainer decision 2026-11-04. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit bf8a93c5a. | **structural** |
> | 2026-11-04 | Amendment added: a git failure is now an error and not "no drift" (`#214`, `#229`), `fix` now writes `Drift detected`, and `ADR-67` made drift conditional on pins and the grace window. **Why:** drift review of 2026-11-04, approved by beauwilliams; proposed, pending maintainer acceptance. The Decision is unchanged. | **substantive** |
> | 2026-11-04 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-io/src/lib.rs` d738b6b7c5c7 -> 671f9e9b0ff5, `rust/crates/urzua-core/src/rules/embodiment.rs` a49eedaabcd0 -> 6b61cdfe6465, `rust/crates/urzua-cli/src/discovery.rs` 4cb230d9ff43 -> 0f25fa8cbe3e. Reason: Re-pinned with the amendment above: git-blame drift detection still holds for unpinned locators; the error handling, fix write and pin refinement are recorded. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-04 | Amendment of 2026-11-04 accepted. **Why:** Maintainer decision, 2026-11-04; the Decision text is unchanged. | **substantive** |
> | 2026-11-04 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/src/discovery.rs` 0f25fa8cbe3e -> 61d7951e332f. Reason: Re-verified: the only change to the pinned file is an Implements: ADR-67 comment; behaviour is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-io/src/lib.rs` 671f9e9b0ff5 -> 06c483542fda. Reason: Re-verified against the Decision and its accepted amendment of 2026-11-04. The remaining file-level pin, urzua-io/src/lib.rs, only gained an added set on GitChanges and sha256_hex_of_str since the pin; commit_for_line_in, last_commit_for_path and commit_strictly_before (the git-history helpers the decision names) are unchanged. The volatile embodiment.rs and discovery.rs evidence is narrowed to compute_embodiment and compute_drifted_records_at: drift still overrides every tier and unpinned locators are still judged by blame on the Realized-by line. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Narrowed the evidence pins on volatile files to scoped pins (`ADR-74`, `MILE-185`): `rules/embodiment.rs` to `compute_embodiment` (code) and `a_drifted_record_computes_to_drift_detected_regardless_of_tier` (test), `cli/discovery.rs` to `compute_drifted_records_at`. Reason: drift review of 2026-11-04. Both items still do what the Decision and its accepted amendment say: `compute_embodiment` returns the declared drift state ahead of every tier, and `compute_drifted_records_at` still compares the blame of the `Realized-by` line with the last commit of each unpinned locator, now also judging pinned and scoped locators by hash, honouring the grace window and reporting git failures. Unrelated edits to those two files no longer make this record stale. **Why:** the pins were set on whole files that change for reasons unrelated to this decision. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-io/src/lib.rs` 06c483542fda -> 788d6af3b718. Reason: BUG-176 added a `last_commit_for_paths` helper to urzua-io (the newest commit among several files, used for a directory or glob locator); the git-history helpers the decision names are unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/src/discovery.rs#compute_drifted_records_at` (lines 398-653) c577cb92c744 -> 68d5fe682901. Reason: BUG-176 let a directory or glob locator drift on the newest commit among its files and added a files-pin branch to compute_drifted_records_at; a file locator is still judged exactly as before, from the git history of the Realized-by line against the cited file, so the decision this record cites is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
