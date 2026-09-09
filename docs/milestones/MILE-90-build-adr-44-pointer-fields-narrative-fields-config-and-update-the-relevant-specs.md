---
Stable-Id: 01M21GTR4QWS6YKM00VEG16SAD
Status: Done
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
- **Three new checks (RFC-23/ADR-44's amendment), all Error severity**:
  `config.pointer-declaration-missing` (a configured type must declare **both**
  `pointer_fields` and `narrative_fields`, even as empty arrays -- declaring only one and omitting
  the other fires the same as omitting both), `config.pointer-field-not-known` (a
  `pointer_fields`/`narrative_fields` entry not also present in that type's
  `required_fields`/`known_fields`), `config.pointer-narrative-overlap` (a field declared in both
  lists for one type).
- `urzua graph` adds a `kind` field (`"pointer"` \| `"narrative"`) to every edge.
- **`.urzua/config.toml` declares `pointer_fields`/`narrative_fields` explicitly for all six of this
  repo's own types** -- required by ADR-44's own "no implicit default" decision, not optional
  follow-up. Concretely closes the `bug`-type gap found live discussing this milestone: `bug` gains
  `pointer_fields = ["Implements"]` and an explicit `narrative_fields = []` (with `Implements` also
  added to `bug`'s `known_fields`, per `config.pointer-field-not-known`'s own requirement) so a bug
  can legitimately point at the ADR that fixed it.
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
> | 2026-09-09 | Scope widened per RFC-23/ADR-44's amendment: three new validation checks, and the concrete `bug`/`Implements` config declaration that motivated closing the review gaps. | **substantive** |
> | 2026-09-09 | Shipped in full. `RecordTypeConfig` gained `pointer_fields`/`narrative_fields`; `pointer_resolution`/`header_pointer_field_clean`/`narrative_field_stale` (renamed from `blocked_on_stale`) read them per type via a shared `FieldKindSpec` capability table and `RelationKind` enum, replacing every hardcoded field-list array in `rules.rs`. All three new config-level rules built and tested. `urzua graph` gained `kind: RelationKind` on every edge, became config-driven, and now uses the same shared `build_normalized_index` helper -- closing `BUG-11` (a pre-existing normalization gap) and a separate pre-existing gap (`graph()` never included `Parent`) as a side effect of the same rewrite. All six types declare real `pointer_fields`/`narrative_fields` in `.urzua/config.toml`; `rfc`/`bug` gained `Implements` in `known_fields`. `SPEC-2`, `SPEC-13`, and all six type specs updated to match; README's `Feeds-into` example is now literally true, closing `BUG-8`. Two rule-id `const RULE_ID` extraction and `main.rs::run_check`'s two-hand-synced-lists collapse (folded in from the same codebase survey) also shipped. `cargo test`/`make ci` both pass against this repo's own corpus with zero new findings from the three new rules. `Status: Done`. | **substantive** |
