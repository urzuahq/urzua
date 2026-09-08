---
Stable-Id: 01M21GTR4QWS6YKM00VEG16SAD
Status: Planned
Phase: '0'
Track: schema-governance
Implements: ADR-44
Blocked-on: —
---
# 90 — Build ADR-44: pointer_fields/narrative_fields config, and update the relevant specs

## What

The actual build for ADR-44's decision, in full -- not just the code:

- `RecordTypeConfig` gains `pointer_fields: Option<Vec<String>>` and
  `narrative_fields: Option<Vec<String>>`.
- `pointer_resolution`, `header_pointer_field_clean`, and `blocked_on_stale` each read their scanned
  field list from these instead of their current hardcoded arrays. Undeclared means zero fields of
  that kind examined, no default.
- `urzua graph` adds a `kind` field (`"pointer"` \| `"narrative"`) to every edge.
- **`.urzua/config.toml` declares `pointer_fields`/`narrative_fields` explicitly for all six of this
  repo's own types** -- required by ADR-44's own "no implicit default" decision, not optional
  follow-up.
- **Specs updated to match, not left describing the old mechanism**: `SPEC-2` (`check`)'s rule
  table, `SPEC-13` (`explain`/`graph`)'s `graph` output contract (the new `kind` field), and each of
  the six type specs (`SPEC-6`/`9`/`10`/`16`/`17`/`18`)'s own `Schema` section, declaring that
  type's real `pointer_fields`/`narrative_fields`.
- README's lineage section corrected to match reality -- closes `BUG-8`.

## Why

ADR-44 decided this; this milestone is the actual work, not a restatement of the decision. Named
explicitly because this project's own history (found live, repeatedly, this session) shows a
decision landing in an ADR/RFC without the specs that describe the corresponding mechanism getting
updated in the same pass -- e.g. `SPEC-18`'s Schema section went stale for a full revision after
its own config change already shipped (corrected only when caught live). Scoping "update the
relevant specs" into this milestone's own `What`, not as an assumed follow-up, is meant to prevent
exactly that recurrence.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial milestone. **Why:** ADR-44 decided the design; this tracks building it, explicitly including the spec updates the decision itself names as a real consequence. | **structural** |
