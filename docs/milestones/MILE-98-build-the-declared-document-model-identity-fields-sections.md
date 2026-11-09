---
Status: Done
Stable-Id: 01M2RX3Z8638CY115J02WTAWQ6
Phase: '0'
Track: schema-governance
Implements: RFC-33
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/document_model_integration.rs#a_prefix_lines_corpus_gets_ids_and_fields_from_its_declaration@sha256:bfc21def9e04130a7a8398cbac2d34d434b031d96035b8f7c036eef85ae09c3e, test:rust/crates/urzua-cli/tests/document_model_integration.rs#the_same_corpus_without_a_declaration_has_no_identity_and_no_header@sha256:5fbede2171a6fcf995dc11f06d3deed6005587165f2b64774225a3cb2df1645e, test:rust/crates/urzua-cli/tests/section_rules_integration.rs#undeclared_rules_do_not_run@sha256:138b9fb43aeed924da842207f1d0a07b87aebaf8d698a723d2b5f5e5e829bf69, code:rust/crates/urzua-core/src/document_model.rs@sha256:d636211daf2d869aff5d670f3c4e0e8a7e63815b75034176b87bc294fd0158bc, code:rust/crates/urzua-core/src/section.rs@sha256:322a093ce29259dc5c3b89fb584e56f9b9c8709e04c73620ad267a2167217efd, code:rust/crates/urzua-core/src/rules/section.rs@sha256:e5014bfc7d4f175434468a0b821ff278f740831a1993f8761cf8dcbd8d23065a, test:rust/crates/urzua-cli/tests/document_model_integration.rs#a_bare_numbered_corpus_passes_a_hand_written_config@sha256:0124821678dbf7b153888887c027907761c89326a1ca6221adbb2b0b14401016, test:rust/crates/urzua-cli/tests/section_rules_integration.rs#this_repositorys_own_h3_headings_are_visible_to_the_section_rules@sha256:d1bfd71a411b84f8da990dd5a271288e997c1a1d0c7dca93878544000735b026, code:.urzua/config.yaml@sha256:21e9ab57a37acf5df4921e86bca7127e7e3881e648115b370e7357339549054d, code:rust/crates/urzua-core/src/schema_version.rs@sha256:125709bf173e507d301188c7a04ed37f834e8f155124c389e144366a53a6ff7d, spec:docs/specs/SPEC-2-urzua-check.md
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

- A record type's identity, fields and sections are declared, and the engine reads no shape it was not told about. — evidence: test:rust/crates/urzua-cli/tests/document_model_integration.rs#a_prefix_lines_corpus_gets_ids_and_fields_from_its_declaration@sha256:bfc21def9e04130a7a8398cbac2d34d434b031d96035b8f7c036eef85ae09c3e, test:rust/crates/urzua-cli/tests/document_model_integration.rs#the_same_corpus_without_a_declaration_has_no_identity_and_no_header@sha256:5fbede2171a6fcf995dc11f06d3deed6005587165f2b64774225a3cb2df1645e, test:rust/crates/urzua-cli/tests/section_rules_integration.rs#undeclared_rules_do_not_run@sha256:138b9fb43aeed924da842207f1d0a07b87aebaf8d698a723d2b5f5e5e829bf69, code:rust/crates/urzua-core/src/document_model.rs@sha256:d636211daf2d869aff5d670f3c4e0e8a7e63815b75034176b87bc294fd0158bc, code:rust/crates/urzua-core/src/section.rs@sha256:322a093ce29259dc5c3b89fb584e56f9b9c8709e04c73620ad267a2167217efd, code:rust/crates/urzua-core/src/rules/section.rs@sha256:e5014bfc7d4f175434468a0b821ff278f740831a1993f8761cf8dcbd8d23065a
- `MILE-51`'s corpus passes with a hand-written config and no code change specific to it -- the founding claim, tested rather than asserted. — evidence: test:rust/crates/urzua-cli/tests/document_model_integration.rs#a_bare_numbered_corpus_passes_a_hand_written_config@sha256:0124821678dbf7b153888887c027907761c89326a1ca6221adbb2b0b14401016
- This repository's own `###` headings are visible to the section rules. — evidence: test:rust/crates/urzua-cli/tests/section_rules_integration.rs#this_repositorys_own_h3_headings_are_visible_to_the_section_rules@sha256:d1bfd71a411b84f8da990dd5a271288e997c1a1d0c7dca93878544000735b026, code:.urzua/config.yaml@sha256:21e9ab57a37acf5df4921e86bca7127e7e3881e648115b370e7357339549054d
- `header.deprecated-shape` is deleted, not ported. — evidence: code:rust/crates/urzua-core/src/schema_version.rs@sha256:125709bf173e507d301188c7a04ed37f834e8f155124c389e144366a53a6ff7d, spec:docs/specs/SPEC-2-urzua-check.md

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
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 5 locators pinned, 5 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/document_model.rs@sha256:db268cd06472, rust/crates/urzua-core/src/section.rs@sha256:322a093ce292, rust/crates/urzua-core/src/rules/section.rs@sha256:f6257018129e, .urzua/config.yaml@sha256:151830e5926d, rust/crates/urzua-core/src/schema_version.rs@sha256:cfe6760770f1 Reason: Pinned the files as they are now after checking each against the record it evidences: declaration.rs and config.yaml still declare pointer_fields and narrative_fields per type (MILE-90), config.yaml still requires Subject on specs with checks.yml gating it (MILE-91), document_model.rs, both section.rs files, schema_version.rs and config.yaml still read the declared identity, fields and sections (MILE-98), .git-hooks/pre-push, the Makefile and checks.yml still run urzua check (MILE-99), waiver.rs, rules/waiver.rs and waiver_stale_integration.rs still implement the waiver type SPEC-10 describes, audit.rs and the own-config and check integration tests still implement SPEC-11, new.rs, new_record.rs and new_integration.rs still implement SPEC-12, explain.rs, both graph.rs files and the own-config test still implement SPEC-13, both migrate.rs files and check_integration.rs still implement SPEC-14, and doctor.rs and its tests still implement SPEC-15. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 151830e5926d -> 0b59c6dfc8b9. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. config.yaml: config.yaml gained Kind among the milestone type's known fields and a kind declaration (build and decision) under statuses.delivery.items (ADR-96 amendment of 2026-11-07), and every other declaration the record relies on is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0b59c6dfc8b9 -> 0a00038a6388. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: this change declares status.done-items-shape (error), status.waived-item-stale (warn) and embodiment.claim-renewal-unattested (warn) and declares relation.cross-type-disallowed, section.shape and diff.source-and-records-mixed as off; every other declaration is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0a00038a6388 -> 3d51c2fac8b7. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: only the comment above the three rules declared off changed, to say they report `not-enabled` exactly as an undeclared rule does; no declaration changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/section.rs` f6257018129e -> b6be4936ab7f. Reason: Read the diff of each re-pinned file against main. core/src/rules/section.rs: Notice struct literals gained detail: None; no behaviour change. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 3d51c2fac8b7 -> 653bb0f54ca6. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: the only change is a new optional pending list (two placeholder patterns) under record_types::milestone statuses.delivery.items; nothing else in the file changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/document_model.rs` db268cd06472 -> 6d5db988288a, `rust/crates/urzua-core/src/schema_version.rs` cfe6760770f1 -> 125709bf173e. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: document_model.rs: split RawRequiredSection::resolve into resolve_shape, resolve_aliases and resolve_under; validation errors are produced in the same order with the same text; schema_version.rs: split upgrade into refuse_omitted_header_shape, drop_header_layouts, enable_rule and version_2_rules; the refusals still run before any change is recorded and the change list keeps its order. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline and reverify JSON compared before and after, with and without --as-of). **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/section.rs` b6be4936ab7f -> e5014bfc7d4f. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: section.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 653bb0f54ca6 -> 594d115541e3. Reason: config.yaml: read the diff since the previous pin; the only change is one added line, outside_writes: shared, under the harness key (this repository declares the new harness.outside_writes option, ADR-89 amendment of 2026-11-09). No record type, rule, status, relation or other declaration this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 594d115541e3 -> 2f0e023ed5a5. Reason: Read the diff of each pinned file or span since the old pin against what this record evidences. .urzua/config.yaml: embodiment.claim-expired gains `drift: {code: block, test: block, spec: acknowledge}` and its explanatory comment; nothing else in the file changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 2f0e023ed5a5 -> cbb1c67c40a0. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: read the diff since the old pin; the only change is the added key authority.decider_table: base with its two-line comment (this repository declares the new decider_table option, ADR-89 amendment of 2026-11-09). No record type, rule, status, relation or other declaration this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/document_model.rs` 6d5db988288a -> d636211daf2d. Reason: Read the diff against the pin. document_model.rs: CompiledIdentity::identify strips the .md extension through record::strip_md_extension (any case) instead of strip_suffix(".md"); lowercase names behave as before. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` cbb1c67c40a0 -> 21e9ab57a37a. Reason: Read the diff of .urzua/config.yaml: the authority section gains waivable: never and a two-line comment; no other key changed. The pinned evidence still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
