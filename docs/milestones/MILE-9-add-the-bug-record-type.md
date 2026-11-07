---
Status: Done
Stable-Id: 01M1Y5JGVDEQ3FGKG31KW7DPX1
Phase: '0'
Track: roadmap-tracking
Implements: ADR-35
Blocked-on: —
Realized-by: code:.urzua/config.yaml#record_types::bug@sha256:8e624908f1e8f7f825e213d1c5fd69723b0e81ca85f564b1a5a183f826101678, spec:docs/specs/SPEC-9-the-bug-record-type.md, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#a_bug_needs_found_in_and_a_what_was_wrong_section@sha256:83080eb2059b6c6eb69353b0a8730fa705e8a160c25134f84c7c16c18ee206bf
---
# 9 — Add the bug record type

## What

A configured record type requiring a named regression test, distinct from milestone's prospective planning fields.

## Why

A milestone's schema doesn't fit a retrospective defect well -- the one real governance gain (a required, checkable pointer to the proving test) needed its own type.

## Done means

- A `bug` record type is configured in `.urzua/config.yaml` with `Status` and `Found-in` required. — evidence: code:.urzua/config.yaml#record_types::bug@sha256:8e624908f1e8f7f825e213d1c5fd69723b0e81ca85f564b1a5a183f826101678
- The `bug` type requires `Regression-test`. — waived: superseded-by-decision (ADR-84) -- `ADR-84` retired the `Regression-test` field: its values moved into `Realized-by` as `test:` locators and a bug's `Status` now follows that evidence (`SPEC-9` 0.8)
- `SPEC-9` documents it. — evidence: spec:docs/specs/SPEC-9-the-bug-record-type.md
- `urzua check` examines every `docs/bugs/` record under it. — evidence: code:.urzua/config.yaml#record_types::bug@sha256:8e624908f1e8f7f825e213d1c5fd69723b0e81ca85f564b1a5a183f826101678, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#a_bug_needs_found_in_and_a_what_was_wrong_section@sha256:83080eb2059b6c6eb69353b0a8730fa705e8a160c25134f84c7c16c18ee206bf
- Adding it needed no change to urzua-core. — waived: process-outcome -- a property of the change that added the type (commit 784fa758: a config declaration, a template and a spec, with two unrelated core defects fixed beside it), which no file can evidence now

> **Revision log**
>
> | Date | Change | Class |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> |---|---|---|
| 2026-09-07 | Initial record: already complete at creation time. | structural |
