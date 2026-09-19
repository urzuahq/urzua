---
Status: Planned
Stable-Id: 01M2RX3Z8638CY115J02WTAWQ6
Phase: '0'
Track: schema-governance
Implements: RFC-33
Blocked-on: —
---
# 98 — Build the declared document model: identity, fields, sections

## What

The layer `RFC-33` calls layer 1, and `ADR-53` decided: a record type declares **how its parts are
found**, instead of the engine assuming.

Expressing the `npryce/adr-tools` corpus -- the one `MILE-51` could not read -- rather than this
repository's own, since being able to write someone else's conventions is the point:

```yaml
record_types:
  adr:
    dir: doc/adr
    identity: {from: filename, pattern: '^(?P<number>\d+)-(?P<slug>.+)$'}
    fields:   {from: prefix-lines}
    sections: {from: headings, depth: 2, items: true}
```

This repository's own would differ in exactly two values -- `identity.pattern` reads
`^ADR-(?P<number>\d+)-(?P<slug>.+)$` and `fields.from` is `yaml-frontmatter` -- which is the claim
being tested: one config mechanism, two corpora, no code change.

## Why it is filed now

It had no milestone. `MILE-4` was the closest thing and was marked *absorbed into `RFC-33`'s declared
document model* on 2026-09-17, which moved the work into an RFC and left nothing on the plan tracking
it. Six of the ten open Phase 0 milestones are blocked on it, and it is the last blocker on the
founding claim -- so the most consequential item in Phase 0 was the one item not in Phase 0.

## What it unblocks, and what it deletes

Blocked on this: `MILE-4`, `MILE-5`, `MILE-6`, `MILE-7`, `MILE-55`, `MILE-97`.

**And six bugs, deliberately.** `BUG-22`, `BUG-40`, `BUG-41`, `BUG-50`, `BUG-51`, `BUG-52` are one
family: *the engine knows something and never compares it to what the corpus says about it*. Every one
is fixable today as another hardcoded comparison, and that is precisely the mistake --
`PLACEHOLDER_TOKENS` was transcribed from this project's own templates by hand, the companion-pair
check knows one pair, the revision-log rule keys on a literal string. **Each is a comparison written as
a constant.** Under a declared model they are declarations, so fixing them first means fixing them
twice.

It is also subtractive, which is the argument for doing it rather than working around it:

- `header_shape`'s three-value enum becomes one case of `fields.from`.
- `ADR-50`'s `header_shape = "none"` -- Accepted and unbuilt -- becomes another case rather than a
  fourth enum variant.
- **`header.deprecated-shape` becomes unwritable**, because no value is blessed to deprecate against.
- `RFC-29` and `RFC-31` dissolve into `identity`.

## The evidence for each primitive

Not designed in the abstract. Each part is wanted by something already measured:

| primitive | evidence |
|---|---|
| `fields.from: prefix-lines` | `MILE-51`'s re-run: all nine `adr-tools` records still report *"no header-shaped region found"*, because a Nygard record keeps its metadata in a bare `Date:` line and there is no way to say so |
| `sections` with `depth` | the MADR paper test: checking that each considered option has a pros-and-cons subsection needs `###`, and `sections.from: h2` is flat |
| `sections` with `items` | the same test: consequence bullets must read `Good, because` / `Bad, because` |
| `identity.pattern` | `BUG-37`: `urzua new` writes `ADR-4-slug.md` into a corpus whose own convention is `0004-slug.md`. Recognising both shapes is not writing the one a corpus uses |

**18 of this repository's own 84 ADR/RFC records use `###` headings that `check` cannot see at all.**

## Done means

1. A record type's identity, fields and sections are declared, and the engine reads no shape it was
   not told about.
2. `MILE-51`'s corpus passes with a hand-written config and no code change specific to it -- the
   founding claim, tested rather than asserted.
3. This repository's own `###` headings are visible to the section rules.
4. `header.deprecated-shape` is deleted, not ported.

## Deliberately not here

The rule vocabulary. `ADR-53` decided the principle and the document model and **left the nine
functions open** pending a third corpus (`MILE-96`). This milestone is the layer beneath that
question, and does not answer it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-18 | Filed. **Why:** layer 1 had no milestone. `MILE-4` was marked absorbed into `RFC-33` and the work moved into an RFC, so the item that blocks six of ten open Phase 0 milestones, and the founding claim, was not on the plan. Filed with the evidence for each primitive rather than as a design sketch, since every part of it is now wanted by something measured. | **substantive** |
> | 2026-09-19 | Six bugs recorded as deferred behind this. **Why:** `BUG-22`, `40`, `41`, `50`, `51` and `52` are one family and each is cheap to fix today as a hardcoded comparison -- which is the mistake the family consists of. Naming them here because a bug has no `Blocked-on` field, so this is the side of the dependency that can hold it. | **substantive** |
