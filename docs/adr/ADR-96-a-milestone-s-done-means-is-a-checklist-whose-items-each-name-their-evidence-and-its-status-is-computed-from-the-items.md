---
Stable-Id: 01M6T314SMZFAZHERWR0TVWC1W
Status: Proposed
Date: 2026-11-07
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
---
# 96 — A milestone's Done means is a checklist whose items each name their evidence, and its status is computed from the items

## Context

`judge_delivery` (`rust/crates/urzua-core/src/rules/status.rs:140-198`) decides whether a record is delivered from one input, its evidence tier. It reads the type's evidence field and nothing else (`status.rs:156-160`), turns any `Realized-by` locator, or any source or test back-pointer that cites the record, into a tier with `compute_embodiment` (`status.rs:163-171`; `rules/embodiment.rs:90-100`, which returns the highest tier that has at least one entry), and compares that tier with the status (`status.rs:179-188`). It cannot read the milestone's `## Done means` section: it is handed a `Record`, and a `Record` holds the path, type, identity and header, not the body (`record.rs:19-34`). One resolvable locator therefore means delivered, whatever the section asks for.

The consequence is a defect in the engine, confirmed by the maintainer on 2026-11-07: a milestone with five Done-means items and evidence for one computes `Delivered`, and `status.delivery-agreement` then reports it as an error until its status is `Done` (`status.rs:179-183`), and `urzua fix --apply` writes `Done` (`fix.rs:133-155`). `MILE-165` is the instance. Its Done means requires the preamble reading to be exercised by this repository's own records; its own revision row admits it is not; the code and test locators are real; so the engine insisted on `Done` and `WAIVER-0051` was filed to hold the honest `InProgress`. The maintainer's words: "it shouldn't be closing off things as done when they're not done". A waiver per milestone does not scale and turns the rule into noise.

The corpus makes the problem larger than one record. At `03a2c6df`, 143 milestones are `Done`, 127 of them in Phase 0 (129 Phase 0 milestones: 127 `Done`, 1 `InProgress`, 1 `Planned`). Of those 127, only 22 write the Done means as a list; 105 write one prose paragraph of semicolon-joined clauses. Two clause shapes are the ones a single locator cannot cover: 35 say the capability is "exercised" in this repository's own config, records or corpus, of which 2 cite a config or corpus path in `Realized-by`; 45 say a spec documents it, of which 17 carry a `spec:` locator. The clause `MILE-165` failed on is the commonest unevidenced one.

The decision is the maintainer's, made on 2026-11-07: evidence per Done-means item. This ADR records the design.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Leave the engine; hold honest milestones with waivers (today, `WAIVER-0051`) | No change | One waiver per partly delivered milestone, each expiring; the rule keeps demanding a false status; the signal is wrong, not noisy |
| B. Evidence-count heuristic: require N locators, or a locator per clause, before a milestone computes delivered | No record syntax change | **Rejected.** The count has no link to the criteria. One test can evidence five items, and five locators can evidence one; a threshold is gamed by adding a locator and is silently wrong in both directions. 105 of 127 Done means are prose with no countable items. It computes from a proxy where the maintainer asked for the criteria |
| C. Human sign-off: an `authority` gate on `Done` (`ADR-89`) so a person decides | Puts judgment on the person who has it | **Rejected.** `status.delivery-agreement` would still demand `Done` from the same wrong evidence, so every gated milestone needs a waiver and the sign-off decides nothing the engine reads. It also contradicts the `ADR-70` amendment, under which a lifecycle command sets a status only when the engine computes it from evidence. Review of the PR stays the human step, as decided on 2026-11-04 |
| D. A header field, `Done-means:` as a YAML list of `{text, evidence}` | Machine-first | Splits each criterion from the section a reviewer reads, duplicates the required `## Done means` heading, and leaves the section as prose beside the list. Two sources for one fact (`ADR-64` clause 4) |
| E. List items with a `[x]` checkbox as the state, plus evidence | Reads well on GitHub | **Rejected.** A ticked box is a hand-written copy of a value the engine computes (`ADR-83` made the same objection to a written `Blocked`). A stale `[x]` is the defect again, and GitHub shows `[ ]` on every item |
| F. Plain list items in the existing section, each ending in a declared evidence clause in the existing `Realized-by` grammar (chosen) | One section, one grammar, no new field kind, state computed from the items, the diff shows criterion and evidence on one line | A record-body syntax to parse; 105 prose paragraphs to split |

## Decision

In the context of a milestone whose status is computed from evidence that never looks at its Done means, facing a record such as `MILE-165` that the engine forces to `Done` while an item is unmet, we decided that **a milestone's Done means is a list of items, each item names its own evidence, and the engine computes delivered only when every item that is not waived resolves**, to achieve a status that cannot claim more than the criteria were met, accepting a record syntax to maintain and a migration of every milestone's Done means.

**1. The item syntax.** In the section the type names, each list item is one criterion. Its evidence follows a declared marker, in the `Realized-by` grammar that `realized_by_entries` already splits (`locator.rs:256`), so a locator, a scoped symbol and a `@sha256:` pin mean exactly what they mean in the header (`locator.rs:13-19`):

```markdown
## Done means

- A declared alias satisfies the section requirement and is reported as the alias used. evidence: code:rust/crates/urzua-core/src/rules/section.rs#section_required, test:rust/crates/urzua-cli/tests/section_rules_integration.rs#an_alias_satisfies_the_requirement
- The capability is exercised by this repository's own config. evidence: code:.urzua/config.yaml#bug
- The release notes name the change. waived: process-outcome -- the notes are written when the release PR is opened, #512
```

The item text is everything before the last occurrence of the marker on the item (continuation lines belong to it); the clause is the rest. The markers are configuration, not engine words. Justification: (a) a list item stays readable in a diff and on GitHub, and a reviewer sees the criterion and its evidence together; (b) the evidence grammar is the one every reader of `Realized-by` shares, so `embodiment.locator-exists`, scoped pins and drift read item locators with no second grammar; (c) no checkbox, because state is computed (option E); (d) it keeps the existing `## Done means` heading, so `section.required` and the template are unchanged in shape. A clause that does not parse is a finding, never skipped: `status.done-items-shape` reports a prose-only section on an opted-in type, a clause whose text after the marker is not entirely locators, a leading `[x]` or `[ ]`, an unknown waiver category, a waiver with an empty reason, and a category that requires a citation without one. Nothing is guessed (the no-silent-no-op principle).

**2. Status follows the items.** For a type that declares the option, `judge_delivery` computes per item. An item is *resolved* when it has at least one locator, every locator exists (the `embodiment.locator-exists` check, extended to item clauses), and its tier, computed by the existing `compute_embodiment` over only that item's locators, is in the item tier set. The item tier set is `delivery.items.resolving_tiers`, default `delivered_tiers`. An item is *waived* when it carries a valid waiver. With `n` items, `w` waived and `r` resolved:

| Items | Computed standing | Tier reported | Status it agrees with |
|---|---|---|---|
| `r >= 1` and every non-waived item resolved | delivered | the lowest tier among resolved items, so `Verified` only if every item is | the type's `set_status` (`Done`) |
| `r >= 1` and some non-waived item unresolved | partial | the same lowest tier, with the unresolved items named | `delivery.items.partial_status` (`InProgress`); also correct under `InProgress` |
| `r = 0` | none | the declared `embodiment.none` (`Not started`) | `Planned`; also correct under `InProgress` (work without evidence yet is legitimate) |

A milestone with every item waived has `r = 0`: waivers excuse an item, they do not deliver one. The tier set decides what counts, exactly as `delivered_tiers` does in `ADR-84`; a repository that wants a documentation or decision item evidenced by a `spec:` locator lists `Specified` in `resolving_tiers` and gets that, one that does not lists nothing and those items resolve only with code or a test.

The disagreements, extending `status.delivery-agreement` with variants (the rule table at `rules/table.rs:1418` gains them):

- delivered while the status is in `not_done`: reported, repaired by `fix` (unchanged in kind from `DeliveredButNotDone`, `status.rs:128-131`).
- partial while the status is `Planned`: reported, repaired by `fix` to `partial_status`.
- a `success` status with unresolved items or with none: reported as `SuccessWithoutEvidence` is today (`status.rs:184-186`), naming each unresolved item by number and text. **`fix` never downgrades it** (the `ADR-84` rule, unchanged: absence of evidence is sometimes legitimate and only a person can say which). The repair is evidence, a waiver, or reopening the milestone by hand (`Done` to `Planned` or `InProgress` is already allowed by the `SPEC-6` transitions).

This maps onto `ADR-84` without new meaning: `delivered_tiers`, `unevidenced_tiers`, `not_done` and `set_status` keep their roles (`status_model.rs:95-113`), and the engine hardcodes no status name; `partial_status` is declared like `set_status`.

**3. Waiving an item.** An item is waived with `waived: <category> -- <reason>` (the markers are declared). The categories are a declared vocabulary with the shape `ADR-86` gave the permanent waiver (`name`, optional `cite: required`, validated at load), and the reason is required prose. This repository declares four, none borrowed from `ADR-86`, because those describe why a defect has no test and none describes a missing criterion: `not-applicable` (the criterion no longer applies); `superseded-by-decision` (cite required: an ADR or RFC removed it); `deferred-to` (cite required: the milestone that now carries it, so the work is not lost); `process-outcome` (a result no file can evidence, such as a green CI run; the reason names the PR). A cited record is a pointer, checked by the existing pointer rules.

Why inline and not a separate waiver record: `ADR-86` rejected an exemption "written by the same hand, in the same file, as the claim it excuses" (its option C), and that argument is weaker here because the item is the claim and its line is reviewed in the PR that writes it. The record is the only place the criterion lives, so a waiver that lives elsewhere has nothing to point at: a `Scope` is one file path (`SPEC-10`), and a waiver scoped to the milestone would excuse every item at once (`ADR-86` option E's objection). Two safeguards replace the separate record. Every waived item is listed in the report, never omitted, and counted by category, as `ADR-86` counts permanent waivers. And a `Done` milestone with a waived item is an informational notice on every run. The cost, an author excusing their own item, is a review risk the report makes visible; open question 1 asks the maintainer to confirm it.

**4. Opt-in.** The option is `statuses.delivery.items` on a type. Absent, `judge_delivery` runs the code it runs today and every existing config is unchanged byte for byte in behaviour:

```yaml
delivery:
  delivered_tiers: ["Implemented", "Verified"]
  unevidenced_tiers: ["Not started"]
  not_done: ["Planned", "InProgress"]
  set_status: "Done"
  items:
    section: "Done means"            # must be a section the type requires
    evidence_marker: "evidence:"
    waiver_marker: "waived:"
    partial_status: "InProgress"     # in values and not_done
    resolving_tiers: ["Specified", "Implemented", "Verified"]
    waiver_categories:
      - {name: not-applicable}
      - {name: superseded-by-decision, cite: required}
      - {name: deferred-to, cite: required}
      - {name: process-outcome}
```

Config load errors, not inert rules (`BUG-44`): `items` on a type without `delivery`; a `section` the type does not declare; a `partial_status` outside `values` or `not_done`, or one with no declared transition from the statuses it repairs; a `resolving_tiers` entry that is not a declared embodiment state; a duplicate or empty marker or category. This repository turns it on for `milestone` and no other type; the `bug` type, whose evidence is one regression test, is not itemised.

**5. The evidence field.** Item locators are evidence for every reader, not only for status. `Embodiment`, drift, `embodiment.locator-exists`, `fix --rebaseline` and the graph read one function that returns the union of the `Realized-by` field and the item clauses (`fix.rs`, `status.rs`, `embodiment.rs` and `diff_scope.rs` each parse the field today; the shared parser is the single seam, `embodiment.rs:48`). `Realized-by` stays valid for record-level evidence tied to no single criterion. Delivery is judged from the items only, so a record-level locator never makes a milestone delivered.

**6. `/done` and `fix --apply`.** Both compute from the items and neither writes a status the engine did not compute. `/done` (`.claude/hooks/urzua-record.sh:133-138`) adds evidence to the items the change delivered, then runs `urzua fix --apply --ids <record>`. If the engine computes nothing, the message now names the unresolved items instead of only "add the evidence" (the hook already stops when no `computed_value` is printed, `urzua-record.sh:139-143`). An agent that has finished three of five items runs `/done` and gets `InProgress`, not `Done`; it does not get to choose. `fix --apply` writes `Done` only when all non-waived items resolve, `InProgress` when some do, and its revision row records the counts ("3 of 5 items resolved"), so the attribution is the evidence. `fix --rebaseline` rewrites pins inside item clauses the way it rewrites them in the header (`MILE-181`). The `ADR-70` amendment holds unchanged: the status is computed from evidence, and the evidence is now finer.

**7. Partially delivered records, the roadmap and blocked.** Blocked stays derived from the written `Status` of the dependency (`sequence.rs:575-580`, `roadmap.rs:278-296`, `roadmap.rs:535`): a dependency is satisfied only in a `success` status, so a partly delivered milestone keeps its dependents blocked. The wait is item-blind on purpose; because `Status` is held to the items by `status.delivery-agreement` at error level, the two cannot disagree. No per-item dependency (`Depends-on: MILE-9#2`) is added (open question 3). The roadmap reads a partial record as open work: it stays in its wave and in the ready set if nothing it depends on is open, and the critical path does not change, since the roadmap weights nothing. `RoadmapRecord` (`roadmap.rs:101-116`) gains an optional `items` object, `{total, resolved, waived}`, present only for an opted-in type, so a reader sees "3 of 5" without a status the engine would have to know. One interaction needs a rule: `partial_status` is a `progress` status (`ADR-83`), so repairing a `Planned` milestone that depends on an open record to `InProgress` would create a `sequence.status-agreement` error. `fix` therefore does not write `partial_status` to a record that is derived-blocked; the finding stays and names the open dependency, because evidence landing ahead of its dependency is a fact the author should see.

**8. Migration of every milestone.** The syntax is backwards compatible: an old rule ignores an item clause as text. So the conversion lands in batches, and the opt-in is switched on in the last PR only when `urzua check` is clean over every milestone. For each of the 127 Phase 0 `Done` milestones (and the 16 `Done` milestones of later phases, which the switch also covers; 143 in all):

1. Split the Done means into items, by hand with tool assistance (a split on semicolons and `and` is a draft, not the result; 105 are prose and 22 are lists).
2. Give each item the locators that evidence it, from the record's `Realized-by`, the merged PR (`git log -S`) and the repository.
3. Run `urzua check`. An item that does not resolve is verified against the repository by reading, then exactly one of: the claim is true and a locator is missing, so add it; the claim is not true, so reopen the milestone (`Done` to `InProgress`, a revision row naming the item and why); the claim is true and no file can evidence it, so waive it with a category and a reason.
4. A milestone is never kept `Done` for a false claim, and there is no blanket waiver. `WAIVER-0051` stays until the switch-on PR, which removes it: `MILE-165`, rewritten into items, computes `InProgress` from the engine.
5. Reviewed, not scripted: a reviewer reads each reopen and each waiver.

Sizing, from 20 Phase 0 `Done` milestones (every sixth in filename order, so not chosen by hand; read against their `Realized-by` and, for every "exercised in our own config" clause, checked in `.urzua/config.yaml`):

| Outcome after splitting into items | Sample | Extrapolated to 127 |
|---|---|---|
| Every item already evidenced by the locators the record carries (`MILE-1`, `119`, `124`, `138`, `144`, `64`, `7`) | 7 of 20 | about 44 |
| Claim true, a locator missing: a config, spec or corpus locator (`MILE-4`, `58`, `130`, `150`, `156`, `162`, `169`, `181`, `187`, `193`, `199`, `204`) | 12 of 20 | about 76 |
| An item no file evidences, such as "`cargo test --workspace` is unchanged in result" (`MILE-175`) | 1 of 20 | about 6, to waive or reword |
| Claim shown false, so reopened | 0 of 20 | 0 at the point estimate; at 95% confidence no more than about 19 |

The sample found no milestone that is `Done` falsely, besides `MILE-165` which was found before it and is already reopened. That is weaker than it sounds: the 35 "exercised" clauses are the shape that failed, only 2 of them cite a config or corpus path, and the sample read 5 of them, not 35. Expect most of the real finds among those 35, not among the 92 others. The ADR commits to the rule, not the number: whatever surfaces is reopened or waived, never kept `Done`.

## Reversibility

Moderate. The option is opt-in and an undeclared config is unchanged, so the engine change is reversible by a config edit. The migration is not free to undo: once 143 Done means carry item clauses, dropping the option leaves the clauses as inert text (harmless, since old rules ignore them) but reopened milestones stay reopened by design. The marker words and waiver categories become a vocabulary in the corpus, and renaming one is a mass edit, as `ADR-86` found for its categories. The riskiest part is the reading of free text into items: a split that loses a criterion makes a milestone look delivered, which is why the split is reviewed, not scripted.

## Consequences

- `judge_delivery` gains an item input. `urzua-core` never opens a file and a `Record` has no body, so the caller reads each opted-in record's section and passes the parsed items beside `BackPointerEvidence`, the way back-pointers are passed (`status.rs:140-145`). `fix.rs:133` and `fix.rs:189` use the same judgement, so `check` and `fix` cannot disagree.
- `SPEC-3` documents `statuses.delivery.items` and its load checks; `SPEC-2` the `status.done-items-shape` rule and the new `status.delivery-agreement` variants; `SPEC-6` the item syntax in its Required sections and its `Realized-by` row; `SPEC-8` the repairs (`Done`, `InProgress`) and that a `success` status is never downgraded. Each gets a revision row. The CLI changeset is minor (a new capability).
- The `milestone` template changes: `Done means` carries an example item line instead of "one or more bullets".
- `/done` and `/milestone` text (generated by `urzua harness`) says to name evidence per item; `urzua harness status` then reports drift until `harness update` is run.
- The report gains the waived items by category and a per-milestone item count; the roadmap gains the optional `items` object.
- Writing a milestone costs more: each criterion needs a locator or a waiver. That is the point; it moves the cost from a reviewer who must remember to ask to the author who must show.
- A milestone cannot be finished by its last PR alone if an earlier item had no evidence: that is the intended friction.
- Not decided here: per-item dependencies, weighting the critical path by items, itemising `bug` or other types, and a stable item id (items are numbered by position in reports, which breaks if an item is inserted).

## Open questions

1. Inline waivers or a separate record per waived item? Recommended: inline, as above, with the report listing every waived item and a notice on a `Done` milestone that has one. Switch to a record with an `Item` field only if waivers are abused in review.
2. Should `fix` reopen a `Done` milestone whose items do not all resolve? Recommended: no. Reported only, as `ADR-84` already holds; reopening is a person's act.
3. Per-item dependencies? Recommended: not now; the wait is item-blind and a milestone that needs another's third item should be split.
4. Should `resolving_tiers` include `Specified` here? Recommended: yes, so a decision or documentation item is evidenced by a `spec:` or ADR locator without a waiver; `delivered_tiers` stays as it is.

## References

- `ADR-84` -- a record's status follows its declared evidence field (the mechanism extended); `ADR-83` -- blocked is derived only, `progress` statuses; `ADR-86` -- the permanent-waiver vocabulary shape reused for item waivers.
- `ADR-70` and its amendment of 2026-11-05 -- a lifecycle command sets a status only when the engine computes it from evidence; `ADR-89` -- lifecycle commands and `authority` gates.
- `ADR-64` -- structured in, structured out; `ADR-74` -- locator resolvers and pins; `ADR-82` -- the roadmap.
- `SPEC-2`, `SPEC-3`, `SPEC-6`, `SPEC-8`, `SPEC-10`.
- `MILE-165`, `WAIVER-0051` -- the instance; `MILE-183` -- the delivery model this refines; `MILE-181` -- re-pinning; `MILE-214` -- the milestone that builds this.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Drafted as Proposed. **Why:** maintainer decision of 2026-11-07, after confirming as an engine defect that `judge_delivery` computes delivered from the evidence tier alone, so a milestone with one item of five evidenced is forced to `Done`; the maintainer approves before it is Accepted. | **substantive** |
