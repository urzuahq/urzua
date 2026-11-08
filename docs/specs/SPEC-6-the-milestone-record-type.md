---
Stable-Id: 01M6R3NK5QF32RM76Y50THPQEG
Version: '0.19'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/status_model.rs@sha256:744b341a578a23b1682b8d176147a340bdab44f3b0736d282295f3c039758094, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs@sha256:7caa2397549099e9e22e83bed990459dc5318b64a2dea6d5770e9b3929301c1e, test:rust/crates/urzua-cli/tests/status_delivery_integration.rs@sha256:3e956fb6227ce9dd6f5fcc614a871d8c20ace9a4ba0169eaf3fee431b4e176a9
Author: beauwilliams
Subject: 'The `milestone` record type -- this project''s own planned-work backlog as a checkable corpus.'
Implements: ADR-34
Parent: —
---
# SPEC-6 — The `milestone` record type

## Purpose

Tracks this project's own backlog — decided-but-unbuilt work, explicitly deferred work, undecided
RFCs, and unrouted research findings — as a real, checkable corpus instead of conversation history
and an unstructured document. `milestone` is the first record type this project has added
beyond ADR/RFC/Spec, and exercises RFC-4's generic layering claim directly: if the mechanism
built for three types doesn't hold for a fourth, that's real evidence, not a hypothetical.

## Schema

```yaml
record_types:
  milestone:
    dir: "docs/milestones"
    prefix: "MILE"
    required_fields: ["Status", "Phase", "Track"]
    header_shape: "yaml-frontmatter"
    known_fields: ["Stable-Id", "Implements", "Blocked-on", "Depends-on", "Blocks", "Realized-by", "Non-reference-ids"]
    revision_log: {marker: "**Revision log**", required: true}
    statuses:
      values: ["Planned", "InProgress", "Done", "WontDo"]
      success: ["Done"]
      failure: ["WontDo"]
      progress: ["InProgress", "Done"]
      implementable: ["Planned", "InProgress", "Done"]
      initial: ["Planned", "Done"]
      transitions:
        Planned: ["InProgress", "Done", "WontDo"]
        InProgress: ["Planned", "Done", "WontDo"]
        Done: ["Planned", "InProgress"]
      delivery:
        delivered_tiers: ["Implemented", "Verified"]
        unevidenced_tiers: ["Not started"]
        not_done: ["Planned", "InProgress"]
        set_status: "Done"
    sections:
      from: headings
      required:
        - {name: "Done means"}
      exempt_statuses: ["WontDo"]
    spec: "SPEC-6"
    pointer_fields: ["Implements", "Depends-on", "Blocks"]
    narrative_fields: ["Blocked-on"]
```

The `statuses` block is what makes `Status` a closed, checked vocabulary: `success` and `failure` say which
values finish a milestone, `progress` which claim work has started, `implementable` at which values code
may cite the milestone, `initial` the values a milestone may be filed with, and `transitions` the moves
between values. `delivery` holds `Status` to the evidence (`MILE-183`, below). `Depends-on` and `Blocks`
are the two ends of the repository's `sequencing` relation pair (`relation_pairs` and `roadmap` in the
same file, `SPEC-2`); `Non-reference-ids` lists ids the text names without citing a record (`ADR-75`).

`prefix` (2026-09-07 addition) decouples the type's own name — used for the `urzua new milestone
...` argument and required-fields lookup — from its filename/ID prefix, the same way `dir` already
decouples the type name from its directory name. Omitted, it defaults to the type name upper-cased
(`MILESTONE`); declared, `urzua new` emits `MILE-N-slug.md` and `check` recognizes that shape.
Per-record-type, not global — any type can shorten its own prefix without affecting the others.

| Field | Values | Notes |
|---|---|---|
| `Status` | `Planned` \| `InProgress` \| `Done` \| `WontDo` | The realization axis for a milestone — orthogonal to whatever `Embodiment` on the RFC/ADR it implements is doing. `WontDo` is terminal, for work deliberately deferred or decided against — distinct from a milestone that is waiting (a specific, nameable blocker exists and the work is still intended), which is not a status: `Blocked` is derived from `Depends-on` and never written (`RFC-48`, `ADR-83`), so a waiting milestone stays `Planned` (or `InProgress` while it has started, which `sequence.status-agreement` reports if it is also blocked) and from leaving a milestone `Planned` (which misrepresents a reversed decision as still-pending work). Precedent: `bug`'s `WontFix`, `adr`'s `Rejected`/`Superseded` — every other record type in this corpus already has a terminal "decided against" state; milestone didn't until this version. |
| `Phase` | a plain tag (`0`, `1`, ...) | Sequential grouping. Not a resolved pointer — no cross-referencing, just a bucket. `urzua roadmap` groups by it (`roadmap.group_by`). |
| `Track` | a plain tag (`header-format`, `section-checks`, ...) | Parallel workstream. Also a plain tag, not a pointer; the roadmap groups by it within a phase. |
| `Kind` | `build` or `decision` (the vocabulary `statuses.delivery.items.kind` declares); optional | What the milestone is, so what counts as done (`ADR-96` amendment of 2026-11-07). `decision`: every Done-means item cites a decided record or evidence above one. `build`, or no `Kind`: the delivered-tier rule. Declared only where the type declares `delivery.items.kind`; see "Done means as items". |
| `Depends-on` | comma-separated, optional | Records that must finish before this milestone can: milestones, or conditions (`SPEC-23`). A pointer field, so each value must resolve (`sequence.unresolved-blocker`) and the relation must not loop (`sequence.cycle`). A milestone with an unfinished `Depends-on` target is blocked; `Blocked` is derived and never written as a `Status`, and `sequence.status-agreement` reports a blocked milestone whose `Status` claims progress. `urzua roadmap` orders the backlog by it. |
| `Blocks` | comma-separated, optional | The inverse of `Depends-on`, declared so a record that writes it still takes part; it is otherwise computed from the records and never written. |
| `Implements` | comma-separated, optional | Already-generic field (RFC-1) — points at whichever RFC(s)/ADR(s) this milestone realizes. **Not required**: a milestone can exist before a decision does (e.g. "decide whether this needs an RFC"). Declared as this type's one `pointer_fields` entry (MILE-90/ADR-44) — clean, comma-separated references only, format-enforced by `header.pointer-field-clean`. |
| `Realized-by` | `spec:`, `code:`, `test:` locators; set by the delivering PR | Delivery evidence (`MILE-183`). Under `statuses.delivery.items` (`ADR-96`) delivery is judged from the Done-means items instead, and this field holds only record-level evidence. The PR that fully delivers a milestone sets it and `Status: Done` together; `status.delivery-agreement` fails when the evidence computes to `Implemented` or `Verified` under `Planned` or `InProgress`, and when `Done` has no evidence. A decision milestone's evidence is the record of the decision (a `spec:` locator), which is neither. Leave it off a milestone that is only partly delivered. |
| `Blocked-on` | free text, optionally citing a record ID; optional | What has to be true before this milestone can move. Free text with no reference token stays legal ("a decision not yet made"); a real ID (`BUG-3`) is checked by `pointer.resolution` for resolution and by `narrative-field.stale` (ADR-42, generalized by MILE-90/ADR-44) for whether the target has since reached a terminal status. Declared as this type's one `narrative_fields` entry (MILE-90/ADR-44) — prose-tolerant, unlike `Implements`. Was a body section (`## Blocked on`) before this version — moved into the header (ADR-42) since it's the one field-shaped exception in the schema that hadn't been. |

`Status` is mechanically validated against the declared `statuses`: `status.undeclared-value` reports a
value outside the four, `status.transition-allowed` reports a move the `transitions` graph does not
allow (on a `--base` run, against the merge base's copy), `status.unreachable-state` reports a declared
value nothing reaches, and the `initial` set is the values a new milestone may carry. A milestone is
filed `Planned`, or `Done` when the PR that files it delivers it; `Done` can be reopened to `Planned` or
`InProgress`; `WontDo` is terminal. Which values are `success` and `failure` is this declaration's, not
the engine's.

History is the existing revision-log model (RFC-6), reused unmodified: a milestone's `Status`
transitions are recorded as dated, classified revision-log entries on the milestone record itself,
not a separate mechanism.

## Required sections

Every milestone, open or closed, carries a `## Done means` section, declared under the type's `sections`
in `.urzua/config.yaml` and enforced at `error` by `section.required` (`SPEC-3`). It states the concrete,
checkable criteria that make the milestone done: the command, rule or file, and what it reports. An open
milestone derives them from its What and Why; a `Done` milestone states the criteria its delivery met, so
"done" is judged against them and not against the What, and a closure that does not meet its What is
reopened rather than left `Done`. Where it fits, the criteria include that the capability is exercised in
the repository's own configuration and records, not only by unit tests. A `WontDo` milestone is exempt
(`exempt_statuses`): it will never be done, so it has no criteria to meet. The template
(`.urzua/templates/milestone.md`) carries the section, and `/milestone` fills it.

## Done means as items (`ADR-96`, `MILE-214`)

A type may declare `statuses.delivery.items` (`SPEC-3`). Its `## Done means` is then a list whose items
each name their own evidence, and the milestone's `Status` is computed from the items. This repository
declares it (`MILE-215` migrated the milestones and switched it on); the grammar below is the
one the engine implements. It is structured text, not a mini-language: the engine reads three declared
words and hands each clause to the `Realized-by` and waiver parsers.

A dependency caps the status the items compute (`ADR-96`, amendment of 2026-11-07): a milestone whose items all
resolve but whose `Depends-on` target is unfinished (not `Done`) computes `InProgress`, not `Done`, and the
findings name the dependency. `sequence.status-agreement` skips that `InProgress` with an info notice (`all items
proven ahead of dependency X`); an `InProgress` milestone with an unresolved item is still flagged.

A milestone declares its kind (`ADR-96`, amendment of 2026-11-07). This repository's `Kind` field takes
`build` or `decision`. A `decision` milestone records a decision and is done when every item resolves at
`Specified` or higher, where an item that cites a record counts only while that record has reached a `success`
status of its own type (an Accepted ADR or RFC, an Accepted or Shipped spec): a Proposed or Draft record
does not. A `build` milestone, and one with no `Kind`, is done under the delivered-tier rule above. The
field is optional and its absence changes nothing; a value outside the vocabulary is an error
(`status.done-items-shape`).

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

`SEP` is `clause_separator` (` — `, space em dash space), `EV` is `evidence_marker` and `WV` is
`waiver_marker`. The rules:

1. **What is an item.** A list item at the item indent, whether the marker is `-`, `*`, `+`, `1.` or
   `1)`. A list item indented deeper than its predecessor at the item indent is a sub-item: it belongs
   to that parent, its text is part of the parent's item text, and it is never itself an item and never
   carries a clause. A marker indented less than the item indent is a shape finding. Items are numbered
   by position, from 1, in report messages. A section with no list item (prose only) has `n = 0`; on an
   opted-in type that is a shape finding, and under a `success` status it is also the
   delivery-agreement finding (`r = 0`).
2. **Where the clauses are.** In exactly one place per item: at the end of the item's first line, or on
   the item's final line or final two lines. The first line is split at the first occurrence of
   `SEP + EV` or `SEP + WV`; the text before it is the item text, and continuation lines and sub-items
   after the first line add to the item text. The final-line form exists for an item whose text runs
   over several lines or has sub-items: after the last sub-item or continuation line (a blank line
   between is allowed), the evidence clause is one line beginning with `EV` and the waiver clause is one
   line beginning with `WV`, in that order, each indented as a continuation. Clauses on the first line
   and on final lines at once are a shape finding. A marker anywhere else, including inside a
   continuation line or a sub-item, is plain text and is not parsed.
3. **A marker inside prose.** `evidence:` without the separator before it is text (an item may say "the
   report prints evidence: per item"); the clause begins only at `SEP + marker`. A waiver's reason runs
   to the end of its line and is free prose, so `waived: process-outcome -- see evidence: CI` has one
   reason, `see evidence: CI`, and `— evidence:` inside a reason is part of the reason. The converse is
   the one thing an author must avoid: item text that itself contains `SEP + EV` splits there, and the
   clause that follows then fails to parse as locators, which is a shape finding naming the item. It is
   loud, never silent.
4. **Both clauses.** An item may carry an evidence clause and a waiver clause, evidence first. That is
   how a waiver stays honest: the item says what will evidence it, and the waiver excuses it until it
   does. A waiver without an evidence clause is valid.
5. **Several locators.** The evidence clause is read by the `Realized-by` splitter: a comma separates
   two locators only when a `category:` prefix (`spec:`, `code:`, `test:`) follows it, optionally after
   one space; any other comma belongs to the path. Scoped symbols and `@sha256:` pins mean what they
   mean in the header. A locator is not wrapped in backticks, quotes or a link: such text before the
   first `category:` is text that belongs to no entry and is a shape finding. A locator holds no
   whitespace, so a trailing explanation after the last locator (`test:a.rs because it covers x`) is a
   shape finding naming the stray text. A path that itself contains `,<category>:` is the one ambiguous
   shape, as it already is in the header.
6. **The waiver and its cite.** The waiver clause is `waived: <category>[ (<cite>[, <date>])] -- <reason>`.
   The category is in the declared vocabulary. The parenthesis, when present, is the cite: one record id
   in the type's identity form (`MILE-215`, `ADR-12`), then for a category declared `expires: required`
   an ISO date. The reason is required, non-empty prose after ` -- `. A category declared
   `cite: required` without a parenthesis, a cite that is not a record id, a date on a category that
   takes none, and a category declared `expires: required` without a date are shape findings. A pull
   request or an issue number is not a cite: it goes in the reason.
7. **Leading checkbox.** `[x]` or `[ ]` at the start of item text is a shape finding: state is computed.

Worked examples, each a fixture (`done_items/tests.rs` and `done_items_integration.rs`):

```markdown
## Done means

- A declared alias satisfies the section requirement and is reported as the alias used. — evidence: code:rust/crates/urzua-core/src/rules/section.rs#section_required, test:rust/crates/urzua-cli/tests/section_rules_integration.rs#an_alias_satisfies_the_requirement
```

Item 1: one item, two locators, the comma is followed by `test:` so it separates them.

```markdown
- Config is read from a directory whose name has a comma. — evidence: code:rust/tools/a,b/src/lib.rs, test:rust/tools/a,b/tests/t.rs
```

Item 2: the first comma is followed by `b/src`, not a category, so it stays in the path; the second is
followed by ` test:` and separates.

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

Item 5: an ordered item. The reason is `the notes are written when the release PR is opened, see evidence: CI and #512`;
the `evidence:` inside it is not a marker.

```markdown
- A second platform builds. — evidence: test:rust/crates/urzua-cli/tests/platform_windows.rs#builds — waived: deferred-to (MILE-230, 2027-01-31) -- the runner is not available until MILE-230 lands
```

Item 6: evidence and waiver together. The waiver holds while the test does not resolve; the day it does,
the waiver is stale (`status.waived-item-stale`, `SPEC-2`).

```markdown
- The reader accepts `code:` locators. — evidence: `code:rust/crates/urzua-core/src/locator.rs`
- The reader is fast. — evidence: test:rust/crates/urzua-core/benches/read.rs because it measures it
- [x] The doc names the command. — evidence: spec:docs/specs/SPEC-2-urzua-check.md
```

Items 7, 8 and 9 are each a shape finding: backticks before the first `category:`; whitespace in the
locator, naming `because it measures it`; a leading checkbox.

The `Realized-by` field stays valid for record-level evidence tied to no single criterion. For an
opted-in type its locators are evidence for `Embodiment` and drift beside the item locators, but never
make the milestone delivered.

## Phase vocabulary

`Phase` stays a plain tag with no mechanical check, but this project uses one fixed vocabulary so a
phase means the same thing on every record:

| Phase | Meaning |
|---|---|
| `0` | Bootstrap: required for the tool to validate and build itself further, plus urgent bugs and the single Phase 0 exit (`SPEC-1`, Exit bar): governing this repository in CI with zero standing warnings, and replacing one adopter's hand-written linter (`MILE-102`). |
| `1` | Worth installing. |
| `2` | Agent-facing. |
| `3` | Differentiated. |
| `4` | Scale. |

A milestone moves phase by an edit to `Phase` and a revision-log row saying why, the same as any other
substantive change. Another repository using this type may define its own phases; these are this
project's.

## Why no new fields for ordering or grouping

A record's display number is identity, not priority (ADR-3) — `MILE-7` existing does not
mean it happens after `MILE-6`, the same as every other record type. `Phase`/`Track` are
plain tags rather than resolved relationship fields because they're categorization, not references
to another record — nothing to check for resolution, no cycle risk, no new rule needed.

## What's deliberately not built

- **A `Blocked-by` field, and reciprocity on `Depends-on`.** The sequencing relation is built as
  `Depends-on` with the computed inverse `Blocks` (`reciprocity: none`, `acyclic: true`), so only one end
  is ever written and the relation is checked for loops and unresolved targets. A second hand-written
  end would only add a way for the two to disagree. Distinct from `Blocked-on` (above): that field is
  one-way prose, checked for resolution and staleness, never for cycles.
- **A hand-maintained roadmap document.** The roadmap is generated: `urzua roadmap` computes the
  waves, the ready set and the critical path from the records, grouped by `Phase` then `Track`
  (`ADR-82`), so it cannot drift from them.
- **Cross-type cycle prevention and per-profile "allowed parent types"** — named as open questions
  on RFC-4 itself (this milestone type is what surfaced the gap), not scoped to milestone
  specifically since they'd apply to every record type's `Implements`/`Derives-from` use.

## References

- ADR-34 — the decision this spec details: `milestone` as a configured type, zero `urzua-core`
  changes.
- ADR-11 — the `waiver`-as-configured-type precedent this follows.
- ADR-3 — display-number-is-identity, the reason `Phase`/`Track` are tags, not numbering.
- RFC-4 — the layering model this record type is the first real test of beyond three types.
- RFC-6 — the revision-log model reused for milestone history.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec. | **structural** |
> | 2026-09-07 | Added `prefix = "MILE"`, shortening the filename/ID prefix from `MILESTONE-N` to `MILE-N`. The type name (`milestone`) and directory (`docs/milestones`) are unchanged. | **substantive** |
> | 2026-09-07 | Added `WontDo` as a terminal `Status` value (ADR-34 amendment). **Why:** every other record type in this corpus (`bug`, `adr`) already has a terminal "decided against" state; `milestone` didn't, so deferring or abandoning a milestone had no honest representation — it either stayed `Planned` (misrepresenting a reversed decision as still-pending) or `Blocked` (implying a specific blocker that may not exist). Folded into this version rather than a new spec number, per ADR-14's amendment: a spec's number is permanent per subject, revisions are a version bump on the same spec. | **substantive** |
> | 2026-09-07 | Moved `Blocked-on` from a body section (`## Blocked on`) into the header, alongside `Implements` (ADR-42). **Why:** found live that `Blocked-on` was the one field-shaped exception across the entire schema left unchecked as prose -- MILE-2/3 both cited an already-fixed bug by description with nothing catching it going stale. All 84 existing milestone files migrated, verified by a before/after content-equality check. | **substantive** |
> | 2026-09-08 | Header shape changed from `blockquote` to `yaml-frontmatter`, `header_layout` removed (ADR-33/38 amendments); `spec = "SPEC-6"` declared in config, closing `type.no-declared-spec`'s live finding for this type (ADR-43). **Why:** ADR-33's migration scope widened to all six configured types the same day; `header_layout` has nothing left to distinguish once no type declares `Blockquote`. | **substantive** |
> | 2026-09-08 | Bumped to `0.5`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`; also corrected `Parent` from `SPEC-1` to `—` (`BUG-10`): this spec's real lineage is already stated via its own `Implements`/`Derives-from`, not a narrowing of `SPEC-1`'s v0-CLI scope. | **structural** |
> | 2026-09-09 | Declared `pointer_fields = ["Implements"]`/`narrative_fields = ["Blocked-on"]` in config (MILE-90/ADR-44), replacing the two rules' prior hardcoded field list; `blocked-on.stale` renamed `narrative-field.stale`. **Why:** `BUG-8` named this exact type's schema as an example of the genericity `pointer.resolution` claimed but didn't have. | **substantive** |
> | 2026-09-17 | This type's `[record_types.*]` declaration re-rendered in YAML (`ADR-52`, shipped in the same change). The same declaration, same keys, same values -- only the syntax differs. | **structural** |
> | 2026-11-02 | Added the "Phase vocabulary" section defining `0` bootstrap, `1` worth installing, `2` agent-facing, `3` differentiated, `4` scale; bumped to `0.8`. **Why:** `Phase` was documented only as "a plain tag (`0`, `1`, ...)", so the same number meant different things across records and nothing said where adopter-blocking work belongs; `0` now has a stated meaning, which `MILE-35` and the first-adopter milestones rely on. | **substantive** |
> | 2026-11-04 | Phase vocabulary: the `0` row now states the single Phase 0 exit (`SPEC-1`, Exit bar) instead of only "the first adopter replacement"; `Version` `0.8` -> `0.9`. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | Bumped to `0.10`. The type declares `Realized-by` (delivery evidence) and `statuses.delivery`; the delivering PR sets `Realized-by` and `Status: Done`. **Why:** `MILE-183`. | **substantive** |
> | 2026-11-04 | Added the "Required sections" section and bumped to `0.11`: every milestone carries `## Done means`, declared under `sections` and enforced by `section.required`, with `WontDo` exempt. **Why:** maintainer decision of 2026-11-04: "done" was being judged from the What, and the closure audit found closures that over-claimed; stating the criteria on every record lets a closure be checked against them. | **substantive** |
> | 2026-11-05 | Bumped to `0.12`. Added `Realized-by` naming the implementing code and the tests that exercise this spec: `a_milestone_must_state_its_done_means_unless_it_is_wontdo`, `a_done_milestone_needs_delivery_evidence_and_evidence_under_planned_is_a_disagreement`. **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`; the tests run this repository's own `.urzua/config.yaml`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-core/src/status_model.rs, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs, test:rust/crates/urzua-cli/tests/status_delivery_integration.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Status changed from Accepted to Shipped and bumped to `0.13`. **Why:** maintainer decision of 2026-11-05 to ship the specs whose implementation is verified; `Embodiment: Verified` with `Realized-by` naming the implementing code and tests. | **structural** |
> | 2026-11-05 | Bumped to `0.14`. `Blocked` is removed from the `Status` value set: it is derived from `Depends-on` and surfaced by `check` and the roadmap, never written. A waiting milestone is `Planned`, with the wait in `Depends-on` (a record) and `Blocked-on` (the explanation). **Why:** `RFC-48` ("accepting this RFC includes amending `SPEC-6`'s `Status` value set to remove `Blocked`"), decided for this repository in `ADR-83`. | **substantive** |
> | 2026-11-05 | Bumped to `0.15`. The schema excerpt now shows the type as configured: `Depends-on`, `Blocks`, `Non-reference-ids`, `statuses` (with `initial`, `transitions` and `delivery`), `sections` and the three pointer fields. The Status paragraph no longer says `Status` is not validated, and "What's deliberately not built" no longer lists `Depends-on` and a generated roadmap, both built. **Why:** the spec described a config and an engine that had since moved on (`status.undeclared-value`, `status.transition-allowed`, the sequencing pair, `urzua roadmap`); a reader following it would have rebuilt what exists. `Phase` and `Track` are still plain tags. | **substantive** |
> | 2026-11-07 | Bumped to `0.16`. Adds "Done means as items": the item grammar of `ADR-96` verbatim with its nine worked examples, for a type that declares `statuses.delivery.items`; the `Realized-by` row notes that delivery is then judged from the items. **Why:** `MILE-214`; this repository does not declare the option until `MILE-215`. | **substantive** |
> | 2026-11-07 | Bumped to `0.17`. "Done means as items" says this repository declares the option, that an unfinished `Depends-on` target caps a milestone whose items all resolve at `InProgress`, and that `sequence.status-agreement` skips that `InProgress` with an info notice. The schema excerpt is unchanged. **Why:** the `ADR-96` amendment of 2026-11-07, accepted by the maintainer; `MILE-34` and `MILE-215` regain their `Depends-on`. | **substantive** |
> | 2026-11-07 | Bumped to `0.18`. "Done means as items" documents the optional `Kind` field (`build`, `decision`): a `decision` milestone is done when every item cites a decided record or evidence above one, and a Proposed or Draft record does not count. The schema excerpt gains `Kind` as an optional field. **Why:** the `ADR-96` amendment of 2026-11-07, accepted by the maintainer (`MILE-73`); this repository sets `Kind: decision` on ten milestones. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/status_model.rs@sha256:744b341a578a, rust/crates/urzua-cli/tests/record_types_own_config_integration.rs@sha256:7caa23975490, rust/crates/urzua-cli/tests/status_delivery_integration.rs@sha256:3e956fb6227c Reason: Pinned the files as they are now after checking each against the record it evidences: init.rs and check_integration.rs still implement and test urzua init adopt mode (SPEC-5), status_model.rs, the own-config test and status_delivery_integration.rs still implement the milestone type and its delivery status (SPEC-6), both fix.rs files and the paths, rebaseline and drift integration tests still implement urzua fix (SPEC-8), and config.yaml and the own-config tests still declare the bug type (SPEC-9). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Bumped to `0.19`. The `code:` and `test:` locators of `Realized-by` are now hash-pinned by `urzua reverify --pin-only` (the `Pinned` row above) and, where the record stated `Drift detected`, its `Embodiment` was recomputed (the tool-authored row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97` section 10: a claim is renewed only over pinned code and test evidence, so the corpus is pinned once before expiry is enabled. | **structural** |
