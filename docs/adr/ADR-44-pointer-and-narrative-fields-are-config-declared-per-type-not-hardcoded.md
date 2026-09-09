---
Stable-Id: 01M21GQHRW1JGSVYTVMRD0R7HB
Status: Accepted
Date: 2026-09-08
Author: '@beauwilliams'
Deciders: '@beauwilliams'
Supersedes / Superseded-by: —
Derives-from: RFC-23
---
# 44 — Pointer and narrative fields are config-declared per type, not hardcoded

## Context

`pointer_resolution`, `blocked_on_stale`, and `header.pointer-field-clean` each hardcode a fixed
list of field names (`Implements`/`Derives-from`/`Parent`/`Blocked-on`) directly in `urzua-core`'s
Rust source, never read from `.urzua/config.toml`. This contradicts the engine's own pitch --
proven false live (`BUG-8`): the README's lineage section claims an org can add a field like
`Feeds-into` and have it just work, but `pointer_resolution` would never check a field outside its
hardcoded array, no matter what `known_fields` declares.

The gap is sharper than "make one list configurable." Two genuinely different kinds of pointer
field are already implicit in the code: plain resolving pointers (`Implements`/`Derives-from`/
`Parent` -- exist, resolve, nothing else checked) and staleness-aware, prose-tolerant pointers
(`Blocked-on` -- resolves if a reference is present, tolerates free text, and is separately checked
for the target's terminal status). Making the list configurable without naming this distinction
would leave `header.pointer-field-clean` unable to tell which behavior a newly-declared field
should get.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Leave hardcoded | No work | README's own claim stays false; an adopting org can't add relationship vocabulary without forking `urzua-core` |
| One config list, one kind | Simple | Can't express `Blocked-on`'s prose-tolerant, staleness-checked behavior alongside clean pointers |
| Two per-type lists (`pointer_fields`/`narrative_fields`), no default | Matches both real kinds; matches `header_layout`/`known_fields`'s own "declared, not inferred, undeclared means skip" precedent; no compatibility-shim cruft for a tool with no real external adopters yet | Every one of this repo's own six types needs an explicit declaration -- a real, visible migration, not free |

## Decision

In the context of an engine that already lets an org declare its own record types with zero code
changes, but silently doesn't extend that to relationship vocabulary, facing a README claim that
was false the moment it was checked, we decided: **add `pointer_fields` and `narrative_fields` as
per-type config concepts**, resolved and checked exactly as `pointer_resolution`/
`header.pointer-field-clean`/`blocked_on_stale` already do today, but reading the field list from
`.urzua/config.toml` instead of a hardcoded Rust array -- to make the engine's own "declared, not
hardcoded" pitch actually true for relationship fields, not just record types -- accepting that
every type in this repo's own corpus needs an explicit declaration with no implicit default,
since there is no real backward compatibility to preserve.

`narrative_fields` (not `blocking_fields`) names the mechanism -- tolerates narrative text
alongside an optional embedded reference -- not today's one use case, so the name stays honest if
reused for a future field that isn't about blocking at all.

## Reversibility

Cheap to reverse in principle (remove the config keys, fall back to the current hardcoded arrays),
but every type's config would need to lose its explicit declaration too, and any adopting org that
had started relying on custom `pointer_fields`/`narrative_fields` would lose that capability
outright. Cost is low today (zero real external adopters), but grows the longer this ships before
being reconsidered.

## Consequences

- `pointer_resolution`, `header.pointer-field-clean`, and `blocked_on_stale` each read their field
  list from config instead of a hardcoded array.
- `urzua graph` gains a `kind` field per edge (`"pointer"` or `"narrative"`), so a consumer can
  distinguish the two without inferring it from the field name.
- Every one of this repo's own six types (`adr`, `rfc`, `spec`, `milestone`, `bug`, `waiver`) must
  declare `pointer_fields`/`narrative_fields` explicitly in `.urzua/config.toml` -- an undeclared
  type gets zero fields of either kind checked, a real behavior change from today's implicit
  coverage, not just a config addition.
- `SPEC-2` (`check`)'s rule table and `SPEC-13` (`explain`/`graph`)'s `graph` output contract both
  need updating to describe the new, config-driven mechanism and the `kind` field. The six type
  specs (`SPEC-6`/`9`/`10`/`16`/`17`/`18`) each need their own `pointer_fields`/`narrative_fields`
  declaration added to their `Schema` sections, matching whatever this repo's own config ends up
  declaring.
- The README's lineage section's `Feeds-into` example becomes literally true once this ships,
  closing `BUG-8`.
- Raised, not resolved by this decision: whether other backward-compatibility shims exist elsewhere
  in `urzua-core` for the same reason this one did (imagined future adopters, not real ones) --
  scoped as a separate research task before any bug gets filed for it.

## Amendment (2026-09-09): three validation checks added; `narrative_fields` clarified

Review found four real gaps in this decision, resolved in full on RFC-23's own amendment (see
References). Consequence for this ADR:

- **Three new checks ship alongside the config-driven rules**, all Error severity, all part of this
  decision's own build, not deferred follow-up: `config.pointer-declaration-missing` (a type must
  declare **both** `pointer_fields` and `narrative_fields` explicitly, even as empty arrays --
  declaring only one and omitting the other must fail visibly, not read as "zero fields, on
  purpose"), `config.pointer-field-not-known` (a field named in either list must also appear in that
  type's `required_fields`/`known_fields` -- no automatic exception, checked explicitly), and
  `config.pointer-narrative-overlap` (a field cannot be declared in both lists for one type).
- **`narrative_fields` are staleness-checked by definition**, for any field in the list, not a
  `Blocked-on`-specific behavior incidentally reused. No change to the mechanism this ADR already
  decided -- `blocked_on_stale` generalizing to read its field list from config already implied
  this; this amendment only makes it explicit.
- `required_fields`/`known_fields` together remain the source of truth for "which fields a type's
  header may carry" -- membership in `pointer_fields`/`narrative_fields` was considered as an
  automatic exception to that and rejected, to keep one pair of config lists answering one question
  each rather than either silently extending the other.

## References

- RFC-23 -- the proposal this ADR decides; the design's own open questions and rejected
  alternatives (`blocking_fields` as a name, a global field list, a backward-compat default), and
  its own amendment resolving the four gaps this ADR's amendment reflects.
- BUG-8 -- the concrete defect (README overstatement) this decision closes.
- BUG-9 -- the separate audit this decision's Consequences names as "raised, not resolved"; filed
  in the same PR as this ADR, it found legacy pre-`ADR-36` filename support as the one other real
  instance of the same shim pattern.
- ADR-38/39/43 -- the "declared, not voted or inferred" precedent this decision extends to pointer
  fields.
- RFC-20 -- the CLI/command taxonomy RFC, whose own table asserts pointer fields are "plain field
  names an org declares" -- this decision is what makes that literally true.
- MILE-90 -- the milestone that builds this decision, including the three checks this amendment
  adds.
