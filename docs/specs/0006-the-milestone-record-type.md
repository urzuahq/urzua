# SPEC-0006 — The `milestone` record type

> Version: 0.1 | Date: 2026-09-07 | Status: Accepted
> **Implements:** ADR-0034
> **Parent:** SPEC-0001 (v0 CLI).

## Purpose

Tracks this project's own backlog — decided-but-unbuilt work, explicitly deferred work, undecided
RFCs, and unrouted research findings — as a real, checkable corpus instead of conversation history
and an unstructured document. `milestone` is the first record type this project has added
beyond ADR/RFC/Spec, and exercises RFC-0004's generic layering claim directly: if the mechanism
built for three types doesn't hold for a fourth, that's real evidence, not a hypothetical.

## Schema

```toml
[record_types.milestone]
dir = "docs/milestones"
required_fields = ["Status", "Phase", "Track"]
```

| Field | Values | Notes |
|---|---|---|
| `Status` | `Planned` \| `InProgress` \| `Blocked` \| `Done` | The realization axis for a milestone — orthogonal to whatever `Embodiment` on the RFC/ADR it implements is doing. |
| `Phase` | a plain tag (`0`, `1`, ...) | Sequential grouping. Not a resolved pointer — no cross-referencing, just a bucket. |
| `Track` | a plain tag (`header-format`, `section-checks`, ...) | Parallel workstream. Also a plain tag, not a pointer. |
| `Implements` | comma-separated, optional | Already-generic field (RFC-0001) — points at whichever RFC(s)/ADR(s) this milestone realizes. **Not required**: a milestone can exist before a decision does (e.g. "decide whether this needs an RFC"). |

History is the existing revision-log model (RFC-0006), reused unmodified: a milestone's `Status`
transitions are recorded as dated, classified revision-log entries on the milestone record itself,
not a separate mechanism.

## Why no new fields for ordering or grouping

A record's display number is identity, not priority (ADR-0003) — `Milestone-0007` existing does not
mean it happens after `Milestone-0006`, the same as every other record type. `Phase`/`Track` are
plain tags rather than resolved relationship fields because they're categorization, not references
to another record — nothing to check for resolution, no cycle risk, no new rule needed.

## What's deliberately not built

- **`Depends-on`/`Blocked-by`** — a real sequencing relationship (this milestone can't start until
  that one finishes) would need a new checked field and eventually cycle detection. Not built until
  a real case demands it (ADR-0034) — `Phase`/`Track` tags are sufficient for the backlog as it
  exists today.
- **A generated roadmap view** — an aggregate report over all milestones, grouped by `Phase` then
  `Track`. Deliberately not a hand-maintained second document (the same drift risk RFC-0017 already
  solved for template/config agreement) — build once there are enough milestones that
  `urzua check docs/milestones/` stops being a sufficient way to read the backlog.
- **Cross-type cycle prevention and per-profile "allowed parent types"** — named as open questions
  on RFC-0004 itself (this milestone type is what surfaced the gap), not scoped to milestone
  specifically since they'd apply to every record type's `Implements`/`Derives-from` use.

## References

- ADR-0034 — the decision this spec details: `milestone` as a configured type, zero `urzua-core`
  changes.
- ADR-0011 — the `waiver`-as-configured-type precedent this follows.
- ADR-0003 — display-number-is-identity, the reason `Phase`/`Track` are tags, not numbering.
- RFC-0004 — the layering model this record type is the first real test of beyond three types.
- RFC-0006 — the revision-log model reused for milestone history.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec. | **structural** |
