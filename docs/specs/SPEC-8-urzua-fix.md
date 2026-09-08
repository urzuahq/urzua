---
Version: '0.2'
Date: 2026-09-07
Status: Accepted
Author: '@beauwilliams'
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
{ "record": "...", "field": "Embodiment", "currentValue": "...", "computedValue": "...", "tier": 1, "evidence": [...] }
```

`--tier` defaults to `1` and is currently locked there — no other tier is implemented, so accepting
a different value would silently do nothing rather than fail loud. Writes nothing, ever. Exit
non-blocking.

## Apply mode (`--apply`)

Every one of the following is a hard gate — none are optional, none are defaults-on:

- **`--ids <record,...>` or `--force` is required.** Nothing applies without one; `--force` (apply to
  every detected repair) is never implied by `--apply` alone.
- **Identity is required**, resolved via `urzua-io::resolve_identity` (`--by` wins outright, else
  `gh api user`, else `git config user.name`); none resolving is a hard error, never a placeholder
  author.
- **A missing Revision log section refuses that record's write outright.** ADR-14's reversibility
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
