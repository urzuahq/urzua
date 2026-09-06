# 0019 — `urzua fix`: detect mode only, apply mode deferred

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:rust/crates/urzua-core/src/fix.rs, test:rust/crates/urzua-core/src/fix.rs
> Date: 2026-09-05
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0008 (Accepted), ADR-0015 (Accepted)

## Context

ADR-0015 specifies `urzua fix` in full: detect mode (read-only) and apply mode (`--ids`/`--by`,
`--force`, tier gating). Detect mode needs only a pure computation over already-parsed records —
ADR-0018's Embodiment MVP supplies exactly one, Tier 1's "recompute a derived field in place." Apply
mode needs three things that don't exist yet: real file mutation of a specific header field without
disturbing the rest of the record, identity resolution (`gh api user` → `git config` → override, per
RFC-0002 §2's tiering), and a revision-log write-path — ADR-0014 defined the revision log's shape but
never its append mechanics.

## Decision

In the context of ADR-0015 specifying both modes together, facing apply mode's real dependencies not
existing yet, we decided to ship **detect mode only** in this pass: `urzua fix` (default, `--tier`
locked to `1` since no other tier is implemented) reports Tier-1 Embodiment repairs as structured
output, exits non-blocking, and writes nothing. `--ids`, `--by`, and `--force` are not yet flags on
the command — adding them ahead of an apply path that doesn't exist would be exactly the
half-finished-implementation shape this project avoids elsewhere.

ADR-0015 itself already frames this as sufficient: "`urzua fix` (detect mode, default) ... Safe in
CI; this alone is a Phase 1 feature."

## Reversibility

Additive: apply mode is a strict superset of what exists now — new flags, an identity resolver, a
write path, and a revision-log append — none of which change detect mode's behavior or output shape
when they land.

## Consequences

- `urzua fix` never writes. A user expecting `--fix`-style auto-repair (the RFC-0008 pitch) gets a
  report today, not a repair; this must be documented plainly rather than implied.
- Apply mode is blocked on the revision-log write-path (ADR-0014 defines the shape; nothing appends
  to it yet) and on identity resolution (RFC-0002 §2, not yet implemented in this codebase). Both are
  real, separate pieces of follow-up work, not sub-tasks of this ADR.
- `migrate schema --apply`'s delegation to `urzua fix` (designed alongside RFC-0008's acceptance)
  stays blocked until apply mode exists — tracked, not silently dropped.

## References

- ADR-0015 — the full `urzua fix` specification, of which this ADR ships the detect-mode half.
- ADR-0018 — the one Tier-1 computation (Embodiment) this detect mode currently knows about.
- RFC-0002 §2 — the identity-resolution tiering apply mode will need.
- ADR-0014 — the revision-log shape apply mode's writes must append to.
