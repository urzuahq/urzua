# 0020 — `urzua fix --apply`: writes real files, gated hard

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:rust/crates/urzua-core/src/fix.rs, code:rust/crates/urzua-io/src/lib.rs, code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-core/src/fix.rs
> Date: 2026-09-05
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0008 (Accepted), ADR-0015 (Accepted), ADR-0019 (Accepted)

## Context

ADR-0019 shipped `urzua fix` detect-mode only, naming apply mode's three missing dependencies:
real file mutation of a specific field without disturbing anything else, identity resolution
(RFC-0002 §2's tiering), and a revision-log write-path (ADR-0014 defined the shape, never the
append mechanics). All three now exist.

## Decision

In the context of ADR-0019's deferred apply mode, facing the same eligibility test ADR-0015 already
set (derivable, single-valued, non-destructive, reversible), we decided apply mode ships with every
one of the following as a hard gate, not a nicety:

- **`--ids <record,...>` or `--force` is required.** Nothing applies by default; `--force` is never
  implied.
- **Identity is required.** `--by` wins outright; else `gh api user`; else `git config user.name`
  (RFC-0002 §2's tiering); none resolving is a hard error, never a placeholder author.
- **A missing Revision log section refuses that record's write outright.** ADR-0014's reversibility
  clause is not optional per-write — a repair with nowhere to record itself is not applied, the
  record is reported as failed, and every other selected repair still proceeds independently.
- **Re-verification at write time.** The exact file is re-read from disk immediately before writing;
  `apply_repair` itself fails if the current value it expects is no longer present, which is
  precisely the shape a concurrent edit since detect ran would take (RFC-0008 §4).
- **A corpus-level lock** (`.urzua/cache/fix.lock`, atomic `create_new`) serializes concurrent
  `--apply` invocations; a second process fails loud rather than interleaving writes.
- **The edit touches only the one field's line.** The rest of the record — every other field, all
  prose — is untouched, byte-for-byte, verified by test.

Partial failure is a first-class outcome: selecting five repairs where one record has no revision
log applies the other four and reports the fifth as failed, rather than the batch succeeding
silently short or aborting everything for one record's gap.

## Reversibility

Every applied write is itself reversible by construction: `git diff`/`git revert` on the commit, and
the revision-log row the write appends names the exact evidence and prior value, so "what changed
and why" is answerable without git at all.

## Consequences

- `Command::Fix` gains `--apply`, `--ids`, `--by`, `--force`. None of these existed as flags before
  this ADR — ADR-0019 deliberately didn't add them ahead of a real apply path.
- `urzua-io` gains `resolve_identity` and `FixLock` — the first genuinely mutating capability
  anywhere in this codebase. Both live in `urzua-io`, not `urzua-core`, keeping the purity boundary
  (ADR-0005/0006) intact: `urzua-core::fix::apply_repair` is a pure `&str -> Result<String, String>`
  transform; the CLI performs the actual read, lock, identity resolution, and write.
  `urzua-core`'s purity test must still pass with this ADR's dependencies in place — asserted, not
  assumed.
- `migrate schema --apply`'s planned delegation to `urzua fix` (designed alongside RFC-0008's
  acceptance) is now genuinely unblocked, since a real apply path exists to delegate to.
- Two things this ADR deliberately does not build: a stale-lock cleanup mechanism (a crashed process
  leaves `fix.lock` behind, requiring manual removal — acceptable for a first cut, a real gap) and
  Tier 2/3 repairs (still only Embodiment, still only Tier 1).

## References

- ADR-0019 — the detect-mode-only decision this ADR completes.
- ADR-0015/RFC-0008 — the eligibility test and tier model this apply path implements.
- ADR-0014 — the revision-log shape every applied write appends to.
- RFC-0002 §2 — the identity-resolution tiering reused here.
