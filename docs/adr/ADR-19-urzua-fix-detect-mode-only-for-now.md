---
Stable-Id: 01M6R3NK48R3KZZ3SZE3FYKSXA
Status: Accepted
Embodiment: Drift detected
Realized-by: code:rust/crates/urzua-core/src/fix.rs@sha256:c9cd87e7b00e4640147d759dc80ea16b5aa076ac79cfc5890055d0fdde7c1e56, test:rust/crates/urzua-core/src/fix.rs@sha256:c9cd87e7b00e4640147d759dc80ea16b5aa076ac79cfc5890055d0fdde7c1e56
Date: 2026-09-05
Accepted-on: 2026-09-05
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: RFC-8, ADR-15
---
# 19 — `urzua fix`: detect mode only, apply mode deferred

## Context

ADR-15 specifies `urzua fix` in full: detect mode (read-only) and apply mode (`--ids`/`--by`,
`--force`, tier gating). Detect mode needs only a pure computation over already-parsed records —
ADR-18's Embodiment MVP supplies exactly one, Tier 1's "recompute a derived field in place." Apply
mode needs three things that don't exist yet: real file mutation of a specific header field without
disturbing the rest of the record, identity resolution (`gh api user` → `git config` → override, per
RFC-2 §2's tiering), and a revision-log write-path — ADR-14 defined the revision log's shape but
never its append mechanics.

## Decision

In the context of ADR-15 specifying both modes together, facing apply mode's real dependencies not
existing yet, we decided to ship **detect mode only** in this pass: `urzua fix` (default, `--tier`
locked to `1` since no other tier is implemented) reports Tier-1 Embodiment repairs as structured
output, exits non-blocking, and writes nothing. `--ids`, `--by`, and `--force` are not yet flags on
the command — adding them ahead of an apply path that doesn't exist would be exactly the
half-finished-implementation shape this project avoids elsewhere.

ADR-15 itself already frames this as sufficient: "`urzua fix` (detect mode, default) ... Safe in
CI; this alone is a Phase 1 feature."

## Reversibility

Additive: apply mode is a strict superset of what exists now — new flags, an identity resolver, a
write path, and a revision-log append — none of which change detect mode's behavior or output shape
when they land.

## Consequences

- `urzua fix` never writes. A user expecting `--fix`-style auto-repair (the RFC-8 pitch) gets a
  report today, not a repair; this must be documented plainly rather than implied.
- Apply mode is blocked on the revision-log write-path (ADR-14 defines the shape; nothing appends
  to it yet) and on identity resolution (RFC-2 §2, not yet implemented in this codebase). Both are
  real, separate pieces of follow-up work, not sub-tasks of this ADR.
- `migrate schema --apply`'s delegation to `urzua fix` (designed alongside RFC-8's acceptance)
  stays blocked until apply mode exists — tracked, not silently dropped.

## References

- ADR-15 — the full `urzua fix` specification, of which this ADR ships the detect-mode half.
- ADR-18 — the one Tier-1 computation (Embodiment) this detect mode currently knows about.
- RFC-2 §2 — the identity-resolution tiering apply mode will need.
- ADR-14 — the revision-log shape apply mode's writes must append to.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Embodiment` recomputed by `urzua fix`: `Drift detected`. **Why:** every file named in `Realized-by` has changed since this line was last verified, and this pass did not re-verify it, so the record states the drift instead of a stale `Verified`; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-core/src/fix.rs, test:rust/crates/urzua-core/src/fix.rs` -- stated `Verified` disagreed with the computed value. | **structural** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit a09084af7. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/fix.rs@sha256:c9cd87e7b00e, rust/crates/urzua-core/src/fix.rs@sha256:c9cd87e7b00e Reason: Pinned the files as they are now after checking each against the record it evidences: header.rs still parses the declared YAML frontmatter shape (ADR-17), fix.rs still has detect_repairs (ADR-19), urzua-id still builds ULID stable ids (ADR-21), main.rs still prints JSON with no --format flag on check (ADR-23), urzua-io still resolves the attesting identity in the ADR-31 order, scripts/generate-dashboard.py still generates the static dashboard (ADR-37), report.rs still defines Report and Notice and main.rs still emits them (ADR-46), publish-release.yml still verifies the tree it ships through checks.yml (ADR-47), knope.toml still releases from changesets (ADR-48), and ci.yml still gates changesets on .github/release-paths (ADR-49). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
