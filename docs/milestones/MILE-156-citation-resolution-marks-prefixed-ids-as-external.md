---
Stable-Id: 01M6JKNTXSX6T751TK8FXKS9DB
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/citation.rs, test:rust/crates/urzua-cli/tests/citation_resolution_integration.rs
---
# 156 — Citation.resolution marks prefixed ids as external

## What

Add a way for `citation.resolution` to mark certain prefixed ids as external references (for example a public standard number that shares a record prefix) so they are not resolved as records.

## Why

An external identifier such as `RFC-4122` collides with a record prefix `RFC` and is reported as an unresolvable record citation. There is no way to say the id is not ours, so the author must reword a correct citation to satisfy the checker. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A declared external-id pattern makes a matching citation skip resolution; a non-matching dangling id is still reported; documented with a fixture of a colliding standard number; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Built the engine side: `citation.resolution` takes an optional `external_ids` list (exact ids or inclusive numeric ranges under a declared record prefix, validated at load); a matching citation is counted out of scope, a non-matching dangling id is still reported; documented in `SPEC-2` and `SPEC-3` with fixture tests. Not yet exercised in this repository's own `.urzua/config.yaml`: that rule is not enabled here, and enabling it reports 44 citations of 11 ids across frozen records, of which only `RFC-822` and `RFC-4122` are external, so enabling it is a separate decision. `Status` is untouched. **Why:** the Done-when requires own-repo exercise, which needs that decision first. | **substantive** |
> | 2026-11-04 | Own-repo rollout: `citation.resolution` is enabled in this repository's `.urzua/config.yaml` at `warn`, with `external_ids: ["RFC-822", "RFC-4122"]` -- the only two real external ids among the 44 citations of 11 ids first reported. The rest were reworded where the text is a living record, and covered by narrow waivers where the id is a record that existed under an old numbering, or an illustration inside a frozen decision. `Status` is untouched. **Why:** the Done-when requires own-repo exercise, which the previous row left as a separate decision; the maintainer has now made it. | **substantive** |
> | 2026-11-04 | Follow-up: `citation.resolution` takes the optional `skip_revision_log` option, reading the type's declared `revision_log` identity as `citation.path-exists` does; enabled in this repository's config, which let the `MILE-151` row cite `WAIVER-0001` as it originally did (reworded in #221 against the append-only rule of `RFC-6`). `SPEC-3` 0.44 and `SPEC-2` 0.56. **Why:** a revision log is history and must not be edited to satisfy a citation check. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #223, #190: `citation.resolution` takes `external_ids`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-177`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
