---
Status: Done
Stable-Id: 01M2RX3Z8638CY115J02WTAWQ6
Phase: '0'
Track: schema-governance
Implements: RFC-33
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/document_model.rs, code:rust/crates/urzua-core/src/section.rs, code:rust/crates/urzua-core/src/rules/section.rs, test:rust/crates/urzua-cli/tests/document_model_integration.rs, test:rust/crates/urzua-cli/tests/section_rules_integration.rs, code:.urzua/config.yaml
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

**And seven bugs, deliberately.** `BUG-22`, `BUG-40`, `BUG-41`, `BUG-50`, `BUG-51`, `BUG-52`, `BUG-57` are one
family: *the engine knows something and never compares it to what the corpus says about it*. Every one
is fixable today as another hardcoded comparison, and that is precisely the mistake --
`PLACEHOLDER_TOKENS` was transcribed from this project's own templates by hand, the companion-pair
check knows one pair, the revision-log rule keys on a literal string, and `waiver.rs` reads `Rule`,
`Scope` and `Expires` as literals. **Each is a comparison written as a constant.** Under a declared model they are declarations, so fixing them first means fixing them
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

## Sits beside `RFC-35`

`RFC-35` asks what a record type's `dir` means -- *this directory* or *this subtree* -- which is the
same class of question as `identity`, `fields` and `sections`, and is currently answered by an
undeclared prefix rule with four heuristics stacked on it. Deciding it before or alongside this
milestone deletes those heuristics rather than carrying them into the declared model.

## Deliberately not here

The rule vocabulary. `ADR-53` decided the principle and the document model and **left the nine
functions open** pending a third corpus (`MILE-96`). This milestone is the layer beneath that
question, and does not answer it.

Remaining follow-up: `urzua new`, `migrate` and `init` still read only `header_shape`, not `fields`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-18 | Filed. **Why:** layer 1 had no milestone. `MILE-4` was marked absorbed into `RFC-33` and the work moved into an RFC, so the item that blocks six of ten open Phase 0 milestones, and the founding claim, was not on the plan. Filed with the evidence for each primitive rather than as a design sketch, since every part of it is now wanted by something measured. | **substantive** |
> | 2026-09-19 | Six bugs recorded as deferred behind this. **Why:** `BUG-22`, `40`, `41`, `50`, `51` and `52` are one family and each is cheap to fix today as a hardcoded comparison -- which is the mistake the family consists of. Naming them here because a bug has no `Blocked-on` field, so this is the side of the dependency that can hold it. | **substantive** |
> | 2026-09-19 | `RFC-35` recorded as adjacent. **Why:** *which records belong to a type* is the same class of question as how to find a record's identity, fields and sections, and is currently inferred rather than declared. Four heuristics have accreted around it; deciding it with this model deletes them instead of porting them. | **substantive** |
> | 2026-09-19 | `BUG-57` added to the deferred set. **Why:** same family -- the engine holds a `Regression-test` claim and the test itself and never compares them, as with locators (`BUG-49`), companion fields (`BUG-41`) and the rule list (`BUG-52`). | **structural** |
> | 2026-11-03 | First slice landed: the config declaration (`identity`, `fields`, `sections`, parsed and validated at load) and a `##` section parser with line numbers. **Why:** both are plain data the later rules need and neither needs a design choice `RFC-33` leaves open. No rule reads either yet; the `header_shape` fold-in, the `###`/items reading, `header.deprecated-shape` removal and the `MILE-51` corpus pass are still to do. | **substantive** |
> | 2026-11-04 | Second slice landed: a declared `identity` pattern, which must capture the type prefix (a prefix-less pattern fails the load; a bare `0001-a.md` gets no identity, `RFC-46` rejected), decides a record's identifier and its H1 title form, and `fields.from` decides how the header is read (`prefix-lines` added beside the existing shapes). Types without a declaration load as before. **Why:** the declaration of slice 1 was read by nothing; this makes it drive loading, which is what the `MILE-51` corpus pass needs. The `sections` reading, the `header_shape` fold-in, `header.deprecated-shape` removal and the corpus pass are still to do. | **substantive** |
> | 2026-11-04 | Evidence so far, no `Realized-by` yet. **Why:** the declared `identity`, `fields` and `sections` (`document_model.rs`, `config.rs`, `document_model_integration.rs`, merged in #153 and #162) and `section.required` reading them are built; the remaining Done means are not shown: `MILE-51`'s corpus passing with a hand-written config and no code specific to it, and this repository's own `###` headings being visible to the section rules. `header.deprecated-shape` was deleted with `MILE-85`. `Status` stays Planned. | **substantive** |
> | 2026-11-04 | `Status` Planned -> Done; `Realized-by` added. `sections.depth` now drives the section reading: a type declaring `depth: 2` has its `###` headings parsed as sections nested under their `##`, and a required section may declare `level` and `under` (`SPEC-3`), so `section.required` and `section.shape` see them; a `level` the `depth` does not reach fails the load, and a type declaring no `level` is read as before. The two Done means still open are met: this repository's `RFC-46` (`### Alternatives considered` under `## Proposal`) is read by the section rules in `section_rules_integration.rs`, and the corpus `MILE-51` could not read passes `urzua check` with 0 findings and no rule that ran on nothing under a hand-written config (`fields: {from: prefix-lines}`, `sections.required` of its four headings), run by hand on 2026-11-04 and held as a committed test on a corpus of the same shape (`document_model_integration.rs`). **Why:** every Done means is now shown. That corpus's bare `0001-slug.md` file names get no declared identity (`RFC-46` was rejected, `SPEC-3`), so the config leaves the `filename.*` identity rules off; the maintainer ruled on 2026-11-04 that this is legitimate configuration, since those rules are opt-in. `urzua new`, `migrate` and `init` still read only `header_shape`, not `fields`, as the body above says. The maintainer reviewing the pull request that carries this row is the human decision. | **substantive** |
> | 2026-11-07 | `Realized-by` gains `.urzua/config.yaml`. **Why:** Done means 3 asks for this repository's own `###` headings to be visible to the section rules, and no type here declared `sections.depth`, so `check` here could not see them (the earlier evidence was a fixture on `RFC-46`). The `adr` and `rfc` types, which hold the `###` headings (140 of them), now declare `depth: 2`; `urzua check` stays clean. | **structural** |
