# 34 — `milestone`: a configured record type, not a built-in one

> Status: Accepted
> Embodiment: Not started
> Date: 2026-09-07
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-1 (Accepted), RFC-4 (Draft)

## Context

This project has accumulated a real backlog — decided-but-unbuilt work (RFC-17's section-parser,
ADR-33's header-shape conversion), explicitly deferred work (RFC-5's full claim graph, `fix`
Tier 2/3, AgDR export), undecided Draft RFCs, and ~30 features/findings from research
with no routing to any public record at all — with no durable, checkable place to track any of it.
It exists only in conversation history and an unstructured roadmap document.

The obvious question this project already answered once, for `waiver` (ADR-11): does tracking
this need new engine capability, or is it just another configured record type? Checked directly
against the actual code before deciding: `"adr"`/`"rfc"`/`"spec"` appear nowhere in `urzua-core`'s
real logic, only in test fixtures — both types are already 100% generic. `waiver` is the one
partial exception, with one hardcoded `record_type == "waiver"` filter, because it has genuine
mechanical behavior (suppressing other findings) nothing else needs. A milestone needs no
equivalent mechanism — it needs fields, relationships, and history, which is exactly what
`Implements`/`Derives-from` (already generic, already comma-separated-multi-value) and the
revision-log model (RFC-6, already generic) already provide for free.

Two things a milestone conceptually needs were checked against real prior art before assuming they
required new mechanism: sequential ordering, and grouping. A record's display number is identity,
not priority (ADR-3's existing distinction — `ADR-7` doesn't mean "happens after
`ADR-6`"), so numbering was never the right tool for ordering and isn't reached for here either.
Two comparable governance-linter implementations were checked directly for hierarchy/
grouping design and found to hardcode exactly three types with a single fixed directional pointer
and no generic relationship field at all — offering no prior art for a stronger mechanism, and if
anything confirming Urzua's existing generic-pointer design is already ahead of them.

## Decision

In the context of a real, accumulating backlog with nowhere durable to live, facing the same "new
mechanism or just configuration" question already answered for `waiver`, we decided: **`milestone`
is a plain configured record type — zero `urzua-core` changes.**

```toml
[record_types.milestone]
dir = "docs/milestones"
required_fields = ["Status", "Phase", "Track"]
```

- `Status`: `Planned | InProgress | Blocked | Done`.
- `Phase`: a plain tag (e.g. `0`, `1`) — sequential grouping, not a resolved pointer.
- `Track`: a plain tag (e.g. `header-format`, `section-checks`) — parallel workstream, not a
  resolved pointer.
- `Implements`: comma-separated, already generic — points at whichever RFC(s)/ADR(s) this milestone
  is actually realizing. Not required, since a milestone can exist before an RFC does (e.g.
  "decide whether this needs an RFC at all").
- History (state transitions over time) is the existing revision-log model (RFC-6), reused, not
  reinvented.

**Explicitly not built now, named as real follow-ups instead of assumed:**
- A `Depends-on`/`Blocked-by` relationship with cycle detection, for genuine sequencing beyond
  `Phase`/`Track` tags — build once a real case needs it, the same discipline as everything else on
  this project's backlog.
- A generated aggregate/pivot view ("the roadmap") over milestone records grouped by `Phase`/
  `Track` — deliberately not a hand-maintained second document (the same drift risk already solved
  for template/config agreement, RFC-17); build once there are enough milestones for a generated
  view to earn its cost over just running `urzua check docs/milestones/`.

## Reversibility

Fully additive: one config entry, one template, one new directory. No existing behavior changes.

## Consequences

- `docs/milestones/` becomes real, checkable corpus — `check`, `fix`, `explain`, `graph` apply to
  it exactly as they do to any other type, with zero special-casing.
- RFC-4's "RFC → ADR → Spec is the common path, not the only one" now has a fourth type
  exercising the same generic relationship mechanism, which is direct, real evidence for whether
  that mechanism actually holds up beyond the three types it was designed against.
- Milestone records with no `Implements` at all are valid and expected — several current backlog
  items (e.g. "decide whether X needs an RFC") don't have a decision to point at yet.

## Amendment (2026-09-07): a configurable, per-type filename prefix

`milestone`'s filename/ID prefix is shortened from `MILESTONE-N` to `MILE-N` (SPEC-6). This is not
special-cased to `milestone`: `RecordTypeConfig` gains an optional `prefix` field, defaulting to
the type name upper-cased when omitted, so every existing type's filenames are unaffected unless
its config declares otherwise. The type name (`milestone`, used for the `urzua new milestone ...`
argument and required-fields lookup) and the directory (`docs/milestones`) are both unchanged —
this decouples the filename prefix the same way `dir` already decouples the directory, and is a
per-record-type option, not a global one.

## References

- ADR-11 — the `waiver`-as-configured-type precedent this decision follows exactly.
- ADR-3 — the display-number-is-identity-not-priority distinction that rules out numbering as
  an ordering mechanism.
- RFC-4 — the layering model whose generic relationship mechanism this exercises.
- RFC-6 — the revision-log model reused for milestone history.
