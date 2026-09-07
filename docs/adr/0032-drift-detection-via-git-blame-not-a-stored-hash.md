# 0032 — Drift detection via git history on the `Realized-by` line, not a stored hash

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:rust/crates/urzua-io/src/lib.rs, code:rust/crates/urzua-core/src/rules.rs, code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-io/src/lib.rs, test:rust/crates/urzua-core/src/rules.rs
> Date: 2026-09-07
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0005 (Accepted)

## Context

RFC-0005 §2 named tier 1 of its confidence ladder — "pointer only, unchanged since last
verification (via git history on the locator)" — as a real, cheap signal, and ADR-0018 explicitly
excluded "per-locator staleness verification against git history" from the Embodiment MVP. The
`Embodiment::DriftDetected` variant has existed in the schema since the start ("alarm state,
reachable from any other state") and has never been computed by anything.

Two real design questions had to be settled before writing any code, not discovered after:

1. **What anchors "last verification"?** A stored hash or commit SHA on the record would be a
   schema change to `Realized-by` — the exact format ADR-0018 deliberately kept flat and simple.
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

In the context of RFC-0005 tier 1's staleness signal and ADR-0018's schema-simplicity constraint, we
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

## References

- RFC-0005 §2, tier 1 — the staleness signal this ADR implements.
- ADR-0018 — the Embodiment MVP this extends without changing its schema.
- `rust/crates/urzua-core/src/rules.rs` — `compute_embodiment`, `embodiment_consistency`.
- `rust/crates/urzua-io/src/lib.rs` — `commit_for_line`, `last_commit_for_path`,
  `commit_strictly_before`.
