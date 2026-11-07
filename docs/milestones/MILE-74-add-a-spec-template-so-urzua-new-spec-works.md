---
Status: Done
Stable-Id: 01M1YSM7BNRXXTYMV4TF93EAG4
Phase: '0'
Track: roadmap-tracking
Implements: SPEC-1
Blocked-on: —
Realized-by: code:.urzua/templates/spec.md, test:rust/crates/urzua-cli/tests/own_config_integration.rs#new_opens_a_spec_title_with_its_declared_prefix_and_other_types_with_the_bare_number@sha256:0680736422297fcaf5d3546bba11345d9e102a325d1c28b438954a204af48876, test:rust/crates/urzua-cli/tests/new_integration.rs#new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present@sha256:abbf083803b8a4fe62e6e7b933ffdec9e9964180b452f0a00334eaca302cfbd0, code:.urzua/config.yaml, code:rust/crates/urzua-core/src/new_record.rs#render_synthetic_yaml@sha256:cc95937d293de5fd0ce20d95ebd31f9ac39da1c34372ac523d37d51b14c7f356, test:rust/crates/urzua-cli/tests/own_config_integration.rs#a_spec_scaffolded_by_new_passes_check_without_an_edit_observed_failing@sha256:25baa15c9d28ec6c4ed061da303c4232955d33f15daafdf4ded2d86a1e51d51e, test:rust/crates/urzua-cli/tests/new_integration.rs#new_fills_the_declared_title_and_starting_version_fields_observed_failing@sha256:1f499bfa082fd14a7071a6cc736cb71784ba883aea0049923ddc340e3d43e263
Non-reference-ids: SPEC-7
---
# 74 — Add a spec template so urzua new spec works

## What

Add `.urzua/templates/spec.md` (or declare `header_shape = "yaml-frontmatter"` on `spec` in
`.urzua/config.toml`) so `urzua new spec "..."` actually works. `adr`, `rfc`, `bug`, and `milestone`
all have a template; `spec` has neither a template nor a synthesizable shape declared, so `urzua new
spec` fails outright with exit 2, "no template ... and no synthesizable shape declared." Every one
of this repo's specs (SPEC-1 through SPEC-6) was hand-authored -- never created through the tool.

Building the template forces a decision this repo's own corpus has ducked so far: the canonical
field set for `spec`. SPEC-1 alone carries `Embodiment`/`Author`/`Derives-from`; SPEC-2 through
SPEC-6 have none of those, just `Version`/`Date`/`Status`/`Implements`/`Parent` -- neither
`header.required-fields` (only checks for *missing* required fields; `Author` was never required
for `spec`) nor `header.layout-consistency` (checks arrangement, not which fields exist) catches
this. Decide whether `Author` becomes a required `spec` field (the same accountability argument
MILE-78 already made for ADR/RFC) and whether SPEC-1's `Embodiment`/`Derives-from` are worth keeping
project-wide or are SPEC-1-specific baggage from being hand-authored first, before this project's
own `spec` config existed.

## Why

Found live, immediately, trying to scaffold SPEC-7 with `urzua new spec` and hitting the exit-2
error. This is a real gap in bootstrapping the tool with itself: `milestone` and `bug` were both
built specifically to be dogfooded this way, but `spec` -- one of the three original record types
this project is built around -- can't be created through `urzua new` at all.

## Done means

- `.urzua/templates/spec.md` exists. — evidence: code:.urzua/templates/spec.md
- `urzua new spec` writes a record rather than failing for want of a template. — evidence: test:rust/crates/urzua-cli/tests/own_config_integration.rs#new_opens_a_spec_title_with_its_declared_prefix_and_other_types_with_the_bare_number@sha256:0680736422297fcaf5d3546bba11345d9e102a325d1c28b438954a204af48876, test:rust/crates/urzua-cli/tests/new_integration.rs#new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present@sha256:abbf083803b8a4fe62e6e7b933ffdec9e9964180b452f0a00334eaca302cfbd0
- The record `urzua new spec` writes passes `check`. — evidence: code:rust/crates/urzua-core/src/new_record.rs#render_synthetic_yaml@sha256:cc95937d293de5fd0ce20d95ebd31f9ac39da1c34372ac523d37d51b14c7f356, test:rust/crates/urzua-cli/tests/own_config_integration.rs#a_spec_scaffolded_by_new_passes_check_without_an_edit_observed_failing@sha256:25baa15c9d28ec6c4ed061da303c4232955d33f15daafdf4ded2d86a1e51d51e, test:rust/crates/urzua-cli/tests/new_integration.rs#new_fills_the_declared_title_and_starting_version_fields_observed_failing@sha256:1f499bfa082fd14a7071a6cc736cb71784ba883aea0049923ddc340e3d43e263
- The canonical spec field set the template carries is decided, with `Author` required. — evidence: code:.urzua/config.yaml, code:.urzua/templates/spec.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | `Status: Done`. The mechanical failure was already resolved as a side effect of ADR-33's yaml-frontmatter migration (`spec`'s `header_shape` got declared, so `urzua new spec` stopped erroring). The real remaining scope -- the canonical field set -- decided: `Author` required (MILE-78's accountability argument, backfilled onto all 16 pre-existing specs lacking it); `Embodiment`/`Derives-from` a project-wide optional convention, not `SPEC-1`-specific baggage; `Implements`/`Parent` declared in `known_fields` since every spec already carries them. `.urzua/templates/spec.md` built so `urzua new spec` produces real section scaffolding, not a bare skeleton. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-05 | Added the `Non-reference-ids` header field declaring `SPEC-7`. **Why:** the text names this id as history or illustration, not as citations of a record; the declaration states that on the record itself, so a new unresolved citation in this file is reported, where a file-wide `citation.resolution` waiver had silenced every one (`BUG-190`, `ADR-75`). The field carries no decision; the milestone is unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | `Status` `Done` -> `InProgress`. Status set from its Done-means items (ADR-96, MILE-215): unresolved item 3. **Why:** the item has no evidence clause, and `DoneWithUnresolvedItems` forbids `Done` while an item is unresolved; `fix` never downgrades, so the status is set by hand to the computed value. | **structural** |
> | 2026-11-07 | `urzua new spec` now writes a record that passes `check`: the `spec` type declares `relation_fields.title: Subject` and `revision_log.version_bump.initial: "0.1"`, so `new` fills the `Subject` with the title and the `Version` with `0.1` instead of leaving both blank (`SPEC-3`, `SPEC-12`, `SPEC-18`). Both keys are optional and an undeclared config is unchanged. **Why:** the item was unresolved and the claim was false: the scaffold failed `field.quality` on a blank `Version` and `Subject` before anyone edited it. | **substantive** |
> | 2026-11-07 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (4 of 4 items resolved) computes to Implemented` -- stated `InProgress` disagreed with the computed value `Done`. | **structural** |
