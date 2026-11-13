---
Status: Done
Stable-Id: 01M2W3XAGYFHAGSQ8NQRP8KBZR
Phase: '1'
Track: section-checks
Implements: SPEC-1
Depends-on: MILE-98
Blocked-on: —
---
# 104 — Section structure rules: duplicates, order, and required presence

## What

Rules about a record's section *structure*, as opposed to its section *contents*:

- **A section heading appears twice.** Two `## Open questions` in one record is malformed under every
  convention; there is no corpus where it is intended.
- **Sections appear out of a declared order.** No type declares one today, and nothing in the schema
  can express it -- `required_fields` is a set, not a sequence. So this needs a source first: either
  the type's template, whose section order is already the de facto convention every record follows, or
  a new ordered declaration. The template is the better candidate, because it is what an author is
  looking at while writing.
- **A heading level is skipped** -- an `###` with no `##` above it. Defined relative to
  `sections.depth` (`MILE-98`), not absolutely: at `depth: 2` the top level is `##` and a skip is an
  `###` with no `##` above it, while at `depth: 3` the same `###` is an ordinary section. A type that
  declares a depth is stating where its section tree starts, and this rule reads the tree from there
  rather than assuming `##`.

## Not `MILE-4`

`MILE-4` is the policy half: *which* sections a type must have and what must be true of their
contents. This is the structural half: whether the document is well-formed at all, independent of
which sections a type wants.

The distinction matters because they fail differently. A missing required section is a governance
finding -- this corpus expects one and it is absent. A duplicated section is a *broken document*,
and no configuration makes it acceptable.

## Why it is filed

**No rule mentions sections at all.** Measured 2026-09-19: of the twenty-one rules shipping, none
examines section structure, so a record with two `## Open questions` and two `## References` is
well-formed as far as the engine is concerned.

That is not hypothetical. `SPEC-1` shipped in exactly that state during this project's own work --
a narrowing edit inserted new sections without removing the old ones, producing two `## Open questions`
that gave contradictory answers to the same question. `check` was green throughout; review caught it.

Three further record defects in the same period were section-shaped: a revision-log block deleted
along with the section containing it, a revision row appended to a file with no such block, and rows
left dangling after `## References` with no table header. `BUG-50` covers the first two. Nothing
covers the structure they sit in.

## Blocked on `MILE-98`

Sections are not addressable until the document model declares how to find them (`sections.from`, with
`depth` and `items`). This is one more thing waiting on that layer, alongside `MILE-4`, `MILE-5`,
`MILE-6`, `MILE-7`, `MILE-55`, `MILE-97` and six bugs.

## A note on what this does not fix

The mistakes that produced `SPEC-1`'s duplicate sections were *edits that looked applied and were
not*. A rule catches the result; it does not catch the cause. `AGENTS.md` already requires a
planted-violation test observed failing before the fix, and the cause each time was skipping that --
which no rule can enforce.

## Done means

- Opt-in rules report a record in which a section heading appears twice. — evidence: code:rust/crates/urzua-core/src/rules/section_structure.rs#section_duplicate@sha256:e45c5f7be18c65ff72960807c826c23b136b7ebd0720c34bf13b3267a30eaf5d, test:rust/crates/urzua-cli/tests/section_structure_integration.rs#a_heading_that_appears_twice_is_reported_at_its_second_line@sha256:455dfe58df04491cdc82a74ccdeceefd71368981adcf2ed3253b52c55677fcb2
- Opt-in rules report a record in which a heading level is skipped (relative to the type's declared `sections.depth`, not an absolute `##`). — evidence: code:rust/crates/urzua-core/src/rules/section_structure.rs#section_level_skipped@sha256:d207a0ac947439741274064d94c73907a1d947916acf66eb502448baa3f7ccab, test:rust/crates/urzua-cli/tests/section_structure_integration.rs#a_level_with_no_parent_above_it_is_a_skip_relative_to_the_depth@sha256:97097d3c848d0ec5ed5b0531bf0cb2ac82c6d7a241422575405de807e9944a72, test:rust/crates/urzua-cli/tests/section_structure_integration.rs#a_level_two_deeper_than_its_parent_is_a_skip_at_depth_three@sha256:e9b3469603db95a7c0b2c6963ab4b5bd9b59e12d6be02c65495024f4a6cf3db9, test:rust/crates/urzua-cli/tests/section_structure_integration.rs#a_heading_below_the_declared_depth_is_not_read_as_a_skip@sha256:5e0be3de49b94e1371baad40748feabde4c1f57a9206b591cceaf11eade21e12
- Opt-in rules report a record whose sections appear out of the order the type's template gives. — evidence: code:rust/crates/urzua-core/src/rules/section_structure.rs#section_order@sha256:639cfb2811b39901340d2d5457887ba5371a39b425b413275ccc93c3f56683ad, test:rust/crates/urzua-cli/tests/section_structure_integration.rs#sections_out_of_the_templates_order_are_reported@sha256:8c6c91888fae399e67ac4a0f47c99db1be7fe130590414d2f1c4f23ff19c7c9c, test:rust/crates/urzua-cli/tests/section_structure_integration.rs#an_order_is_read_within_one_parent@sha256:9b8752d744e1c8904639bdae9f5c0db108b984e58700f4fda184a7f0ccb414bc
- A fixture per case and a clean record show each; the rules are declared in urzua's own `.urzua/config.yaml`; and `check` over urzua's own records reports none unwaived. — evidence: test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-z/expectations/a-heading-that-appears-twice-is-reported.yaml@sha256:b2e32e5b0e2bb109328622998dd2ace3a529d38eafd17c2f92b311a0c5599580, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-z/expectations/a-skipped-heading-level-is-reported.yaml@sha256:48f9c82ca3dcd67bfea02a7113c8e4756f35236847d8051b13792a60700ae960, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-z/expectations/sections-out-of-the-templates-order-are-reported.yaml@sha256:61efa2b459079a42e9d8cfeca6e4161ac70a6bc1e75afe6b00ca8901da2d4c51, code:.urzua/config.yaml#record_types::adr@sha256:c1e0be317c437c5bbca9cb88d27f1658a0b35d52edff4802f249ef05437cb947, test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#this_repositorys_own_corpus_is_audited_and_checked_with_rules@sha256:9cac64a9f6e7cc899b135613c8d806cc5559a0b87a33a70adba1d1d09daa252e
- The rules build on `MILE-98`'s sections and leave `MILE-4`'s required-presence policy where it is. — evidence: code:rust/crates/urzua-core/src/rules/section_structure.rs#structure_census@sha256:225e8eede4e51d041e1b55fedd1e287633e97151fa409443a00b34b74301f407, test:rust/crates/urzua-core/src/document_model.rs#the_structure_keys_default_off_and_parse_when_declared@sha256:c28131f8793d47e9f37c7742aff97ceefcc3b3887c7e07fde474a79de9332ff7, test:rust/crates/urzua-cli/tests/section_rules_integration.rs#declared_rules_report_each_failure_and_count_slots@sha256:fd72db5c46d6b283f04a776dfb01e5fec3d7b8e2e28a7a6dcf2fac68e22c5376

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** none of the twenty-one shipping rules examines section structure, so `SPEC-1` shipped with two `## Open questions` giving contradictory answers and `check` stayed green. Separated from `MILE-4` deliberately: a missing required section is a governance finding, a duplicated one is a broken document, and no configuration makes the second acceptable. | **substantive** |
> | 2026-09-19 | Order and skipped-level cases specified against their sources. **Why:** both were stated absolutely. *Out of a declared order* had no declaration to read -- `required_fields` is a set, not a sequence -- and *a skipped level* assumed `##` is always the top, which `MILE-98`'s `sections.depth` makes configurable. A milestone that cannot say where its rule reads from is a direction, not a deliverable. | **substantive** |
> | 2026-11-04 | Re-phased from Phase 0 to Phase 1; `Status` unchanged. Not needed to replace the adopter's linter. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the exit is the adopter's linter deleted and urzua's own repo clean in CI, and this work is on neither path, so it moves out of Phase 0. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-04 | `Blocked-on` cleared: `MILE-98` is Done (declared `identity`, `fields` and `sections`, including `###` headings read through `sections.depth`). `Status` unchanged. **Why:** `narrative-field.stale` flagged a blocker that reached a terminal status; the dependency no longer holds. | **substantive** |
> | 2026-11-05 | `Depends-on: MILE-98` added. **Why:** the section 'Blocked on MILE-98' says sections are not addressable until the document model declares them (Done). Proposed for maintainer review; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-13 | Built as the opt-in rules `section.duplicate`, `section.level-skipped` and `section.order`, each read from one key of a type's `sections` declaration (`unique_headings`, `no_skipped_levels`, `order: {from: template}`; `SPEC-3`, `SPEC-2`). Order reads `.urzua/templates/<type>.md`, the source this record preferred. `Realized-by` added and `Status` Planned -> Done. Read against `MILE-98`'s `sections.depth`, which counts levels from `##`: at `depth: 3` the tree reads `##` to `####`, so a `###` with no `##` above it is still a skip, and the 'ordinary section' sentence above is read as a `###` nested under a `##`. The rules are enabled in this repository's own `.urzua/config.yaml` for every type that declares `sections`; `MILE-4` held a second copy of three sections and `ADR-50` and `ADR-51` placed `## Reversibility` after `## Consequences`, and each is corrected in the same change with its own revision row. **Why:** the work is delivered with a test that fails before it, an acceptance case per rule, and a clean `check` over this corpus. | **substantive** |
> | 2026-11-13 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (5 of 5 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
