---
Stable-Id: 01M6N6PC6Y79X69H1AKA01RA9K
Status: Accepted
Accepted-on: 2026-11-05
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: —
---
# 84 — A record's status follows its declared evidence field

## Context

`MILE-183` made a milestone's `Status` answer to evidence: `statuses.delivery` names the computed tiers
that mean the work was delivered, and `status.delivery-agreement` reports a record that is not done while
its evidence says it is, and a done record with nothing behind it. The mechanism is already per declared
type. What it reads, though, is the embodiment machinery (`Realized-by` and source back-pointers). A type
whose evidence is a different declared field, a test locator for instance, has no way to say so, and its
status is held to nothing.

urzua is a general-purpose engine. A record type called `bug` is only what this repository declares; another
team may declare none, or call the same thing something else. This decision is therefore phrased over
declared types and fields, and the engine names no type, status or field.

This repository is the measured example. Its declared defect type has the problem: of 167 such records that
reached their success status, 110 were filed already in it by the fixing PR, 48 were closed later by a code
PR, and 9 by a records-only commit, of which 6 were genuine lag (`BUG-17`, `BUG-57`, `BUG-65`, `BUG-145`,
`BUG-178`, `BUG-179`, found by one triage pass on 2026-11-05, b031f80). All six fixing PRs named the record
in their title or message, and five of the six appear in no release fragment, so a rule that reads
changesets cannot see them (`ADR-81`, rejected on the same evidence, tried that). That is 6 of 167, 3.6
percent, found by hand once.

A declared signal exists in principle: a record in a not-done status whose evidence field names a test that
exists. Before the triage pass, 3 of the 6 (`BUG-65`, `BUG-178`, `BUG-179`) carried a named test; 3 did not.
Those figures are what the signal would have done **after** the evidence field is migrated to locators; they
are not what the corpus supports at HEAD. At HEAD the field is free text. Of the 178 `Regression-test`
values in `docs/bugs`, 1 uses the `path#Symbol` form `locator.rs` parses (`parse_locator`), 90 use
`path::test_name`, 113 contain a comma (a list, or `name, file.rs`; `BUG-99` is `name, check_integration.rs`),
and some are prose. Almost none resolves as a locator today. After migration the expected figures are 3 of 6
historical cases caught, with one deliberate false positive (`BUG-99`, a partial fix). This ADR claims no
more, and sizing the migration (how many records, in which shapes) is part of the implementing change, not
assumed here.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Leave it to review and periodic triage | No engine change | The lag recurs (6 of 167) and was found by one manual pass |
| B. Read prose: a changeset or PR body that names the record (`ADR-81`) | No new field | Rejected on measurement: 0 real hits among 19 flagged |
| C. Let `statuses.delivery` read the evidence field every type already uses, `Realized-by`, verified to exist (chosen) | One mechanism, already built; one evidence field across every type and no type-specific mapping; no new field kind and no change to the fixed `RelationRole` set (`SPEC-3`); opt-in per type | A type's separate test field is migrated into `Realized-by` as category-prefixed locators and then retired; recall is partial |
| D. A new "test-locator" field kind beside `Realized-by` | Lets a type keep a separate test field | A second way to name evidence that `judge_delivery` and `embodiment.locator-exists` would both have to learn; the role set is fixed by design |
| G. Remap the `embodiment_locator` role (`relation_fields`) of a type to its own test field | Needs no value move into `Realized-by`; the existing machinery reads the remapped field | Not chosen: it keeps two evidence fields (a type's own and `Realized-by`), needs a type-specific mapping, and a role maps to one field per type, so that type's existing `Realized-by` values would stop being read |
| E. Add a "partially fixed" status | Expresses `BUG-99` | A status the engine would have to know; the waiver mechanism already expresses a deliberate exception |
| F. Auto-downgrade a done record with no evidence | Symmetric | Wrong where a done record legitimately has none (a documentation or performance change) |

## Decision

In the context of records whose status lags the change that delivers them, facing a measured lag of 6 in
167 that no rule reads, we decided to **let a type declare an evidence field and hold the record's `Status`
to it, as `statuses.delivery` already does for milestones**, to achieve a delivered record being reported
before the lag is found by hand, accepting that the signal is partial and has a known false positive.

- **The evidence is the type's `Realized-by` field, the same field every other type uses.** (Maintainer
  decision of 2026-11-05.) `judge_delivery` (`rules/status.rs`) already reads the `embodiment_locator` role
  field, which defaults to `Realized-by`, and `relation_fields.embodiment_locator` needs no remap. The
  field's value is read in the `Realized-by` grammar (`category:locator`, `locator.rs` `realized_by_entries`),
  each locator is `path` or `path#Symbol` resolved by `locator_resolvers` (`ADR-74`), and
  `embodiment.locator-exists` verifies it whatever the record's status. No new field kind is added and the
  fixed `RelationRole` set (`SPEC-3`) is unchanged.
- **A type's separate test field migrates into `Realized-by` and is retired.** The values of such a field
  (this repository's example: the defect type's `Regression-test`) move into `Realized-by` as `test:path#name`
  entries, alongside the values `Realized-by` already holds (17 here, which stay). Several tests are allowed,
  comma-separated before each category prefix: `test:a.rs#one, test:b.rs#two` (the splitter breaks at a comma
  only when a `category:` follows, `BUG-129`, merged in #291). The implementing change then retires the
  separate field: it is removed from the type's `known_fields` and from the template. The splitter requires a
  category prefix, so a value that is bare text or `path::test_name` yields no entry: it is not evidence, and
  it is counted and disclosed, never guessed at and never reported as a defect.
- **The tier is the existing tier computation.** `judge_delivery` gets its tier from `compute_embodiment`
  (`rules/embodiment.rs`), which ranks the highest category with a non-empty entry list: `test:` entries compute
  to the test tier, `code:` to the code tier, `spec:` to the spec tier, in the order the repository declares
  under `embodiment.tiers`. A `test:` entry in `Realized-by` therefore computes to the test tier, and a type's
  `statuses.delivery.delivered_tiers` lists the tiers that count (this repository: the test tier, and the code
  tier if it chooses). `delivered_tiers` consumes that tier exactly as it does for milestones; nothing new is
  read. Several locators in one value are resolved and verified one by one; a comma inside a path stays in the path.
- **The delivering PR sets the status.** The change that delivers a record sets its declared success status
  and fills its evidence field in the same change (the fixed-in-PR convention, `BUG-180`, `BUG-186`;
  `MILE-183` for milestones).
- **A not-done record with evidence is reported.** A record whose `Status` is a declared not-done status
  while its evidence field computes to a delivered tier is a disagreement. The finding names the evidence.
  As `ADR-73` fixes for `status.delivery-agreement`, `urzua fix --apply --ids <record>` may set the declared
  success status with a structural revision row, and the suggested action is a `run-command` for that
  command (an `edit-field` set of `Status` is the fallback where no repair exists).
- **A done record with no evidence is reported and never downgraded.** `fix` does not touch it: absence of
  evidence is legitimate for some changes, and only a human can say which.
- **Whether a source back-pointer alone counts is a per-type option, with today's behaviour as its default.**
  `judge_delivery` already counts source back-pointers (`BackPointerEvidence`, `status.rs`; tested in
  `status_delivery_integration.rs`, `a_source_back_pointer_is_delivery_evidence_for_a_record_with_no_realized_by`),
  so every config that declares `statuses.delivery` today keeps that. The option sits beside
  `statuses.delivery` on the type and, when a type declares it, can switch the back-pointer off as evidence
  for that type. A type that declares nothing is unchanged. The "only the declared field counts" behaviour
  applies only where a config says so.
- **A deliberately still-open record with evidence is a waiver, not a status.** There is no "partial"
  status. `BUG-99` is expressed with the existing waiver record (`ADR-11`). A waiver's `Scope` is a file path
  (`SPEC-10`), so "scoped to that record" means the record's path, with `Rule` set to
  `status.delivery-agreement`, and the record keeps a `Reason` and an `Expires`. The stale-waiver rule keeps it
  honest: when the record is closed or the expiry passes, the waiver is reported.
- **It is opt-in config with no type knowledge in the engine.** The mapping is `statuses.delivery`, on any
  type that declares it. The engine hardcodes no type, status or field name. A type that declares none is
  not examined, and no config that exists today changes behaviour. Enabling the rule on a type that does not
  declare its evidence field (`Realized-by`) in `known_fields` is a config load error, not an inert rule
  (`BUG-44`).

## Reversibility

Moderate. The rule is additive and opt-in; a repository that declares it on no type is unchanged, and
`MILE-183`'s keys keep their meaning. What is harder to take back is the migration of a separate test field into
`Realized-by` and the retirement of that field, because rewriting a field's values across a corpus is a mass edit. The
`fix --apply` write is reversible by a normal revert.

## Consequences

- `SPEC-3` documents that `statuses.delivery` reads whichever field the type maps to `embodiment_locator`,
  gains the per-type back-pointer option, and says `statuses.delivery` applies to any type; `SPEC-2` gains the
  finding wording. Each gets a revision row.
- This repository declares `statuses.delivery` on its defect type, which reads `Realized-by` with no
  `relation_fields` remap, migrates the `Regression-test` values that name a real test into `Realized-by` as
  `test:path#Symbol` entries (`path::test_name` becomes `test:path#test_name`; the Rust resolver covers a
  free function), and retires `Regression-test` from `known_fields` and the template in the implementing
  change. The rest stay as disclosed non-evidence. Migration sizing: 178 `Regression-test` values, of which 1
  uses `path#Symbol`, 90 contain `::` and 113 contain a comma; plus the 17 existing `Realized-by` values,
  which stay. Part of the migration is judgement and should be reviewed, not scripted.
- A value such as `none -- performance-only` is neither evidence nor reported unless the repository
  declares it evidence-free; the engine does not know the words.
- After migration the rule would have caught 3 of the 6 known lagging records. The other 3 had no test when
  the lag began; only a source back-pointer could have caught them, which `judge_delivery` already counts
  where a type has not switched it off. That is a limit of the signal.
- `BUG-99` is flagged on the first run, so the change that enables the rule also adds its waiver.

## Resolved questions

1. A test written as a name plus a file (`..._observed_failing, rules.rs`) is a comma-separated list of
   category-prefixed locators (`test:rules.rs#name`); the field accepts several, split before each
   `category:` by the existing splitter.
2. The back-pointer option is per type, beside `statuses.delivery`, not per repository.
3. Which field is the evidence (decided by the maintainer, 2026-11-05): `Realized-by`, as for every other
   type. The separate test field's values migrate into it and the field is retired; no `embodiment_locator`
   remap is needed (option G, not chosen).

## References

- `MILE-183` -- the delivery model this extends; `ADR-81` -- the prose route, rejected on measurement.
- `BUG-180`, `BUG-186` -- the fixed-in-PR convention; `BUG-99` -- the known false positive.
- `ADR-64` -- structured in, structured out; `ADR-53` -- declared, opt-in; `ADR-73` -- suggested actions.
- `ADR-67` -- source back-pointers; `ADR-74` -- locator resolvers; `ADR-11` -- waivers.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** maintainer decision of 2026-11-05, after the review of `ADR-81` found that the recurring failure is a delivered record left in a not-done status (6 of 167) and not a bare reference in a changeset; the maintainer approves before it is Accepted. | **substantive** |
> | 2026-11-05 | Amended while Proposed: retitled and rephrased over declared types and fields, since a record type like `bug` is only what this repository declares; the evidence is a declared test-locator field (no `Realized-by` is added); a back-pointer alone counting is a config option, default off; a deliberately open record with evidence is a scoped, expiring waiver, not a status. **Why:** maintainer answers of 2026-11-05 to the open questions. | **substantive** |
> | 2026-11-05 | Amended while Proposed after review: the measured recall is restated as post-migration figures (at HEAD 1 of 178 `Regression-test` values is a `path#Symbol` locator); the evidence reuses the existing `relation_fields.embodiment_locator` role remap instead of a new field kind (which is kept as a considered option), with its limits stated; the tier is stated (`compute_embodiment`, a `test:` entry computes to the test tier, several locators split before each `category:`); the claim that no `Realized-by` is added is corrected (the defect type already declares it and 17 records use it); the back-pointer option defaults to today's behaviour; the fix and waiver wording follow `ADR-73` and `SPEC-10`; open questions resolved. **Why:** review blockers on the Proposed text. | **substantive** |
> | 2026-11-05 | Amended while Proposed: the evidence field is `Realized-by` for every type; a separate test field migrates into it as `test:path#name` entries (several comma-separated) and is retired in the implementing change; the `embodiment_locator` remap moves to the considered options (G) as not chosen; migration sizing restated (178 values: 1 `path#Symbol`, 90 `::`, 113 comma; 17 existing `Realized-by` stay). **Why:** maintainer decision of 2026-11-05 on the question left to the implementing change. | **substantive** |
> | 2026-11-05 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-05. | **substantive** |
