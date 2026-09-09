---
Version: '0.4'
Date: 2026-09-07
Status: Accepted
Author: '@beauwilliams'
Subject: 'The `bug` record type -- real, already-found defects and required regression-test accountability.'
Implements: ADR-35
Parent: —
---
# SPEC-9 — The `bug` record type

## Purpose

Tracks a real defect this project's own tooling had — retrospective, not prospective: what was
actually wrong, how it was found, and a named, checkable pointer to the test proving it's fixed.
`bug` is a sibling of `milestone` (SPEC-6) and `waiver` (SPEC-10), all configured record types with
zero `urzua-core` changes, but answers a genuinely different question than either: a milestone is
*planned* work moving toward done, a bug is *what already broke*.

## Why not fold this into `milestone`

Checked directly before building a new type: forcing a bug into milestone's schema (`Status`,
`Phase`, `Track`) either drops the one real governance gain a bug record needs — a named pointer to
the regression test proving the fix — or bolts it on as an optional field nothing requires. A bug
record's whole value is making "nobody verified this is actually fixed" impossible to state by
accident.

## Schema

```toml
[record_types.bug]
dir = "docs/bugs"
required_fields = ["Status", "Found-in", "Regression-test"]
header_shape = "yaml-frontmatter"
known_fields = ["Realized-by", "Stable-Id"]
spec = "SPEC-9"
```

| Field | Values | Notes |
|---|---|---|
| `Status` | `Open` \| `Fixed` \| `WontFix` | The bug's own lifecycle, independent of any milestone or ADR. |
| `Found-in` | free text | How/where it was actually discovered — a narrative, not a resolvable reference (e.g. `"check docs/adr/ vs check docs/ compared by hand"`). |
| `Regression-test` | free text | The specific test name/path proving the fix — **required, not optional**. A `Status: Fixed` bug with a blank or placeholder `Regression-test` is a `field.quality` finding for free; no new rule was needed. |
| `Realized-by` | optional, `known_fields` | Reused, generic — points at the actual fix commit/function, same evidence field every other type uses. |
| `Stable-Id` | optional, `known_fields` | Assigned by `urzua new bug` or backfilled by `migrate ids` (SPEC-14), same as any other type. |

No `Phase`/`Track` — a bug isn't planned or grouped the way a milestone is; it either exists because
something broke, or it doesn't exist yet.

## What's deliberately not built

- **Severity or priority fields.** Not named as a real gap yet — `Status` alone has been sufficient
  for every bug this corpus has recorded. Add only once a real case needs to distinguish urgency.
- **A generated bug-index view**, for the same reason SPEC-6 declines one for milestones: build once
  `urzua check docs/bugs/` stops being sufficient to read the backlog, not before.

## References

- ADR-35 — the decision this spec details: `bug` as a configured type, zero `urzua-core` changes.
- ADR-34/SPEC-6 — the `milestone` precedent this type is a sibling of, and the reasoning it follows.
- ADR-11/SPEC-10 — the `waiver` precedent, same footprint.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec. **Why:** `bug` (ADR-35) was documented only as an ADR while `milestone`, the same shape of decision (a configured record type), got a matching spec (SPEC-6) — MILE-77 named this inconsistency and this spec resolves it for `bug`. | **structural** |
> | 2026-09-08 | Header shape changed from `blockquote` to `yaml-frontmatter`, `header_layout` removed (ADR-33/38 amendments); `spec = "SPEC-9"` declared in config, closing `type.no-declared-spec`'s live finding for this type (ADR-43). | **substantive** |
> | 2026-09-08 | Bumped to `0.3`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`; also corrected `Parent` from `SPEC-1` to `—` (`BUG-10`): this spec's real lineage is already stated via its own `Implements`/`Derives-from`, not a narrowing of `SPEC-1`'s v0-CLI scope. | **structural** |
