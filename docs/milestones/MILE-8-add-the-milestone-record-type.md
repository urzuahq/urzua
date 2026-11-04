---
Status: Done
Stable-Id: 01M1Y5JGA8FSBE13XZS36KJR6K
Phase: '0'
Track: roadmap-tracking
Implements: ADR-34
Blocked-on: —
Realized-by: spec:docs/specs/SPEC-6-the-milestone-record-type.md, code:.urzua/config.yaml
---
# 8 — Add the milestone record type

## What

A configured record type (Status/Phase/Track/Implements) tracking this project's own backlog, zero urzua-core changes.

## Why

The backlog was accumulating in conversation history and an unstructured document with nowhere durable or checkable to live.

## Done means

A `milestone` record type is configured in `.urzua/config.yaml` with `Status`, `Phase` and `Track` required and `Implements` a pointer field, `SPEC-6` documents it, `urzua check` examines every `docs/milestones/` record under it, and adding it needed no change to urzua-core.

> **Revision log**
>
> | Date | Change | Class |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> |---|---|---|
| 2026-09-07 | Initial record: already complete at creation time. | structural |
