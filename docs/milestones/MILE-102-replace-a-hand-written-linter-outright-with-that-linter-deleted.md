---
Status: Planned
Stable-Id: 01M2W1994CH6D121YBRPAD620R
Phase: '1'
Track: schema-governance
Implements: SPEC-1
Blocked-on: MILE-98
---
# 102 — Replace a hand-written linter outright, with that linter deleted

## What

`SPEC-1`'s third success criterion, and the one it calls **the real bar**:

> It replaces the hand-written linter in at least one codebase outright, with that linter deleted.
>
> Criterion 3 is the real bar. A tool that runs *alongside* the thing it was meant to replace has not
> replaced it.

## Why it is filed late

It had no milestone, and was then **deleted** from `SPEC-1` while narrowing that spec -- in the change
whose stated purpose was filing the untracked exit criteria. Two of four were dropped instead. Caught
by review.

That is worth recording rather than quietly correcting: a criterion with nothing tracking it is
invisible, and the thing most likely to happen to an invisible criterion is that someone tidies it
away.

## What "outright" means

Not "urzua also runs". The existing linter is removed from that repository, and whatever it checked is
either expressed in `.urzua/config.yaml` or consciously dropped with the reason recorded. A rule that
cannot be expressed is a finding about the schema (`MILE-51`'s standard), not a reason to keep both
tools.

## Blocked on the document model, not on `MILE-100`

The adopter's codebase is the hand-written linter to replace, and the replacement is itself the
engagement that gives urzua its second real codebase. `MILE-100` (a second codebase green, with a
different primary language) is Phase 1 and does not gate this switchover. What does gate it is the
engine work the adopter's checks need: the declared document model (`MILE-98`), identity for the
adopter's bare numeric ids (`MILE-126`), declared status vocabulary and status-dependent fields
(`MILE-132`), and the checks filed for the adopter's inventory (`MILE-136` to `MILE-143`).

## Consciously dropped: numbering gaps

The adopter's linter warns on gaps in the sequence of record numbers. This is dropped, on purpose.
Renumbering a record is forbidden (identity is stable), so a gap can never be fixed by editing the
records that surround it, and a gap carries no defect: it is a vacated number, which the tombstone
convention already covers. A rule that can only warn and cannot be acted on would be a standing
warning, which the Phase 0 exit forbids.

## Done means

- **Findings are diffed before the old linter is deleted.** Both linters run on the adopter's corpus
  and their findings are compared. Any check the old linter catches that urzua misses blocks the
  switchover until it is closed by a rule or consciously dropped with the reason recorded.
- **The CI wiring is replicated.** One job runs on every pull request and every push to the default
  branch, with no label or path gate and with full history checked out. It feeds the required gate:
  an error finding blocks, and a warn-only step does not. The base ref is passed on pull requests so
  diff-based checks can run, and a requested diff that fails is a hard error, not a silent downgrade.
- **A pre-commit hook runs the integrity checks** (no base ref, so no diff checks).
- **An editor hook blocks an Embodiment edit** that disagrees with the value urzua computes, with an
  explicit logged override as the only way past it.
- **The one-off scripts are written and run:** the filename rename to the declared convention, and the
  header-shape rewrite to the canonical shape (`MILE-85`). Neither is a shipped command.
- **The old linter is deleted** from the adopter's repository, together with its CI steps and hooks,
  and urzua governs that repository with zero standing warnings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** `SPEC-1`'s criterion 3, which that spec calls the real bar, had no milestone and was then deleted from the spec by the change meant to file its untracked criteria. Recovered from `main` and filed. | **substantive** |
> | 2026-11-04 | Removed the block on `MILE-100`: `Blocked-on` now names `MILE-98`, `MILE-126` and `MILE-132`. Recorded that gaps in record numbers are consciously dropped, because renumbering is forbidden and a gap carries no defect. Added Done-means: findings of both linters diffed before deletion, the CI wiring to replicate, the pre-commit and editor hooks, and the one-off rename and header-rewrite scripts. `Status` and `Phase` unchanged. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); `MILE-100`'s second codebase is Phase 1, so the switchover cannot wait on it, and the exit needs the replacement proven against the adopter's own findings. | **substantive** |
> | 2026-11-04 | `Phase` 0 -> 1; `Status` unchanged. **Why:** Maintainer decision, 2026-11-04: Phase 0 is bootstrapping and capability parity in urzua's own repository, and the adopter switchover follows it; `SPEC-1`'s exit bar now lists this as a Phase 1 criterion. | **substantive** |
> | 2026-11-04 | `Blocked-on` now reads `MILE-98`. **Why:** `narrative-field.stale` flagged a blocker that reached `Done`, and the delivery backfill of `MILE-177` set it; the remaining blocker is still open. `Status` unchanged. `MILE-126` and `MILE-132` are Done. | **substantive** |
