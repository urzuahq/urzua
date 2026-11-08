---
Status: Done
Stable-Id: 01M1Y5JGVDEQ3FGKG31KW7DPX1
Phase: '0'
Track: roadmap-tracking
Implements: ADR-35
Blocked-on: —
Realized-by: code:.urzua/config.yaml#record_types::bug@sha256:db5d407b1b71173ea2819d4b2213de057659bb319a8cffc32edee485a73fe44c, spec:docs/specs/SPEC-9-the-bug-record-type.md, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#a_bug_needs_found_in_and_a_what_was_wrong_section@sha256:83080eb2059b6c6eb69353b0a8730fa705e8a160c25134f84c7c16c18ee206bf
---
# 9 — Add the bug record type

## What

A configured record type requiring a named regression test, distinct from milestone's prospective planning fields.

## Why

A milestone's schema doesn't fit a retrospective defect well -- the one real governance gain (a required, checkable pointer to the proving test) needed its own type.

## Done means

- A `bug` record type is configured in `.urzua/config.yaml` with `Status` and `Found-in` required. — evidence: code:.urzua/config.yaml#record_types::bug@sha256:db5d407b1b71173ea2819d4b2213de057659bb319a8cffc32edee485a73fe44c
- The `bug` type requires `Regression-test`. — waived: superseded-by-decision (ADR-84) -- `ADR-84` retired the `Regression-test` field: its values moved into `Realized-by` as `test:` locators and a bug's `Status` now follows that evidence (`SPEC-9` 0.8)
- `SPEC-9` documents it. — evidence: spec:docs/specs/SPEC-9-the-bug-record-type.md
- `urzua check` examines every `docs/bugs/` record under it. — evidence: code:.urzua/config.yaml#record_types::bug@sha256:db5d407b1b71173ea2819d4b2213de057659bb319a8cffc32edee485a73fe44c, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#a_bug_needs_found_in_and_a_what_was_wrong_section@sha256:83080eb2059b6c6eb69353b0a8730fa705e8a160c25134f84c7c16c18ee206bf
- Adding it needed no change to urzua-core. — waived: process-outcome -- a property of the change that added the type (commit 784fa758: a config declaration, a template and a spec, with two unrelated core defects fixed beside it), which no file can evidence now

> **Revision log**
>
> | Date | Change | Class |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> |---|---|---|
| 2026-09-07 | Initial record: already complete at creation time. | structural |
| 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml#record_types::bug` (lines 370-435) 8e624908f1e8 -> db5d407b1b71. Reason: Read the diff of each pinned span since the old pin and the claim still holds. config.yaml#record_types::bug: the type's revision_log was expanded from the inline marker and required pair to also declare classes, class_column, date_column and tool_class (the class vocabulary of ADR-94); this change added Last-audited to the type's known_fields, the rest of the declaration is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
