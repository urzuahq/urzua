---
Version: '0.11'
Date: 2026-09-07
Status: Accepted
Author: beauwilliams
Subject: '`urzua fix` -- detecting and applying Embodiment corrections computed from evidence.'
Implements: ADR-15, ADR-18, ADR-19, ADR-20
Parent: SPEC-1
---
# SPEC-8 — `urzua fix`

## Purpose

`fix` is the tool-authored repair path: it detects fields whose stated value disagrees with what the
tool can compute from evidence already in the tree, and — only under a strict eligibility test — can
write the computed value back. This spec is the complete, current build of that path: detect mode,
apply mode, the eligibility test that gates what may ever be written, and the safety mechanics apply
mode enforces as hard gates, not niceties.

## Eligibility: what a tool may ever write (ADR-15)

A field is **tool-writable** if and only if it is:

- **Derivable** — computed from evidence already in the record tree, no inference.
- **Single-valued** — one correct answer, not a judgment call.
- **Non-destructive on write** — the value it replaces is a stale copy of the *same computation*,
  never human-composed prose.
- **Reversible** — via ADR-14's revision log, so the write is undoable by inspection.

Anything failing any clause stays permanently hand-authored. No confidence score makes prose
eligible — the eligibility test is a closed set of clauses, not a threshold to tune.

**One computation exists today, Tier 1**: recompute `Embodiment` from `Realized-by`'s categorized
locators (ADR-18). `Embodiment` itself is explicitly excluded from *auto-repair* despite satisfying
every clause: a stated-exceeds-computed mismatch could be "resolved" by lowering the claim, which is
the mechanical answer and the wrong one whenever the evidence is merely missing rather than the claim
being wrong (RFC-12 §5). `fix` therefore *reports* the Embodiment mismatch (this is
`check`'s own `embodiment.consistency` rule, surfaced here in repair-shaped form) but does not
silently downgrade a stated value on `--apply` without the record being explicitly selected.

Two further tiers are named, not built: completing a half-stated relationship additively (Tier 2),
and repointing a stale locator only when the move is unambiguously recoverable (Tier 3). Each ships
only once a real case demands it, in increasing order of nerve required.

## Detect mode (default, read-only, CI-safe)

`urzua fix` with no flags: for every record with a stated `Embodiment` and a `Realized-by`, computes
the expected tier and reports a disagreement as a structured entry:

```json
{ "record": "docs/adr/ADR-1-x.md", "field": "Embodiment", "line": 3, "current_value": "...", "computed_value": "...", "tier": 1, "evidence": "Realized-by: code:src/a.rs" }
```

A record whose `Realized-by` locator changed since the `Realized-by` line was last touched (ADR-32)
computes to `Drift detected` regardless of the locators' tier, exactly as `check`'s
`embodiment.consistency` does: `fix` reads the same drifted-record set `check` computes, so the two
never disagree. The repair is written like any other Tier 1 repair, and a stated `Drift detected`
that is no longer drifted (the `Realized-by` line was touched again) is repaired back to the
tier-computed value. `fix` computes drift only when `embodiment.consistency` is enabled, as `check`
does (`MILE-159`): with the rule off or undeclared there is no embodiment verdict to repair, so `fix`
reports it could not run instead of computing a verdict from an empty drift set. It applies the same opt-in `@sha256:` locator pins and
`drift_grace_days` window `check` does (`MILE-129`), so the two still never disagree; `fix` does not
rewrite a pin, which stays a hand edit.

Git trouble is never read as "no drift" (`MILE-159`). A shallow clone, detected once per run, and any
git call that fails while computing drift (a commit the repository does not have, say) are reported as
`notices` with subject `embodiment-drift-git`, in `check` and in `fix` alike; the affected records are
not judged for drift. The per-record `git blame` calls run on a few threads.

### Scoping to paths

`urzua fix [PATHS...]` examines, computes drift for and repairs only the records the paths select
(`MILE-159`). A path is a record file or a directory; a record is selected when its repo-relative
path equals the path or sits under it. Paths are relative to the working directory or absolute. The
population counts the selected records only, and `git blame` and the drift computation run for those
records alone, so a single-record run costs a fraction of a whole-corpus run. The corpus a
back-pointer resolves against stays whole.

A path that selects no record, or lies outside the repository, is reported in `failed` naming it
(`partial-failure`, exit 1), never skipped. In apply mode `--ids` and `--force` select among the
scoped repairs only, so an `--ids` entry outside the given paths matches no pending repair and fails
as above.

### Judging proposed content (`--stdin-path`)

`urzua fix --stdin-path <path>` reads the proposed content of one record from stdin and judges it
exactly as it would the file at `<path>`, without writing anything (`MILE-159`). It is the entry
point for an editor hook that wants a verdict before the file is saved; an example script is
`.git-hooks/examples/editor-fix-hook.sh`, which nothing installs.

- **The path need not exist on disk.** A new record is judged from the proposal alone. The record
  type is the one whose `dir` directly holds the path, under the same filename rules discovery
  applies; a path no type governs is `not-run` (exit 2) with an `error` naming the reason, never an
  empty `ok`.
- **The proposal replaces the on-disk record** for this run. Nothing else is examined, and drift is
  computed for this one record only: the `Realized-by` line is blamed against the proposed text, so
  an unchanged line keeps its commit, and a line the proposal edits or adds counts as touched now
  (no locator can have moved since). A path with no git history is not drifted.
- **The output is the ordinary report**, and `repairs` is its stable machine-readable list for that
  one path: each entry is `record`, `field`, `line` (the 1-based line of the stated field in the
  proposed content), `current_value`, `computed_value`, `tier` and `evidence`. A hook reads
  `status` (`repairs-available`, `ok`, or `not-run` when the record states no `Embodiment` to
  judge) and `repairs`; `line` is the location to point at. `notices` carry git trouble as in the
  whole-corpus run.
- **It cannot be combined** with `PATHS`, `--apply`, `--ids`, `--by` or `--force`: judging proposed
  content never writes, and the combination is `not-run` (exit 2) rather than a silently ignored
  flag.

`--tier` defaults to `1` and is currently locked there — no other tier is implemented, so accepting
a different value would silently do nothing rather than fail loud. Writes nothing, ever. Exit
non-blocking.

## Apply mode (`--apply`)

Every one of the following is a hard gate — none are optional, none are defaults-on:

- **`--ids <record,...>` or `--force` is required.** Nothing applies without one; `--force` (apply to
  every detected repair) is never implied by `--apply` alone.
- **An `--ids` entry that matches no pending repair is a failure.** It is reported in `failed` naming
  the entry (`partial-failure`, exit 1) and writes nothing, never a clean `ok` (`BUG-166`). An entry is
  matched against a record's repo-relative path; accepting a record id is not defined here.
- **Identity is required**, resolved via `urzua-io::resolve_identity` (`gh api user` wins even over
  an explicit `--by`, which wins over `git config user.name` -- ADR-31's amendment); none resolving
  is a hard error, never a placeholder author. A `--by` that diverges from an authenticated `gh`
  login surfaces as a `notices` entry (ADR-46) rather than being silently overridden or discarded.
- **A missing Revision log section refuses that record's write outright, where the log is required.**
  A type declares `revision_log: {required: false}` (`MILE-159`, SPEC-3) to be written without a row;
  `true` is the default. The error names the record and the identity looked for. ADR-14's reversibility
  clause is enforced per write, not assumed — a repair with nowhere to record itself is reported
  failed, and every other selected repair still proceeds independently. Partial failure is a
  first-class outcome, not a batch-wide abort.
- **Re-verification at write time.** The exact file is re-read from disk immediately before writing;
  the pure `apply_repair` function itself fails if the value it expects to replace is no longer
  present — the shape a concurrent edit since detect ran would take.
- **A corpus-level lock** (`.urzua/cache/fix.lock`, atomic `create_new`) serializes concurrent
  `--apply` invocations; a second process fails loud rather than interleaving writes. No stale-lock
  cleanup exists yet — a crashed process leaves the lock file behind, requiring manual removal. A
  real, named gap, not silently accepted as fine.
- **The edit touches only the one field's line.** Every other field and all prose is untouched,
  byte-for-byte — verified by test, not just intended.
  That includes the file's line endings and trailing-newline state, and the new revision-log row
  repeats the existing table's own row prefix (`BUG-164`, `BUG-165`).

Every applied write appends a `structural` revision-log entry naming the computation and the
evidence read, so the write is reversible both by `git revert` and by reading the corpus itself.

## Purity boundary

`urzua-core::fix::apply_repair` is a pure `&str -> Result<String, String>` transform. The CLI
(`urzua-cli`, via `urzua-io`) performs the actual file read, lock acquisition, identity resolution,
and write. `urzua-core`'s purity test (ADR-5/6) must pass with `fix`'s dependencies in place —
asserted by test, not assumed by convention.

## What's deliberately not built

- **Tiers 2 and 3** — additive relationship completion and locator repointing. Ship only once a
  real case needs them.
- **Stale-lock cleanup** — a crashed `--apply` process's lock file requires manual removal today.
- **`migrate schema --apply`'s delegation to `fix`** — named as a consequence of apply mode existing
  (ADR-20), not yet wired; blocked on deciding which newly-required fields are ever tool-writable
  (structurally, almost none — see SPEC-14).

## References

- ADR-15 — the eligibility test and tier model.
- ADR-18 — the one Tier-1 computation (Embodiment).
- ADR-19 — detect mode shipped alone, apply mode's three then-missing dependencies named.
- ADR-20 — apply mode's hard gates, once those dependencies existed.
- ADR-14 — the revision-log shape every applied write appends to.
- RFC-2 §2 — the identity-resolution tiering apply mode reuses.
- RFC-12 §5 — why Embodiment is excluded from auto-repair despite satisfying every eligibility clause.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec, bundling ADR-15/18/19/20 into one buildable document for the `fix` feature area. **Why:** MILE-77 named this as a real feature area (an eligibility test, one computation, a detect/apply split, and apply's hard gates) that was documented only as four separate ADRs, none of which was a complete build reference on its own. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-09-11 | Corrected the identity-resolution order, stale since ADR-31's amendment (`gh api user` now wins even over `--by`, which wins over `git config user.name` -- not "`--by` wins outright" as this spec still said). Noted the new `notices` field (ADR-46) for a `--by`/`gh` divergence. **Why:** found live, touching this exact paragraph for the `notices` addition -- leaving the identity-order claim wrong while editing the sentence next to it would have been the same "found it, patched around it" mistake this project's own `AGENTS.md` already warns against. | **substantive** |
> | 2026-11-02 | Bumped to `0.5`. Apply mode now fails an `--ids` entry that matches no repair, and keeps CRLF endings, the absent trailing newline and a plain-table log's row prefix. **Why:** round 29 found each as a silent no-op or a whole-file rewrite (`BUG-164`, `BUG-165`, `BUG-166`). | **substantive** |
> | 2026-11-03 | Bumped to `0.6`. `fix` now computes `Drift detected` (ADR-32) from the same drifted-record set `check` uses, so it is detected and, on `--apply`, written like any other Tier 1 repair. **Why:** MILE-15 -- ADR-32 named this as separate follow-up work. | **substantive** |
> | 2026-11-04 | Bumped to `0.7`. Noted that `fix` reads the same opt-in locator pins and grace window as `check` and never rewrites a pin. **Why:** `MILE-129` -- `fix` shares `check`'s drifted-record set, so a new drift input changes what it repairs. | **substantive** |
> | 2026-11-04 | Bumped to `0.8`. `fix` computes drift only when `embodiment.consistency` is enabled and reports git failures and shallow clones as `embodiment-drift-git` notices. **Why:** `MILE-159` -- drift ran unconditionally (about 1.1s on this corpus), and git errors read as a clean zero, which is how a shallow clone once produced a false zero. | **substantive** |
> | 2026-11-04 | Bumped to `0.9`. `fix` takes `PATHS` that scope examination, drift and repair, and reports a path matching no record. **Why:** `MILE-159` -- an editor hook needs a quick single-file refresh, and the whole-corpus run is about 0.8s on this corpus against about 0.15s for one record. | **substantive** |
> | 2026-11-04 | Bumped to `0.10`. `fix --stdin-path <path>` judges proposed content for one record without writing, for a path that need not exist yet, with drift computed for that record alone; each repair carries the 1-based `line` of the stated field; the detect-mode example now shows the real snake_case shape. **Why:** `MILE-159` -- an editor hook judges a file before it is saved and must find its own repair without parsing the whole list. | **substantive** |
> | 2026-11-04 | Bumped to `0.11`. Apply mode writes without a revision-log row for a type declaring `revision_log: {required: false}`; a required log (the default) still refuses a record with no section, now naming the identity looked for. **Why:** `MILE-159` -- an adopter's records have no revision log section, so `--apply` could never run on them. | **substantive** |
