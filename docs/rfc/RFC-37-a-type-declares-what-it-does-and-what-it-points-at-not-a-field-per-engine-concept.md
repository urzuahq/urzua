---
Stable-Id: 01M2YMW302CE074VNQFXYK1H52
Status: Draft
Date: 2026-09-20
Author: beauwilliams
---
# 37 — A type declares what it does and what it points at, not a field per engine concept

## Summary

`RecordTypeConfig` has grown one field per engine behaviour -- nine today, and the next two are
already on the plan. Each new capability the engine gains becomes another optional key, which is the
same defect as hardcoding a type name, one level up: instead of the engine knowing a corpus is
spelled `waiver`, it knows the *concept* `waiver` and carries a schema field for it. Propose two
general axes -- **what a type does** (`capabilities`) and **what a type points at** (`relations`) --
so a new engine capability is a declared value rather than a new key.

## Motivation

`RFC-33`'s thesis is that the engine ships general-purpose primitives and a repository's governance
is a configuration of them. The rules table now follows it. The type schema does not.

Nine type-level fields today:

| field | what it declares |
|---|---|
| `dir` | where the type's records live |
| `required_fields` | which fields must be present |
| `header_shape`, `header_layout` | how the header is written |
| `prefix` | the filename/ID prefix |
| `known_fields`, `pointer_fields`, `narrative_fields` | which fields exist and of what kind |
| `spec` | the spec record governing this type |

`MILE-98` already dissolves five of them -- `header_shape` and `header_layout` into `fields.from`,
`prefix` into `identity.pattern`, the three field lists into the `fields` layer -- and its argument
for doing so is that the change is subtractive. That half has a plan.

**The remainder does not.** After `MILE-98` the document model accounts for `dir`, `identity`,
`fields` and `sections`. `spec` is left over, and it is governance rather than structure:

```rust
// type_no_declared_spec
message: format!(
    "record type '{type_name}' has no declared spec -- add `spec = \"SPEC-N\"` once one exists,
     or leave undeclared if ADR-41's editorial judgment says one isn't warranted"
)
```

The engine carries a schema field, a rule, and a finding message that names this repository's own
identifier shape and cites this repository's own ADR. It is `BUG-59` in a different costume: there
the engine knew a corpus's type was called `waiver`, here it knows a corpus has a thing called a
*spec* and that `ADR-41` governs when one is warranted.

**And the count is about to grow.** `BUG-74` needs the engine to know which type carries suppression
records without knowing it is called `waiver`; the obvious fix is a tenth field. `MILE-55` -- roles
and self-acknowledgement -- will need to know which type carries a role, which is an eleventh. One
instance is a field. Two is a pattern, and the second is already on the plan with a milestone number.

`MILE-98`'s own argument applies here unchanged: each of these is fixable today as another key, and
that is precisely the mistake, because under a general axis they are declarations. **Fixing them
first means fixing them twice.**

## Proposal

Two axes on a record type. Both are maps of declared values, so a new engine capability adds a
recognised *value*, never a struct field.

```yaml
record_types:
  exception:
    dir: "governance/exceptions"
    capabilities: ["suppresses-findings"]

  adr:
    dir: "docs/adr"
    relations:
      governed-by: "SPEC-3"
```

**`capabilities`** -- what a type *does* in the engine. The engine recognises a closed set of
capability names and asks "which type declared this capability", never "is this type called X".
Evidenced twice: suppression (`BUG-74`, `ADR-11`) and roles (`MILE-55`).

**`relations`** -- what a type *points at*, as a typed link to another record. `spec` becomes
`relations: {governed-by: SPEC-3}`, and `type.no-declared-spec` becomes a general rule taking the
relation name as a declared option: *a type must declare the `governed-by` relation*. The rule stops
naming specs, the message stops citing `ADR-41`, and a corpus that governs its types with something
other than a spec can say so.

This is subtractive in the same way `MILE-98` is: `spec` and `type.no-declared-spec`'s hardcoded
vocabulary both resolve into it rather than being rebuilt.

## Open questions

- **Is `relations` justified by one instance?** `spec` is the only current example, and a general
  mechanism built for a single case is speculative -- the objection `RFC-33` itself raises against
  designing presets before a second preset is asked for. `capabilities` has two instances and is on
  firmer ground. It may be right to build `capabilities` now and leave `spec` alone until a second
  relation exists.
- **Is the capability set closed or open?** A closed set the engine recognises keeps the error
  message useful (an unknown capability is a typo, and a silently-ignored declaration is a check that
  never fires -- `ADR-55`). An open set lets an adopter declare capabilities the engine has no
  behaviour for, which is worse than useless.
- **Does `required_fields` belong on either axis?** It is arguably policy rather than structure, but
  it predates all of this and nothing currently forces the question.
- **What happens to `dir`?** It survives both `MILE-98` and this proposal as the one genuinely
  structural key that is neither identity, fields, sections, capability nor relation. That may
  indicate a third category rather than an exception.
- **Migration.** Both axes are additive, so a v2 config keeps loading. Whether `spec` is accepted as
  a deprecated alias or removed at a schema bump is undecided, and `MILE-98` will force a schema
  conversation anyway.

## Non-goals

Redesigning the document model. `identity`, `fields` and `sections` are `MILE-98`'s subject and this
proposal assumes them rather than revisiting them.

Deciding `BUG-74`'s or `MILE-55`'s behaviour. This is about where a declaration lives, not what
suppression or roles do.

Widening what the engine can do. Both axes are notation for capabilities the engine already has or
has already decided to build.

## References

- `RFC-33`, whose thesis this extends from the rules table to the type schema.
- `MILE-98`, which dissolves five of the nine fields and makes the same "fixing them first means
  fixing them twice" argument.
- `BUG-59` and `BUG-74`, the engine knowing a corpus's vocabulary; `ADR-11`, which decided a waiver
  is a record and explicitly required "no special-casing in the tool".
- `MILE-55`, the second capability, already on the plan.
- `ADR-55`, on a declaration the engine silently ignores being a check that never fires.
