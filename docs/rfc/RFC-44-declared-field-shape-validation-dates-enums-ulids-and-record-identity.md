---
Stable-Id: 01M368EHQFV7N8SBF13G98EE93
Status: Draft
Date: 2026-09-23
Author: beauwilliams
---
# 44 — declared field shape validation: dates, enums, ULIDs, and record identity

## Summary

The document model has no type system. Every field this engine reads — `Status`, `Date`,
`Stable-Id`, a record's own identity in its H1 — is treated as an opaque string: present or absent,
blank or not, but never checked against what kind of value it's supposed to hold. This proposes
fixing that at the model level: every declared field carries a **type**, and the engine validates a
record's value against it, the same way `required_fields`/`known_fields` already declare which
fields exist. A record's identity (filename number ↔ H1 title, `BUG-32`/`BUG-123`) is one instance of
a typed field, not a special case — it is the record's own `identity` field, and it gets a real type
like every other one.

## Motivation

Concrete, load-bearing gaps, not a hypothetical:

- **No enum validation.** Nothing validates `Status` against a declared set of values for any record
  type. `SPEC-2`'s own "Not yet built" section has said this since before this session.
- **No identity-shape validation.** `BUG-32`: the H1 number is extracted by deleting every non-digit
  character from the first token, so `SPEC-18`, `ADR-18`, a stray `MILE-18`, or `# 2026-09-16 Notes`
  (→ `20260916`) all parse as *some* number and none are rejected for having the wrong shape.
- **No prefix validation.** `BUG-123`: `spec`'s H1s carry a `SPEC-N` prefix no other type's H1s carry,
  and nothing checks the prefix at all — only the number, once smashed into a bare digit string.
- **No date-shape validation.** `Date` fields are read as strings everywhere; a malformed date
  (`2026-9-5`, `09/05/2026`, a typo) passes every existing rule silently.
- **No ULID-shape validation.** `Stable-Id` is assigned as a ULID by `urzua new`, but nothing checks
  a hand-edited or hand-written one is actually shaped like one.

Each of these was previously going to be patched independently — an H1-prefix fix here, an
enum-validation rule there. That repeats this release's own recurring mistake: `field.pending` split
from `field.quality` (`BUG-38`), the companion-pair check hardcoded to one pair (`BUG-41`), the
revision-log rule keyed to a literal marker (`BUG-50`) — each a hand-written comparison for one field,
when the actual gap is one level up: **the document model was never given a type system**, so every
field's validity is reinvented per rule instead of declared once per field.

## Proposal

Every field a record type declares — in `required_fields`, `known_fields`, or the record's own
identity — carries a **field type**, declared in config:

```yaml
record_types:
  bug:
    dir: "docs/bugs"
    identity: { prefix: "BUG" }
    required_fields:
      Status: { type: enum, values: [Open, Fixed, WontFix, Rejected] }
      Found-in: { type: string }
    known_fields:
      Stable-Id: { type: ulid }
      Date: { type: date }
```

- **`identity`**: the record's own number/prefix shape — `filename.title-consistency` validates the
  filename's number, the H1's prefix and number, and the `identity.prefix` declaration all agree, in
  one place, instead of `first_h1`'s current all-digits extraction. This is what closes `BUG-32` and
  `BUG-123` — not as a special case, but as the `identity` field's own type.
- **`enum`**: a fixed value set, replacing `SPEC-2`'s unbuilt "Status enum validation" with a real
  mechanism, generalized to any field, not hardcoded to `Status`.
- **`date`**: `YYYY-MM-DD`, rejecting a malformed date rather than reading it as an opaque string.
- **`ulid`**: the shape `urzua new` already assigns, checked rather than trusted.
- **`string`**: today's default — an opaque value, checked only for presence/blank/placeholder as
  `field.quality`/`field.pending` already do. Every field not given a type stays `string`, so an
  existing config needs no change until it opts a field into a stricter type (`ADR-53`).

One rule family reads the declared type and reports a shape mismatch, the same way
`header.field-case-mismatch` is the one place a case mismatch is diagnosed rather than every caller
inventing its own check.

## Open questions

- **Where this lives relative to `MILE-98`.** `MILE-98`'s declared document model already plans
  `identity`, `fields`, and `sections` as the three declared axes of a record type. This RFC's
  `identity` type and per-field `type:` declaration are that same layer — not an alternative to
  `MILE-98`, its actual content. Two paths: fold this RFC into `MILE-98` directly (one RFC, one
  implementation, no interim mechanism to later replace), or land this RFC's narrower field-typing
  piece first as a standalone step `MILE-98` later absorbs. Recommend the former — a second,
  hardcoded interim mechanism is exactly the kind of thing this project's own `ADR-54` warns against
  accreting.
- **Type set completeness.** `enum`, `date`, `ulid`, `string`, `identity` cover every gap named above.
  Whether more types are needed (a URL, a semver, a cross-reference format distinct from
  `pointer_fields`' existing clean-reference check) is unevidenced today — start with the five named,
  add more only against a real case (`AGENTS.md`'s "don't build speculative capability").
- **Severity and migration.** A field retyped from `string` to `enum`/`date`/`ulid` may immediately
  flag existing corpus values that were never validated before. Each retyping needs its own
  before/after `check` diff, the same discipline already used for every rule change this session —
  not assumed safe.

## Non-goals

- Does not build free-text content validation (a `Purpose` section actually making sense) — this is
  shape/type only, the same ceiling `SPEC-1` already names for structural-vs-content checking.
- Does not replace `pointer_fields`/`narrative_fields`'s existing reference-resolution mechanism —
  a field's *type* and whether it *resolves* to another record are different questions.
- Does not attempt every possible type on day one (see Open questions).

## References

- `MILE-98` — the declared document model this RFC's `identity`/field-type mechanism belongs inside,
  not beside.
- `BUG-32` — the H1 identity-shape defect this RFC's `identity` type closes.
- `BUG-123` — the generator/corpus H1-prefix disagreement, closed the same way.
- `SPEC-2` — already names "Status enum validation" as unbuilt; this RFC is that mechanism,
  generalized to any field and any type, not `Status` alone.
- `BUG-38`, `BUG-41`, `BUG-50` — the recurring "one hardcoded comparison per field" shape this RFC
  replaces with one declared, typed mechanism.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed as `Draft`. **Why:** started as a narrow H1-title-format fix (`BUG-123`); the user correctly pushed back that the real gap is the document model having no field type system at all, of which H1 identity is one instance among several (enums, dates, ULIDs) already named as unbuilt elsewhere (`SPEC-2`). Reframed before implementation started, not after. | **substantive** |
