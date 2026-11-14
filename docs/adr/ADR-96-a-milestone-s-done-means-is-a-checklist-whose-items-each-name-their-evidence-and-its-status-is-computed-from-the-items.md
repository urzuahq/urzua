---
Stable-Id: 01M6T314SMZFAZHERWR0TVWC1W
Status: Accepted
Accepted-on: 2026-11-07
Date: 2026-11-07
Author: beauwilliams
Deciders: beauwilliams
Embodiment: Implemented
Supersedes / Superseded-by: null
Non-reference-ids: WAIVER-0051
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

In the context of a milestone whose status is computed from evidence that never looks at its Done means, facing a record such as `MILE-165` that the engine forces to `Done` while an item is unmet, we decided that **a milestone's Done means is a list of items, each item names its own evidence, and the engine computes delivered only when every item that is not waived resolves**, to achieve a status that cannot claim more than the criteria were met and a guaranteed error whenever it does, accepting a record syntax to maintain and a migration of every milestone's Done means.

**1. The item syntax.** The syntax is specified here as a grammar and is carried into `SPEC-6` verbatim by `MILE-214`; it is not a "last occurrence" heuristic. It is structured text, not a string mini-language in the sense of `ADR-64`: the engine reads three declared words (the separator and the two markers, all in configuration), splits a line at the first occurrence of one declared sequence, and hands the clause to parsers that already exist (`realized_by_entries` for locators, the config vocabulary for waivers). Nothing in a clause is interpreted by position, regex or guess, and every deviation is a `status.done-items-shape` finding.

```
section   = { block } ;                  # the text of the type's declared section, fences skipped
block     = item | other ;               # `other`: any non-list text; it is not an item and is never evidence
item      = marker, line, { continuation | subitem | blank }, [ final ] ;
marker    = ( "-" | "*" | "+" | digits ("." | ")") ), " " ;
                                         # at the item indent: the indent of the first marker in the section
subitem   = deeper-indented marker line   # belongs to its parent; never an item, never a clause
line      = text, [ clauses ] ;
clauses   = [ SEP, EV, locators ], [ SEP, WV, waiver ] ;     # at least one; evidence first
final     = [ EV, locators ], [ WV, waiver ] ;               # one or two last lines, each starting at its marker
locators  = locator, { ",", [ " " ], locator } ;             # the Realized-by comma rule (BUG-129)
waiver    = category, [ " (", cite, [ ", ", date ], ")" ], " -- ", reason ;
```

`SEP` is `clause_separator` (` — `, space em dash space), `EV` is `evidence_marker` and `WV` is `waiver_marker`. The rules, each of which a worked example below exercises:

1. **What is an item.** A list item at the item indent, whether the marker is `-`, `*`, `+`, `1.` or `1)`. A list item indented deeper than its predecessor at the item indent is a sub-item: it belongs to that parent, its text is part of the parent's item text, and it is never itself an item and never carries a clause. A marker indented less than the item indent is a shape finding. Items are numbered by position, from 1, in report messages. A section with no list item (prose only) has `n = 0`; on an opted-in type that is a shape finding, and under a `success` status it is also the delivery-agreement finding of section 2 (`r = 0`).
2. **Where the clauses are.** In exactly one place per item: at the end of the item's first line, or on the item's final line or final two lines. The first line is split at the first occurrence of `SEP + EV` or `SEP + WV`; the text before it is the item text, and continuation lines and sub-items after the first line add to the item text. The final-line form exists for an item whose text runs over several lines or has sub-items: after the last sub-item or continuation line (a blank line between is allowed), the evidence clause is one line beginning with `EV` and the waiver clause is one line beginning with `WV`, in that order, each indented as a continuation. Clauses on the first line and on final lines at once are a shape finding. A marker anywhere else, including inside a continuation line or a sub-item, is plain text and is not parsed.
3. **A marker inside prose.** The first-occurrence rule is why a marker word inside text is harmless. `evidence:` without the separator before it is text (an item may say "the report prints evidence: per item"); the clause begins only at `SEP + marker`. A waiver's reason runs to the end of its line and is free prose, so `waived: process-outcome -- see evidence: CI` has one reason, `see evidence: CI`, and `— evidence:` inside a reason is part of the reason. The converse is the one thing an author must avoid: item text that itself contains `SEP + EV` splits there, and the clause that follows then fails to parse as locators, which is a shape finding naming the item. It is loud, never silent.
4. **Both clauses.** An item may carry an evidence clause and a waiver clause, evidence first. That is how a waiver stays honest: the item says what will evidence it, and the waiver excuses it until it does (section 3). A waiver without an evidence clause is valid.
5. **Several locators.** The evidence clause is read by `realized_by_entries` (`locator.rs:256`): a comma separates two locators only when a `category:` prefix (`spec:`, `code:`, `test:`) follows it, optionally after one space; any other comma belongs to the path. Scoped symbols and `@sha256:` pins mean what they mean in the header (`locator.rs:13-19`). A locator is not wrapped in backticks, quotes or a link: such text before the first `category:` is "text that belongs to no entry" (`realized_by_unrecognised`) and is a shape finding. A locator holds no whitespace, so a trailing explanation after the last locator (`test:a.rs because it covers x`) is a shape finding naming the stray text rather than a path that is silently wrong. A path that itself contains `,<category>:` is the one ambiguous shape, as it already is in the header.
6. **The waiver and its cite.** The waiver clause is `waived: <category>[ (<cite>[, <date>])] -- <reason>`. The category is in the declared vocabulary. The parenthesis, when present, is the cite: one record id in the type's identity form (`MILE-215`, `ADR-12`), then for a category declared `expires: required` an ISO date. The reason is required, non-empty prose after ` -- `. A category declared `cite: required` without a parenthesis, a cite that is not a record id, a date on a category that takes none, and a category declared `expires: required` without a date are shape findings. A pull request or an issue number is not a cite: it goes in the reason.
7. **Leading checkbox.** `[x]` or `[ ]` at the start of item text is a shape finding (state is computed, option E).

The shape findings are `status.done-items-shape`: a prose-only section, an item whose clause is not entirely locators, a misplaced or doubled clause, a leading checkbox, an unknown waiver category, an empty reason, a cite or date missing or malformed. A clause that does not parse is a finding, never skipped.

Worked examples (each is a fixture `MILE-214` fails before the change):

```markdown
## Done means

- A declared alias satisfies the section requirement and is reported as the alias used. — evidence: code:rust/crates/urzua-core/src/rules/section.rs#section_required, test:rust/crates/urzua-cli/tests/section_rules_integration.rs#an_alias_satisfies_the_requirement
```
Item 1: one item, two locators, the comma is followed by `test:` so it separates them.

```markdown
- Config is read from a directory whose name has a comma. — evidence: code:rust/tools/a,b/src/lib.rs, test:rust/tools/a,b/tests/t.rs
```
Item 2: the first comma is followed by `b/src`, not a category, so it stays in the path; the second is followed by ` test:` and separates.

```markdown
- The report prints evidence: per item, with no separator before the word.
```
Item 3: plain text. No clause, so on an opted-in type it is unresolved (no evidence).

```markdown
- Every rule that reads a section reads it through one function.
  - `section.required` calls it
  - `section.order` calls it
  - the harness text calls it

  evidence: code:rust/crates/urzua-core/src/rules/section.rs#sections_of, test:rust/crates/urzua-core/src/rules/section.rs#every_reader_shares_one_function
```
Item 4: one item; the three sub-bullets are its text, not three items; the evidence is on the final line.

```markdown
1. The release notes name the change. — waived: process-outcome -- the notes are written when the release PR is opened, see evidence: CI and #512
```
Item 5: an ordered item. The reason is `the notes are written when the release PR is opened, see evidence: CI and #512`; the `evidence:` inside it is not a marker.

```markdown
- A second platform builds. — evidence: test:rust/crates/urzua-cli/tests/platform_windows.rs#builds — waived: deferred-to (MILE-230, 2027-01-31) -- the runner is not available until MILE-230 lands
```
Item 6: evidence and waiver together. The waiver holds while the test does not resolve; the day it does, the waiver is stale (section 3).

```markdown
- The reader accepts `code:` locators. — evidence: `code:rust/crates/urzua-core/src/locator.rs`
- The reader is fast. — evidence: test:rust/crates/urzua-core/benches/read.rs because it measures it
- [x] The doc names the command. — evidence: spec:docs/specs/SPEC-2-urzua-check.md
```
Items 7, 8 and 9 are each a shape finding: backticks before the first `category:`; whitespace in the locator, naming `because it measures it`; a leading checkbox.

**2. Status follows the items.** For a type that declares the option, `judge_delivery` computes per item. An item is *resolved* when it has an evidence clause with at least one locator, every locator exists (the `embodiment.locator-exists` check, extended to item clauses), and its tier, computed by the existing `compute_embodiment` over only that item's locators, is in the item tier set `delivery.items.resolving_tiers` (default `delivered_tiers`). An item is *waived* when it carries a valid waiver clause and is not resolved. With `n` items, `r` resolved, `r_d` of them resolved at a tier in `delivered_tiers` (not merely `resolving_tiers`), and the *unresolved* items being the non-waived, non-resolved ones:

| Items | Computed standing | Tier reported | Status it agrees with |
|---|---|---|---|
| `r_d >= 1` and no unresolved item | delivered | the lowest tier among resolved items, so `Verified` only if every item is | the type's `set_status` (`Done`) |
| `r_d >= 1` and some unresolved item | partial | the same lowest tier, with the unresolved items named | `delivery.items.partial_status` (`InProgress`); also correct under `InProgress` |
| `r_d = 0` and `r >= 1` | specified only | the lowest resolved tier (`Specified`) | not forced either way: as a spec-only record is today, neither delivered nor unevidenced |
| `r = 0` (including `n = 0` and every item waived) | none | the declared `embodiment.none` (`Not started`) | `Planned`; also correct under `InProgress` (work without evidence yet is legitimate) |

Waivers excuse an item, they do not deliver one, so a milestone with every item waived has `r = 0`. `Specified` counts in `resolving_tiers` (decided, see Decisions below) so that a decision or documentation item is evidenced by a `spec:` locator, but the `r_d >= 1` guard keeps a `spec:` locator alone from computing a milestone delivered: an item such as "the capability is implemented" resolved by a `spec:` locator is exactly the false `Done` this ADR exists to stop. A spec-only milestone (a decision milestone) therefore behaves as it does today.

The disagreements extend `status.delivery-agreement` with variants (the rule table definition, which starts at about `rules/table.rs:1403`, gains them; `table.rs:1418` is the existing `delivery-not-done` action entry):

- delivered while the status is in `not_done`: reported, repaired by `fix` (unchanged in kind from `DeliveredButNotDone`, `status.rs:128-131`).
- partial while the status is `Planned`: reported, repaired by `fix` to `partial_status`.
- **`DoneWithUnresolvedItems`, a guaranteed error (new variant).** A `success` status while any item is unresolved, whatever `r` and `r_d` are. It is distinct from `SuccessWithoutEvidence`, which fires only when the tier is in `unevidenced_tiers` (`status.rs:184-186`) and so cannot see an item. The message contract is `<ID> is <Status> but Done means item <n> "<text>" is unresolved: <cause>` once per unresolved item, in item order, where `<cause>` is one of `no evidence clause`, `locator <path> does not exist`, `locator <path> resolves only to <tier>, which is not in resolving_tiers`. A milestone with `n = 0` or every item waived has no unresolved item and is reported by `SuccessWithoutEvidence` as `r = 0`.
  - *Severity.* The variant's severity is declared `error` on its entry in the rule table and is a floor: a repository's severity setting for `status.delivery-agreement` applies to the rule's other variants and cannot lower this one, and `urzua explain` prints the floor. An `error` finding fails `check` under any `fail_on`. Config load also rejects `items` on a type whose `status.delivery-agreement` rule is `off`.
  - *Not waivable by a rule-level waiver.* Waivers match findings by rule and scope, and the rule table now marks this variant `waivable: false`. A dated `WAIVER` record whose `Rule` is `status.delivery-agreement` does not match a finding of this variant, and the `permanent` declaration for that rule (`.urzua/config.yaml:412`, whose categories describe why a defect has no test) is not consulted for it. The waiver still matches the rule's other variants, so existing waivers keep their meaning (`WAIVER-0051` covers a delivered-while-`InProgress` finding and is unaffected). A waiver with no match is reported by `waiver.matches-nothing` as before. Only evidence (a resolving locator) or an inline item waiver (section 3) clears the finding, because those are the only things that change the item.
  - *Suggested actions (`ADR-73`).* The variant declares two, not the `Exempt` entry of `delivery-without-evidence` (`rules/table.rs:1419`), which would leave the finding with no action: `edit_field` on the type's status field with the value `partial_status` when `r_d >= 1` (otherwise the first declared status that is neither a `success`, `failure` nor `progress` status, `Planned` here, so a derived-blocked record is not offered a status `sequence.status-agreement` would refuse), the reopen; and `edit_section` on the Done means item, to add the evidence. Both are offered. Neither is applied by `fix`.
  - *`fix` never downgrades.* The `ADR-84` rule is unchanged: `fix` does not reopen a `Done` milestone, because absence of evidence is sometimes legitimate and only a person can say which. What changes is that this is now safe: `check` always fails on the variant, so the false `Done` is never silent. The repair is evidence, an inline waiver, or reopening by hand (`Done` to `Planned` or `InProgress` is already allowed by the `SPEC-6` transitions).

This maps onto `ADR-84` without new meaning: `delivered_tiers`, `unevidenced_tiers`, `not_done` and `set_status` keep their roles (`status_model.rs:95-113`), and the engine hardcodes no status name; `partial_status` is declared like `set_status`.

**3. Waiving an item.** An item is waived with the waiver clause of section 1. The categories are a declared vocabulary with the shape `ADR-86` gave the permanent waiver (`name`, optional `cite: required`, validated at load), plus an optional `expires: required` flag, and the reason is required prose. This repository declares four, none borrowed from `ADR-86`, because those describe why a defect has no test and none describes a missing criterion: `not-applicable` (the criterion no longer applies); `superseded-by-decision` (cite required: an ADR or RFC removed it); `deferred-to` (cite and expiry date required: the milestone that now carries it, so the work is not lost, and the day by which it must be re-checked); `process-outcome` (a result no file can evidence, such as a green CI run; the reason names the PR).

Why inline and not a separate waiver record: `ADR-86` rejected an exemption "written by the same hand, in the same file, as the claim it excuses" (its option C), and that argument is weaker here because the item is the claim and its line is reviewed in the PR that writes it. The record is the only place the criterion lives, so a waiver that lives elsewhere has nothing to point at: a `Scope` is one file path (`SPEC-10`), and a waiver scoped to the milestone would excuse every item at once (`ADR-86` option E's objection). Three safeguards replace the separate record. Every waived item is listed in the report, never omitted, and counted by category, as `ADR-86` counts permanent waivers. A `Done` milestone with a waived item is an informational notice on every run. And an inline waiver is as honest as `ADR-86`'s: it is checked every run by the stale check below, the counterpart of `waiver.matches-nothing`.

**The stale check, `status.waived-item-stale`** (a new rule, warning by default, so it fails this repository's CI under `fail_on: warning`; an ordinary `WAIVER` record can waive it like any warning). It reports, naming the record, item number and text, each waived item for which:

- *its evidence now resolves.* The item carries an evidence clause (section 1, rule 4) and that clause resolves as in section 2: the waiver excused an item that is now evidenced, so remove the waiver. This is how a waived item that later gains its locator is found, and why the waiver clause is allowed to sit beside the evidence it is waiting for.
- *its `deferred-to` target is gone or finished.* The cited record does not exist, or its status is in the target type's `success` or `failure` set (`Done`, `WontDo`, or the equivalent for another type). The work was either delivered there, in which case this item is evidenced by a locator here, or it was not carried, in which case the deferral was false; either way a person re-points the cite, evidences the item or reopens the milestone. A target that is open is a pointer the existing pointer rules already resolve; this check adds that it must also be unfinished. The ADR does not try to verify that the target's own items cover this one; that is a reviewer's reading, and the `MILE-215` migration list records it.
- *its expiry has passed.* A category declared `expires: required` carries a date, and the finding fires from that date on. `deferred-to` is inherently temporary, so it is never permanent: when the date comes the author either extends it with a new date (a diff a reviewer sees) or resolves it. This is the `Expires` of an `ADR-86` waiver, placed on the clause.
`superseded-by-decision` is held by the existing pointer rules (the cite must resolve). `not-applicable` and `process-outcome` have no machine-checkable staleness and are reported in the waived-items listing for a reviewer; the categories are few on purpose.

**4. Opt-in.** The option is `statuses.delivery.items` on a type. Absent, `judge_delivery` runs the code it runs today and every existing config is unchanged byte for byte in behaviour:

```yaml
delivery:
  delivered_tiers: ["Implemented", "Verified"]
  unevidenced_tiers: ["Not started"]
  not_done: ["Planned", "InProgress"]
  set_status: "Done"
  items:
    section: "Done means"            # must be a section the type requires
    clause_separator: " — "
    evidence_marker: "evidence:"
    waiver_marker: "waived:"
    partial_status: "InProgress"     # in values and not_done
    resolving_tiers: ["Specified", "Implemented", "Verified"]
    waiver_categories:
      - {name: not-applicable}
      - {name: superseded-by-decision, cite: required}
      - {name: deferred-to, cite: required, expires: required}
      - {name: process-outcome}
```

Config load errors, not inert rules (`BUG-44`): `items` on a type without `delivery`; a `section` the type does not declare; a `partial_status` outside `values` or `not_done`, or one with no declared transition from the statuses it repairs; a `resolving_tiers` entry that is not a declared embodiment state; a duplicate or empty separator, marker or category; a marker that is a prefix of the other; `items` on a type whose `status.delivery-agreement` rule is `off`; an `expires: required` category without `cite: required` (a deferral needs a target). This repository turns it on for `milestone` and no other type; the `bug` type, whose evidence is one regression test, is not itemised.

**5. The evidence field.** Item locators are evidence for every reader, not only for status. `Embodiment`, drift, `embodiment.locator-exists`, `fix --rebaseline` and the graph read one function that returns the union of the `Realized-by` field and the item clauses (`fix.rs`, `status.rs`, `embodiment.rs` and `diff_scope.rs` each parse the field today; the shared parser is the single seam, `embodiment.rs:48`). `Realized-by` stays valid for record-level evidence tied to no single criterion. Delivery is judged from the items only, so a record-level locator never makes a milestone delivered.

**6. `/done` and `fix --apply`.** Both compute from the items and neither writes a status the engine did not compute. `/done` (`.claude/hooks/urzua-record.sh:133-138`) adds evidence to the items the change delivered, then runs `urzua fix --apply --ids <record>`. If the engine computes nothing, the message now names the unresolved items instead of only "add the evidence" (the hook already stops when no `computed_value` is printed, `urzua-record.sh:139-143`). An agent that has finished three of five items runs `/done` and gets `InProgress`, not `Done`; it does not get to choose. `fix --apply` writes `Done` only when all non-waived items resolve, `InProgress` when some do, and its revision row records the counts ("3 of 5 items resolved"), so the attribution is the evidence. `fix --rebaseline` rewrites pins inside item clauses the way it rewrites them in the header (`MILE-181`). The `ADR-70` amendment holds unchanged: the status is computed from evidence, and the evidence is now finer.

**7. Partially delivered records, the roadmap and blocked.** Blocked stays derived from the written `Status` of the dependency (`sequence.rs:575-580`, `roadmap.rs:278-296`, `roadmap.rs:535`): a dependency is satisfied only in a `success` status, so a partly delivered milestone keeps its dependents blocked. The wait is item-blind on purpose; because `Status` is held to the items by `status.delivery-agreement` at error level, the two cannot disagree. No per-item dependency (`Depends-on: MILE-9#2`) is added (decided, question 3). The roadmap reads a partial record as open work: it stays in its wave and in the ready set if nothing it depends on is open, and the critical path does not change, since the roadmap weights nothing. `RoadmapRecord` (`roadmap.rs:101-116`) gains an optional `items` object, `{total, resolved, waived}`, present only for an opted-in type, so a reader sees "3 of 5" without a status the engine would have to know. One interaction needs a rule: `partial_status` is a `progress` status (`ADR-83`), so repairing a `Planned` milestone that depends on an open record to `InProgress` would create a `sequence.status-agreement` error. `fix` therefore does not write `partial_status` to a record that is derived-blocked; the finding stays and names the open dependency, because evidence landing ahead of its dependency is a fact the author should see.

**8. Migration of every milestone.** Built and landed separately from the engine: `MILE-214` is the engine and `MILE-215` (Phase 0, depends on `MILE-214`) is the migration, so the engine can land first and the opt-in is switched on only by the migration. The syntax is backwards compatible: an old rule ignores an item clause as text. So the conversion lands in batches, and the opt-in is switched on in the last PR only when `urzua check` is clean over every milestone. For each of the 127 Phase 0 `Done` milestones (and the 16 `Done` milestones of later phases, which the switch also covers; 143 in all):

1. Split the Done means into items, by hand with tool assistance (a split on semicolons and `and` is a draft, not the result; 105 are prose and 22 are lists).
2. Give each item the locators that evidence it, from the record's `Realized-by`, the merged PR (`git log -S`) and the repository.
3. Run `urzua check`. An item that does not resolve is verified against the repository by reading, then exactly one of: the claim is true and a locator is missing, so add it; the claim is not true, so reopen the milestone (`Done` to `InProgress`, a revision row naming the item and why); the claim is true and no file can evidence it, so waive it with a category and a reason.
4. A milestone is never kept `Done` for a false claim, and there is no blanket waiver. `WAIVER-0051` stays until the switch-on PR, which removes it: `MILE-165`, rewritten into items, computes `InProgress` from the engine.
5. Reviewed, not scripted: a reviewer reads each reopen and each waiver. Each migration PR carries a list of its reopens and its waivers, with the item and the reason, for that reader, and for each `deferred-to` whether the target's own items cover the deferred one.

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
- `SPEC-3` documents `statuses.delivery.items` and its load checks; `SPEC-2` the `status.done-items-shape` and `status.waived-item-stale` rules and the new `status.delivery-agreement` variants, `DoneWithUnresolvedItems` among them (error floor, `waivable: false`); `SPEC-6` the item grammar of section 1 verbatim, with its worked examples as fixtures, in its Required sections and its `Realized-by` row; `SPEC-8` the repairs (`Done`, `InProgress`) and that a `success` status is never downgraded. Each gets a revision row. The CLI changeset is minor (a new capability).
- The `milestone` template changes: `Done means` carries an example item line instead of "one or more bullets".
- `/done` and `/milestone` text (generated by `urzua harness`) says to name evidence per item; `urzua harness status` then reports drift until `harness update` is run.
- The report gains the waived items by category and a per-milestone item count; the roadmap gains the optional `items` object.
- Writing a milestone costs more: each criterion needs a locator or a waiver. That is the point; it moves the cost from a reviewer who must remember to ask to the author who must show.
- A milestone cannot be finished by its last PR alone if an earlier item had no evidence: that is the intended friction.
- Not decided here: per-item dependencies, weighting the critical path by items, itemising `bug` or other types, and a stable item id (items are numbered by position in reports, which breaks if an item is inserted).

## Decisions on the open questions

Answered on 2026-11-07 from the independent review of this ADR, as decided. The ADR itself stays Proposed.

1. **Inline waivers, with a stale check.** Decided: inline, on the conditions of sections 1 and 3: a cite grammar, an expiry on `deferred-to`, a `deferred-to` target that must exist and be unfinished, and `status.waived-item-stale` reporting a waived item that now resolves. Without those conditions it would have been a separate record.
2. **No auto-reopen.** Decided: `fix` never downgrades a `success` status. This stands because of the guaranteed error finding of section 2 (`DoneWithUnresolvedItems`), which `check` always fails on, which no rule-level waiver can clear and which offers the reopen as an action; the maintainer's intent is that nothing stays `Done` when it is not done, and the finding, not `fix`, is what holds that.
3. **No per-item dependencies.** Decided: the wait stays item-blind; a milestone that needs another's third item is split.
4. **`Specified` counts in `resolving_tiers`, with a guard.** Decided: a `spec:` locator resolves an item, but at least one item must resolve at a `delivered_tiers` tier (`r_d >= 1`) for the milestone to compute delivered or partial. A spec-only milestone stays as it is today.

## Amendment (2026-11-07): `urzua explain` has a rule mode that prints the severity floor and the waivable flag

*Status of this amendment: accepted by the maintainer on 2026-11-07. Found by the independent review of the `MILE-215` migration. `Status` stays `Accepted` and the Decision text above is unchanged.*

Section 2 says of `DoneWithUnresolvedItems` that "`urzua explain` prints the floor", and that the rule table marks the variant `waivable: false`. When the review found it, neither was true of `explain`: `urzua explain <path>` answers which records govern a file or scope (`SPEC-13`), takes a path, reads no rule, and its report held no severity floor and no waivable flag. Both were printed only by `urzua rules`, which lists every rule's variants with `severity_floor` and `waivable` (`commands/rules.rs`).

The first draft of this amendment proposed to read "`urzua explain` prints the floor" as "`urzua rules` prints the floor", and left a rule mode for `explain` as a separate milestone for the maintainer to ask for. The maintainer rejected that and chose the rule mode: `urzua explain --rule <rule-id>` prints, as JSON, the rule's `id`, `scope` and `level` in this repository, its `severity_floor` and whether it is `waivable`, its `options` with their current values, its `ADR-73` suggested-action `actions`, its `population_unit` and its `spec`. Section 2's wording is therefore accurate as written, and the Decision text is unchanged.

The rule is a flag, `--rule`, and not a second positional form of the existing argument. A path and a rule id cannot be told apart by their spelling: `status.delivery-agreement` is a legal file name, and a path may contain dots and no slash. A guess (a dot, a slash, an extension, whether the file exists) would read one as the other on some input, and a rule id read as a path answers "no record governs it", which is false assurance. Under the flag a bare argument is always a path and `--rule` always a rule, and giving both or neither is a usage error. An id the build does not ship is an error that names the nearest valid id.

The mode reads the rule table that `urzua rules` prints; it holds no second copy of a rule's variants, floors or options. `urzua rules` gains `scope`, `population_unit` and `spec` for the same reason: the table declares them once. `SPEC-13` specifies the mode, `SPEC-1` and `SPEC-2` carry the pointer, and the change ships as a minor changeset.

## Amendment (2026-11-07): dependencies cap the computed status

*Status of this amendment: accepted by the maintainer on 2026-11-07, including the skip of a capped record by `sequence.status-agreement`. Drafted at the maintainer's request after `MILE-215` switched the option on. `Status` stays `Accepted` and the Decision text above is unchanged, except where section 7 is read as this amendment says.*

**The defect.** With the option on, a milestone can have every Done means item resolved while a `Depends-on` target is unfinished. `sequence.status-agreement` (`ADR-83`) refuses `Done` for it, because a blocked record cannot claim progress, and `status.delivery-agreement` refuses `InProgress`, because every item resolves and the items say delivered. No status passes. `#419` worked around it by dropping `Depends-on` from `MILE-34` (on `MILE-171`) and from `MILE-215` (on `MILE-214`), and the revision rows of both say when to restore it.

**Decided.** Dependencies cap status. A record's computed status is the lower of the item result and the dependency result. A record whose items all resolve (the delivered standing of section 2) but which has an unfinished dependency in a declared acyclic pair computes the partial standing and `partial_status`, `InProgress` here. `status.delivery-agreement` accepts `InProgress` for it, and `DoneWithUnresolvedItems` does not fire on it, since no item is unresolved. Nothing else about the standing changes: the cap only lowers a delivered standing, and a record that is already partial or has no evidence computes as before.

**Unfinished is defined as `ADR-83` defines it.** A dependency is unfinished when it is not in a status its own type declares `success`. A dependency in a `failure` status, one with no `Status`, and one whose type declares no `success` are therefore unfinished. The dependencies are those over the pairs `roadmap.pairs` names, each an acyclic `relation_pairs` entry (`ADR-76`), read in either field. A `Depends-on` value that resolves to nothing is `sequence.unresolved-blocker`'s and caps nothing. The cap and `sequence.status-agreement` read one function, so they cannot disagree about which records are blocked.

**Where it applies.** Only where both are declared: the type declares `statuses.delivery.items` and the repository declares a sequencing pair in `roadmap.pairs`. A configuration with neither is unchanged, and no new configuration key is added.

**Naming the dependency.** The `delivery-partial-not-started` finding says that every item resolves and which dependency caps the record (`MILE-171 (InProgress)`). `urzua fix` repairs a `Planned` capped record to `InProgress`, never to `Done`, and prints a notice for a capped record naming the dependency; a `success` status is not downgraded by `fix` (section 2, question 2), and the derived blocked value (`sequence.status-agreement`) is the finding that holds a `Done` milestone with an unfinished dependency.

**The derived blocked value skips a capped record.** `sequence.status-agreement` (`ADR-83`) refuses a `progress` status for a blocked record, and `InProgress` is a `progress` status in this repository's `milestone` type. A record whose `InProgress` comes from the cap has not made progress out of order: all its items were proven ahead of the dependency, and only the dependency holds it below `Done`. So `sequence.status-agreement` skips such a record. It is skipped when its status is the type's `partial_status`, every item resolves, and the dependency is the only thing capping it; it gets an info notice, `all items proven ahead of dependency X` (`X` is the open dependency with its status, as `MILE-171 (InProgress)`), and no error. A record genuinely in progress out of order, where an item is unresolved, is still reported, and so is a capped record stating `Done`. The type's `statuses.progress` is unchanged: `InProgress` stays listed there, so out-of-order detection stays on for every other `InProgress` milestone. The maintainer chose this on 2026-11-07, over removing `InProgress` from `progress`, which would have turned that detection off for every milestone.

**What this changes in section 7.** Section 7 reads `partial_status` as a `progress` status, so `fix` does not write it to a derived-blocked record. That now holds only for a derived-blocked record with an unresolved item. For a capped record `fix` writes `partial_status`, because `sequence.status-agreement` accepts it. The wait stays item-blind (question 3): no per-item dependency is added.

**Consequences.** `MILE-34` and `MILE-215` regain their `Depends-on` and compute `InProgress`. `SPEC-2`, `SPEC-3` and `SPEC-6` document the cap, and `ADR-83` carries the matching amendment. It is a patch to the CLI: it corrects a state no configuration could satisfy.

**A milestone declares its kind.** *Accepted by the maintainer on 2026-11-07, as a further decision of the same date; added to this section because the repository keeps one amendment section per date. The Decision text above is unchanged.*

*The gap.* Section 2 requires at least one item resolved at a `delivered_tiers` tier (`Implemented` or `Verified` here) before a record computes `Delivered`. A decision milestone, `MILE-73` "Decide when a spec needs a new spec" for one, is evidenced by records at the `Specified` tier, so it computes `SpecifiedOnly` however many of its items are decided, and `urzua fix` never writes `Done` for it.

*Decided.* The milestone declares its kind, and the kind says which tiers count as delivered. The type declares the field and the vocabulary under `statuses.delivery.items.kind`: `field` (the header field, `Kind` here) and `kinds`, each with a `name`, an optional `delivered_tiers` and an optional `cited_records`. A kind's `delivered_tiers` replaces the type's for the section 2 rule, the `resolved_delivered >= 1` guard and the `Delivered` and `Partial` standings; a kind that omits it uses the type's. This repository declares two kinds. `decision` counts `Specified`, `Implemented` and `Verified`, so a decision milestone is done when every item resolves at `Specified` or higher. `build` names nothing and keeps today's rule, as does a milestone that declares no kind. The feature is opt-in: a configuration with no `kind` key is unchanged, byte for byte.

*`Specified` means decided.* The `Specified` tier is computed from a locator's category alone: any `spec:` locator that resolves lifts an item to it, and `compute_embodiment` reads no record's status. A Proposed ADR or a Draft spec would therefore evidence a decision milestone. `cited_records: success` closes that: a locator that resolves to a record counts only while that record's status is one its own type declares `success` (here an Accepted ADR, an Accepted RFC, and an Accepted or Shipped spec). A Proposed, Rejected, Superseded or Draft record does not count, and the item's cause names the record and its status, as `locator docs/adr/ADR-2-x.md is ADR-2 at status Proposed, which its type does not declare a success status`. A locator that is not a record (code, a test, a plain document) is not judged by status, and a kind that omits `cited_records` reads no record's status. The rule reads the type's declared `success` list and names no status.

*What the engine reports.* A `Kind` value outside the declared vocabulary judges the record as one with no kind, and `status.done-items-shape` reports it as an error naming the allowed kinds, so the label is never inert. A declaration is refused at load when its `field` is blank or not a field the type declares, when `kinds` is empty or lists a name twice, and when a `delivered_tiers` entry is not a declared tier, is not in `resolving_tiers` (no item could resolve at it) or is in `unevidenced_tiers`.

*Consequences.* `SPEC-2`, `SPEC-3` and `SPEC-6` document the key, the judgement and the finding. This repository sets `Kind: decision` on the milestones that record a decision and build nothing (`MILE-11`, `MILE-12`, `MILE-13`, `MILE-47`, `MILE-48`, `MILE-50`, `MILE-53`, `MILE-73`, `MILE-77`, `MILE-117`) and leaves the milestones that decide and also build (`MILE-10`, `MILE-14`, `MILE-126`, `MILE-127`, `MILE-128`, `MILE-129`) as builds. It ships as a minor changeset: a new capability, off until declared.

## Amendment (2026-11-08, placeholder items): an item can be declared unfilled

*Status of this amendment: accepted by the maintainer on 2026-11-09. It was first decided by the orchestrator under the maintainer's standing opt-in instruction of 2026-11-08 that new capabilities are opt-in config. `Status` stays `Accepted` and the Decision text above is unchanged.*

*The gap.* A milestone scaffolded from the milestone template holds placeholder list items under Done means ("Replace this item with ..."). Each is a list item with no evidence clause, so it reads as an unresolved item, but `urzua check` flags it only for a record whose status claims Done; a Planned milestone passes with nothing reported. No mechanism reads placeholder text in an item: `pending` on a status declaration and the `template_placeholders` vocabulary read header values only.

*Decided.* `statuses.delivery.items` takes an optional `pending` list, in the shape and with the matching of the existing `pending` vocabulary (a whole value, compared trimmed and case-insensitively, or `{pattern: "..."}` searched in the trimmed text). An item whose text matches is an unfilled placeholder. It never resolves: its evidence and waiver clauses are dropped, so it is not evidence and cannot be waived, and a record holding one cannot compute a delivered standing. `status.done-items-shape` reports it as an error on the item's line, with that rule's `edit-section` action.

*Why `status.done-items-shape`.* The rule judges the shape of the Done-means section and already reports a prose-only section and a leading checkbox, both of the kind "this is not a real item". `status.delivery-agreement` judges status against evidence, and a Planned record has no disagreement to report. Keeping the finding in the shape rule also makes it independent of the record's status, which is the gap.

*Opt-in.* Omitting `pending` keeps today's behaviour byte for byte. A declaration with a blank entry, a duplicate or an invalid pattern is refused at load. This repository declares the placeholder text of its milestone template.

*Caveat.* Whole-value entries match case-insensitively, but `{pattern: ...}` entries are case-sensitive regexes, and a real item that begins with a declared pattern is flagged as a placeholder, so patterns should be specific to the template's wording.

*Consequences.* `SPEC-2` and `SPEC-3` document the key and the finding. It ships as a minor changeset: a new capability, off until declared.

## Amendment (2026-11-14, scope-changed-by-decision): an item whose scope the maintainer changed is waived with the decision quoted

`process-outcome` is "a result no file can evidence", which does not describe an item that is unmet as written because the maintainer changed what was wanted. This repository declares a fifth category, `scope-changed-by-decision` (no cite and no expiry, because the decision is not always a record). Its reason quotes the maintainer's instruction or cites the pull request or review where they gave it, says the item is unmet as written, and the item keeps its original text. The engine treats it like `not-applicable` and `process-outcome`: it has no machine-checkable staleness and is listed in the waived-items report, so a reviewer reads the quote. The Decision text is unchanged. Decided by the orchestrator on 2026-11-14, pending maintainer review.

## References

- `ADR-84` -- a record's status follows its declared evidence field (the mechanism extended); `ADR-83` -- blocked is derived only, `progress` statuses; `ADR-86` -- the permanent-waiver vocabulary shape reused for item waivers.
- `ADR-70` and its amendment of 2026-11-05 -- a lifecycle command sets a status only when the engine computes it from evidence; `ADR-89` -- lifecycle commands and `authority` gates.
- `ADR-64` -- structured in, structured out; `ADR-74` -- locator resolvers and pins; `ADR-82` -- the roadmap.
- `SPEC-2`, `SPEC-3`, `SPEC-6`, `SPEC-8`, `SPEC-10`.
- `MILE-165`, `WAIVER-0051` -- the instance; `MILE-183` -- the delivery model this refines; `MILE-181` -- re-pinning; `MILE-214` -- the milestone that builds the engine; `MILE-215` -- the migration of the corpus, which depends on it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Drafted as Proposed. **Why:** maintainer decision of 2026-11-07, after confirming as an engine defect that `judge_delivery` computes delivered from the evidence tier alone, so a milestone with one item of five evidenced is forced to `Done`; the maintainer approves before it is Accepted. | **substantive** |
> | 2026-11-07 | Amended while Proposed, from the independent review of the same day (three blockers). **Why:** (1) the ADR now names the guaranteed error finding `DoneWithUnresolvedItems`, gives its message contract, an error floor, `waivable: false` (the rule-level `permanent` category does not apply to it) and two ADR-73 actions in place of `Exempt`; (2) the item syntax is a grammar with worked examples: nested lists, a single separator-plus-marker form, the Realized-by comma rule, the waiver cite; (3) inline waivers get ADR-86's honesty: `status.waived-item-stale`, a checked and expiring `deferred-to`. The four open questions are recorded as decided (inline waivers with the stale check; no auto-reopen; no per-item dependencies; `Specified` in `resolving_tiers` with an `r_d >= 1` guard), and the migration is split out as `MILE-215`. Status stays Proposed; the maintainer approves before it is Accepted. | **substantive** |
> | 2026-11-07 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-07. | **substantive** |
> | 2026-11-07 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Embodiment` from `no Realized-by field; cited by source back-pointers` -- stated no value disagreed with the computed value `Implemented`. | **structural** |
> | 2026-11-07 | Amendment added, proposed and pending maintainer acceptance: the severity floor and `waivable: false` of `DoneWithUnresolvedItems` are printed by `urzua rules`, not `urzua explain` as section 2 says. **Why:** a reviewer of the `MILE-215` migration found `explain` takes a path and shows neither; making it show them is a new input form on a public command, so the correction is recorded here for the maintainer to accept or to turn into a milestone. The Decision text is unchanged. | **structural** |
> | 2026-11-07 | Amendment accepted by the maintainer on 2026-11-07, replacing the pending text: `urzua explain --rule <rule-id>` prints the rule's scope, level here, severity floor, waivable flag, options with current values, `ADR-73` action variants, population unit and spec, from the rule table `urzua rules` reads. The amendment's earlier text, which redirected to `urzua rules`, was rejected and is recorded in the row above. **Why:** maintainer decision of 2026-11-07: the floor belongs on `explain` as section 2 says, so the engine is changed to match the ADR and not the ADR to match the engine. The Decision text is unchanged. | **structural** |
> | 2026-11-07 | Amendment added, accepted by the maintainer on 2026-11-07: dependencies cap the computed status. A record whose items all resolve but which has an unfinished dependency in a declared acyclic pair computes `InProgress`; `status.delivery-agreement` accepts it and `DoneWithUnresolvedItems` does not fire on it; the finding names the capping dependency; the cap applies only where both `delivery.items` and a sequencing pair are declared. **Why:** maintainer decision of 2026-11-07, after the switch-on of `MILE-215` left a milestone with every item resolved and an unfinished `Depends-on` target no status that both `sequence.status-agreement` and `status.delivery-agreement` accept. The Decision text is unchanged. | **substantive** |
> | 2026-11-07 | Amendment extended, still accepted by the maintainer on 2026-11-07: `sequence.status-agreement` skips a record whose `InProgress` status comes from the dependency cap (every item resolves, only an unfinished dependency holds it back) and emits an info notice, `all items proven ahead of dependency X`; a record genuinely in progress out of order is still flagged. `statuses.progress` of this repository's `milestone` type is unchanged. **Why:** maintainer decision of 2026-11-07, over removing `InProgress` from `progress`, which would have turned off out-of-order detection for every `InProgress` milestone. The Decision text is unchanged. | **substantive** |
> | 2026-11-07 | Amendment extended, accepted by the maintainer on 2026-11-07: a milestone declares its kind. The type declares a field and a vocabulary under `statuses.delivery.items.kind`; each kind names the tiers that count as delivered, so a `decision` milestone is done when every item resolves at `Specified` or higher, and `build` (or no kind) keeps the delivered-tier rule; `cited_records: success` makes a cited record count only at a `success` status of its own type, so a Proposed ADR or a Draft spec does not. Opt-in; an unknown kind is a finding. **Why:** maintainer decision of 2026-11-07, after `MILE-73` showed that a decision milestone evidenced at the `Specified` tier can never compute `Done` under section 2. The Decision text is unchanged. | **substantive** |
> | 2026-11-08 | Amendment added, decided by the orchestrator under the maintainer's standing instruction of 2026-11-08 that new capabilities are opt-in config; pending maintainer acceptance: `statuses.delivery.items.pending` declares placeholder item texts. An item that matches never resolves, cannot be waived, and is reported by `status.done-items-shape` on its line. Opt-in; undeclared keeps today's behaviour. **Why:** a milestone scaffolded from the template passed `urzua check` with its placeholder Done means items unflagged. The Decision text is unchanged. | **substantive** |
> | 2026-11-09 | Amendment of 2026-11-08 (placeholder items) accepted. **Why:** Maintainer decision, 2026-11-09. | **substantive** |
> | 2026-11-14 | Amendment added, decided by the orchestrator on 2026-11-14, pending maintainer review: this repository declares a fifth waiver category, `scope-changed-by-decision`, for an item whose scope the maintainer changed; its reason quotes the decision and says the item is unmet as written. Used on `MILE-49` and `MILE-165`. The Decision text is unchanged. **Why:** an external review found `process-outcome` a poor fit for "not met as written, by maintainer decision". | **substantive** |
