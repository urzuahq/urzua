---
Version: '0.13'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/status_model.rs, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs, test:rust/crates/urzua-cli/tests/status_delivery_integration.rs
Author: beauwilliams
Subject: 'The `milestone` record type -- this project''s own planned-work backlog as a checkable corpus.'
Implements: ADR-34
Parent: —
---
# SPEC-6 — The `milestone` record type

## Purpose

Tracks this project's own backlog — decided-but-unbuilt work, explicitly deferred work, undecided
RFCs, and unrouted research findings — as a real, checkable corpus instead of conversation history
and an unstructured document. `milestone` is the first record type this project has added
beyond ADR/RFC/Spec, and exercises RFC-4's generic layering claim directly: if the mechanism
built for three types doesn't hold for a fourth, that's real evidence, not a hypothetical.

## Schema

```yaml
record_types:
  milestone:
    dir: "docs/milestones"
    prefix: "MILE"
    required_fields: ["Status", "Phase", "Track"]
    header_shape: "yaml-frontmatter"
    known_fields: ["Stable-Id", "Implements", "Blocked-on", "Realized-by"]
    spec: "SPEC-6"
    pointer_fields: ["Implements"]
    narrative_fields: ["Blocked-on"]
```

`prefix` (2026-09-07 addition) decouples the type's own name — used for the `urzua new milestone
...` argument and required-fields lookup — from its filename/ID prefix, the same way `dir` already
decouples the type name from its directory name. Omitted, it defaults to the type name upper-cased
(`MILESTONE`); declared, `urzua new` emits `MILE-N-slug.md` and `check` recognizes that shape.
Per-record-type, not global — any type can shorten its own prefix without affecting the others.

| Field | Values | Notes |
|---|---|---|
| `Status` | `Planned` \| `InProgress` \| `Blocked` \| `Done` \| `WontDo` | The realization axis for a milestone — orthogonal to whatever `Embodiment` on the RFC/ADR it implements is doing. `WontDo` is terminal, for work deliberately deferred or decided against — distinct from `Blocked` (a specific, nameable blocker exists and the work is still intended) and from leaving a milestone `Planned` (which misrepresents a reversed decision as still-pending work). Precedent: `bug`'s `WontFix`, `adr`'s `Rejected`/`Superseded` — every other record type in this corpus already has a terminal "decided against" state; milestone didn't until this version. |
| `Phase` | a plain tag (`0`, `1`, ...) | Sequential grouping. Not a resolved pointer — no cross-referencing, just a bucket. |
| `Track` | a plain tag (`header-format`, `section-checks`, ...) | Parallel workstream. Also a plain tag, not a pointer. |
| `Implements` | comma-separated, optional | Already-generic field (RFC-1) — points at whichever RFC(s)/ADR(s) this milestone realizes. **Not required**: a milestone can exist before a decision does (e.g. "decide whether this needs an RFC"). Declared as this type's one `pointer_fields` entry (MILE-90/ADR-44) — clean, comma-separated references only, format-enforced by `header.pointer-field-clean`. |
| `Realized-by` | `spec:`, `code:`, `test:` locators; set by the delivering PR | Delivery evidence (`MILE-183`). The PR that fully delivers a milestone sets it and `Status: Done` together; `status.delivery-agreement` fails when the evidence computes to `Implemented` or `Verified` under `Planned`, `InProgress` or `Blocked`, and when `Done` has no evidence. A decision milestone's evidence is the record of the decision (a `spec:` locator), which is neither. Leave it off a milestone that is only partly delivered. |
| `Blocked-on` | free text, optionally citing a record ID; optional | What has to be true before this milestone can move. Free text with no reference token stays legal ("a decision not yet made"); a real ID (`BUG-3`) is checked by `pointer.resolution` for resolution and by `narrative-field.stale` (ADR-42, generalized by MILE-90/ADR-44) for whether the target has since reached a terminal status. Declared as this type's one `narrative_fields` entry (MILE-90/ADR-44) — prose-tolerant, unlike `Implements`. Was a body section (`## Blocked on`) before this version — moved into the header (ADR-42) since it's the one field-shaped exception in the schema that hadn't been. |

`Status` is documented here but not mechanically validated: nothing in `urzua-core` checks a
record's `Status` value against an enum for any record type today (RFC-9's field-presence rules
check that a required field is non-empty, not that its value is one of a closed set). Moving to
`WontDo` today is a config/template/documentation change only — declaring it here, and in
`.urzua/templates/milestone.md`, is what makes it the documented contract until a future milestone
adds real enum enforcement.

History is the existing revision-log model (RFC-6), reused unmodified: a milestone's `Status`
transitions are recorded as dated, classified revision-log entries on the milestone record itself,
not a separate mechanism.

## Required sections

Every milestone, open or closed, carries a `## Done means` section, declared under the type's `sections`
in `.urzua/config.yaml` and enforced at `error` by `section.required` (`SPEC-3`). It states the concrete,
checkable criteria that make the milestone done: the command, rule or file, and what it reports. An open
milestone derives them from its What and Why; a `Done` milestone states the criteria its delivery met, so
"done" is judged against them and not against the What, and a closure that does not meet its What is
reopened rather than left `Done`. Where it fits, the criteria include that the capability is exercised in
the repository's own configuration and records, not only by unit tests. A `WontDo` milestone is exempt
(`exempt_statuses`): it will never be done, so it has no criteria to meet. The template
(`.urzua/templates/milestone.md`) carries the section, and `/milestone` fills it.

## Phase vocabulary

`Phase` stays a plain tag with no mechanical check, but this project uses one fixed vocabulary so a
phase means the same thing on every record:

| Phase | Meaning |
|---|---|
| `0` | Bootstrap: required for the tool to validate and build itself further, plus urgent bugs and the single Phase 0 exit (`SPEC-1`, Exit bar): governing this repository in CI with zero standing warnings, and replacing one adopter's hand-written linter (`MILE-102`). |
| `1` | Worth installing. |
| `2` | Agent-facing. |
| `3` | Differentiated. |
| `4` | Scale. |

A milestone moves phase by an edit to `Phase` and a revision-log row saying why, the same as any other
substantive change. Another repository using this type may define its own phases; these are this
project's.

## Why no new fields for ordering or grouping

A record's display number is identity, not priority (ADR-3) — `MILE-7` existing does not
mean it happens after `MILE-6`, the same as every other record type. `Phase`/`Track` are
plain tags rather than resolved relationship fields because they're categorization, not references
to another record — nothing to check for resolution, no cycle risk, no new rule needed.

## What's deliberately not built

- **`Depends-on`/`Blocked-by`** — a real, bidirectional sequencing relationship (this milestone
  can't start until that one finishes) with reciprocity checking and eventual cycle detection. Not
  built until a real case demands it (ADR-34) — `Phase`/`Track` tags are sufficient for the backlog
  as it exists today. Distinct from `Blocked-on` (above): that field is one-way, optional, and only
  checked for resolution/staleness, never for reciprocity or cycles.
- **A generated roadmap view** — an aggregate report over all milestones, grouped by `Phase` then
  `Track`. Deliberately not a hand-maintained second document (the same drift risk RFC-17 already
  solved for template/config agreement) — build once there are enough milestones that
  `urzua check docs/milestones/` stops being a sufficient way to read the backlog.
- **Cross-type cycle prevention and per-profile "allowed parent types"** — named as open questions
  on RFC-4 itself (this milestone type is what surfaced the gap), not scoped to milestone
  specifically since they'd apply to every record type's `Implements`/`Derives-from` use.

## References

- ADR-34 — the decision this spec details: `milestone` as a configured type, zero `urzua-core`
  changes.
- ADR-11 — the `waiver`-as-configured-type precedent this follows.
- ADR-3 — display-number-is-identity, the reason `Phase`/`Track` are tags, not numbering.
- RFC-4 — the layering model this record type is the first real test of beyond three types.
- RFC-6 — the revision-log model reused for milestone history.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec. | **structural** |
> | 2026-09-07 | Added `prefix = "MILE"`, shortening the filename/ID prefix from `MILESTONE-N` to `MILE-N`. The type name (`milestone`) and directory (`docs/milestones`) are unchanged. | **substantive** |
> | 2026-09-07 | Added `WontDo` as a terminal `Status` value (ADR-34 amendment). **Why:** every other record type in this corpus (`bug`, `adr`) already has a terminal "decided against" state; `milestone` didn't, so deferring or abandoning a milestone had no honest representation — it either stayed `Planned` (misrepresenting a reversed decision as still-pending) or `Blocked` (implying a specific blocker that may not exist). Folded into this version rather than a new spec number, per ADR-14's amendment: a spec's number is permanent per subject, revisions are a version bump on the same spec. | **substantive** |
> | 2026-09-07 | Moved `Blocked-on` from a body section (`## Blocked on`) into the header, alongside `Implements` (ADR-42). **Why:** found live that `Blocked-on` was the one field-shaped exception across the entire schema left unchecked as prose -- MILE-2/3 both cited an already-fixed bug by description with nothing catching it going stale. All 84 existing milestone files migrated, verified by a before/after content-equality check. | **substantive** |
> | 2026-09-08 | Header shape changed from `blockquote` to `yaml-frontmatter`, `header_layout` removed (ADR-33/38 amendments); `spec = "SPEC-6"` declared in config, closing `type.no-declared-spec`'s live finding for this type (ADR-43). **Why:** ADR-33's migration scope widened to all six configured types the same day; `header_layout` has nothing left to distinguish once no type declares `Blockquote`. | **substantive** |
> | 2026-09-08 | Bumped to `0.5`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`; also corrected `Parent` from `SPEC-1` to `—` (`BUG-10`): this spec's real lineage is already stated via its own `Implements`/`Derives-from`, not a narrowing of `SPEC-1`'s v0-CLI scope. | **structural** |
> | 2026-09-09 | Declared `pointer_fields = ["Implements"]`/`narrative_fields = ["Blocked-on"]` in config (MILE-90/ADR-44), replacing the two rules' prior hardcoded field list; `blocked-on.stale` renamed `narrative-field.stale`. **Why:** `BUG-8` named this exact type's schema as an example of the genericity `pointer.resolution` claimed but didn't have. | **substantive** |
> | 2026-09-17 | This type's `[record_types.*]` declaration re-rendered in YAML (`ADR-52`, shipped in the same change). The same declaration, same keys, same values -- only the syntax differs. | **structural** |
> | 2026-11-02 | Added the "Phase vocabulary" section defining `0` bootstrap, `1` worth installing, `2` agent-facing, `3` differentiated, `4` scale; bumped to `0.8`. **Why:** `Phase` was documented only as "a plain tag (`0`, `1`, ...)", so the same number meant different things across records and nothing said where adopter-blocking work belongs; `0` now has a stated meaning, which `MILE-35` and the first-adopter milestones rely on. | **substantive** |
> | 2026-11-04 | Phase vocabulary: the `0` row now states the single Phase 0 exit (`SPEC-1`, Exit bar) instead of only "the first adopter replacement"; `Version` `0.8` -> `0.9`. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | Bumped to `0.10`. The type declares `Realized-by` (delivery evidence) and `statuses.delivery`; the delivering PR sets `Realized-by` and `Status: Done`. **Why:** `MILE-183`. | **substantive** |
> | 2026-11-04 | Added the "Required sections" section and bumped to `0.11`: every milestone carries `## Done means`, declared under `sections` and enforced by `section.required`, with `WontDo` exempt. **Why:** maintainer decision of 2026-11-04: "done" was being judged from the What, and the closure audit found closures that over-claimed; stating the criteria on every record lets a closure be checked against them. | **substantive** |
> | 2026-11-05 | Bumped to `0.12`. Added `Realized-by` naming the implementing code and the tests that exercise this spec: `a_milestone_must_state_its_done_means_unless_it_is_wontdo`, `a_done_milestone_needs_delivery_evidence_and_evidence_under_planned_is_a_disagreement`. **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`; the tests run this repository's own `.urzua/config.yaml`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-core/src/status_model.rs, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs, test:rust/crates/urzua-cli/tests/status_delivery_integration.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Status changed from Accepted to Shipped and bumped to `0.13`. **Why:** maintainer decision of 2026-11-05 to ship the specs whose implementation is verified; `Embodiment: Verified` with `Realized-by` naming the implementing code and tests. | **structural** |
