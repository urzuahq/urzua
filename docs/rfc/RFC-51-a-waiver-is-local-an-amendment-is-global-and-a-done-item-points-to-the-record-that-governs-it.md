---
Stable-Id: 01M865NQWD49P3J8XHT9CCM0DJ
Status: Draft
Date: 2026-11-24
Author: beauwilliams
Supersedes / Superseded-by:
---
# 51 — A waiver is local, an amendment is global, and a Done item points to the record that governs it

## Summary
A Done item's scope can be narrowed today by an agent writing a sentence into the milestone's
revision log, and nothing reports it. The route `AGENTS.md` prescribes for a scope change, an item
waiver, cannot be followed once the item's evidence resolves, because the engine then calls the
waiver stale. This RFC proposes the maintainer's direction of 2026-11-24 as six points: a waiver is
local and visible where the excused record is read; an amendment is a separate record that changes
the rule everywhere and is named back by `Amended-by`; a Done-means item points to the record that
governs it instead of restating it; a stop reports a hand-written scope note on a Done record; the
`AGENTS.md` paragraph on scope changes is reconciled with the engine; and, as an option only, urzua
states a decision only a person can make as one yes-or-no line naming the prescribed route. Each
point gives options with their costs, the narrowest buildable reading, the migration impact and
open questions. Nothing here is decided.

## Motivation

### The case

1. `MILE-72` is `Done`. Its first Done-means item says "Every command's JSON output carries a
   top-level `next_action`"
   (`docs/milestones/MILE-72-add-a-top-level-next-action-to-every-command-s-output.md:33`).
   `SPEC-2` §"Next action (`MILE-72`)" says the same: "Every command's JSON, not only `check`'s,
   carries two top-level keys" (`docs/specs/SPEC-2-urzua-check.md:866-868`).
2. Commands were added after it with no `next_action`. `harness hook`, `harness record` and the
   `harness run` events dispatch straight to their own printers and never reach `emit`, which adds
   the keys (`rust/crates/urzua-cli/src/main.rs:1047-1062`; `harness/record.rs` and
   `harness/hook/mod.rs` contain no `next_action`). `urzua describe` (pull request #647, open, not
   merged) prints the agent CLI contract's `describe` object, which carries no `next_action`. The
   published agent CLI contract, RFC-4 §6 at `v1.1.0-rc.4` (the version #647 vendors at
   `rust/crates/urzua-cli/tests/fixtures/agent-cli-contract/VERSION`), says: "`describe` does not
   list itself. Its answer is not a report, so it fits no profile."
3. Agents wrote the notes that narrowed the item. On `main`, `MILE-72`'s revision log records the
   harness commands only inside re-pin reasons: "adds the `harness hook <adapter> <event>` and
   `harness record <step>` subcommands; no existing command changed" (line 85) and "adds the harness
   run question and harness run answer subcommands and their dispatch; nothing else" (line 99), both
   `actor: agent`. Neither says the item no longer holds for them. On #647's branch an agent adds a
   `structural` row: "The Done means items are unchanged. "Every command" in the first item reads as
   every command that prints a report: `urzua describe` ... carries no `next_action` keys; the
   pinned test does not run it" (that branch's MILE-72 line 104). That is an explicit narrowing of a
   Done item's scope, in prose, by an agent, with no quoted maintainer decision.
4. No rule flagged any of it. The item's pinned test,
   `next_action_integration.rs#every_other_command_and_a_could_not_run_emit_both_keys_as_null`,
   runs a fixed list of six commands plus `doctor` and a no-config `check`
   (`rust/crates/urzua-cli/tests/next_action_integration.rs:137-165`); the exactly-once guard runs a
   fixed list of sixteen (lines 172-191). Neither enumerates the command set, so a new command is
   skipped, the evidence still resolves, and the gate passes.
5. The prescribed route does not work for a delivered item. `AGENTS.md:250-251`: "When the
   maintainer changes an item's scope, keep the item's original text and waive it with the decision
   quoted (`ADR-96`, item waivers ...)". An item waiver on an item whose evidence resolves is
   reported "its evidence now resolves: remove the waiver"
   (`rust/crates/urzua-core/src/rules/delivery_items.rs:824-834`, `status.waived-item-stale`, `warn`
   at `.urzua/config.yaml:841`). `ADR-96` §1 rule 4 and §3 designed it so: a waiver excuses an item
   until it resolves, then is stale. A waiver means "not yet delivered", not "the requirement
   changed".
6. The other route, `Amends`/`Amended-by`, exists only between RFCs: the fields are declared for the
   `rfc` type (`.urzua/config.yaml:178-180`, `214`) and the relation pair is limited to it
   (`.urzua/config.yaml:598-604`: "Only RFCs carry the pair (`MILE-155`): an RFC that amends an ADR
   claims something the ADR cannot echo, since an ADR is amended by a dated section"). A milestone or
   spec cannot be amended by a separate record the engine checks.
7. The question reached the maintainer as a menu of three options, framed by the orchestrating
   agent; urzua printed nothing about it. `SPEC-2` sets `next_action` to `null` "when the only blocker
   needs a human" (`docs/specs/SPEC-2-urzua-check.md:892`), so urzua had no way to say which decision
   was needed.

### Why it matters

`ADR-101` principle 2, "Claims are backed by evidence", is the mission's own test. Here a Done claim
stopped being true of the code, an agent rewrote what the claim means in a sentence no rule reads,
and the gate stayed green. `AGENTS.md`'s own sentence (lines 248-250) names the failure: "a scope
change recorded that way narrows a Done means without evidence".

### The maintainer's direction (2026-11-24, this session, relayed)

1. **A waiver is local.** The rule still applies, except in the waived circumstances. A waiver must
   be visible where the excused item or record is read. A separate WAIVER record in another file
   isn't visible to an agent reading a milestone.
2. **Amends is a global override.** It is a separate record built on top of the one it amends, and
   it changes the rule everywhere. The amended record must show `Amended-by`, so a reader reaches
   the current rule. An amendment is not a section written inside the amended record.
3. **Done-means items point to their governing record.** An item points to a spec, ADR or bug ("as
   SPEC-2 §Next action specifies"), with code and test locators as evidence; it doesn't restate the
   requirement in its own prose. A scope change then means amending the governing record, and the
   milestone stays true. Items with no governing record stay as they are.
4. **A stop on silent narrowing.** Something must report an agent changing a Done item's scope in
   prose. Propose the smallest mechanism and argue it from the engine.
5. **Reconcile `AGENTS.md:250-251`** with the engine and with points 1-2: a scope change is an
   override, recorded on the governing record, not a waiver.
6. **How the question reaches the maintainer.** When only a person's decision resolves a conflict,
   urzua's output should state the decision in one line they can answer yes or no, and name the
   route the repo's rules prescribe. An option this RFC weighs, not a commitment.

The six points are the maintainer's words as relayed in the session; the wording above is the brief
given to the drafting agent, not a transcript.

## Proposal

### 1. A waiver is local

What exists. Two waiver forms. An item waiver is a clause on the item's own line (`ADR-96` §1, rule
6: `waived: <category> (<cite>, <date>) -- <reason>`), so it is already read where the item is read.
A `waiver` record (`SPEC-10`, 16 files in `docs/waiver`) excuses a rule finding from another file;
an agent reading the excused record does not see it.

Options:

- **1a. Keep both forms; `explain` and `check` show a record's waivers when it is read.** `urzua
  explain <path>` and the MCP `urzua_show_record` list every waiver record that targets the record.
  Cost: small; the file still says nothing to a reader with only a text editor.
- **1b. A waiver record is named back on the excused record.** A declared field (`Waived-by`) on the
  excused record, kept in step by a `relation_pairs` entry with `reciprocity: required`, the same
  mechanism the `amendment` pair uses. Cost: a config change on every type a waiver can target, a
  back-pointer to write on 16 existing waivers' targets, and `urzua fix` would want to write it.
  Benefit: the excused file itself shows the exception; the engine checks the pair already.
- **1c. Only inline waivers for record-scoped findings.** A finding on one record is waived by a
  clause in that record; waiver records remain for findings with no single home (a config or
  corpus-level finding). Cost: a second waiver syntax for non-milestone types and a migration of
  record-scoped waiver records.

Narrowest buildable reading: 1b, using the existing relation-pair engine, no new rule code. 1a is
a cheap complement.

Migration: 16 waiver records; each target gains one field. Item waivers (inline) are untouched.

Open questions: does a waiver whose target is a rule over many records (no single record) stay a
separate file with no back-pointer? Does `Waived-by` belong in `pointer_fields` (clean) for every
type, which needs a config change only the maintainer can make (`.urzua/config.yaml` is a protected
path)?

### 2. Amends is a global override

What exists. `Amends`/`Amended-by` on RFCs only (`.urzua/config.yaml:598-610`, with a provisional
claim while the amending RFC is a draft, `MILE-154`, and `Amended-by-pending`, `MILE-163`). ADRs are
amended by a dated `## Amendment` section inside the ADR (`AGENTS.md:226-229`, `SPEC-16`, `ADR-14`):
101 such sections across 48 ADRs, and 5 in specs and RFCs (`grep -c '^## Amendment'`). Two records
carry `Amends:` today. The maintainer's point 2 says an amendment is not a section inside the
amended record, which runs against this convention.

Options:

- **2a. Extend the `amendment` pair to every decision type (rfc, adr, spec, bug).** Config only: add
  the fields to `known_fields`/`pointer_fields` and widen `types`. An amending record is an ADR (or a
  spec revision) that names `Amends: SPEC-2` and the amended record carries `Amended-by`. The
  existing provisional handling covers a proposed amendment. Cost: the `MILE-155` reason ("an ADR
  cannot echo") goes away only if an accepted ADR may gain an `Amended-by` header field, which is a
  header edit to a frozen record; `ADR-14` and `SPEC-16` would need amending to allow it (a
  tool-written, structural change, the same class as a re-pin row). `relation_pairs` may need a
  cross-type pair (an ADR amends a spec), which the config already expresses via `types`.
- **2b. 2a, and dated `## Amendment` sections are retired for new amendments.** Existing sections
  stay as history (frozen records are not rewritten). Cost: two conventions coexist in the corpus
  for good; `AGENTS.md:226-229`, `SPEC-16` and the `amend` command change.
- **2c. Keep sections; add `Amended-by` pointing at the section's own record.** Rejected by the
  direction itself: the section is not a separate record.

For a milestone: a milestone is never amended. Under point 3 its items point at the governing
record, so the amendment lands there.

For a spec: `SPEC-*` is living and edited in place with a version bump (`AGENTS.md:51-53`). Two
readings: (i) a spec's scope change is still an in-place edit plus a revision row, and `Amends`
applies to ADRs and RFCs; or (ii) a scope change that narrows a requirement a Done item points at is
an amending ADR that names the spec section, and the spec is edited to match and carries
`Amended-by`. Reading (ii) keeps a decision record for every narrowing; (i) is cheaper.

Narrowest buildable reading: 2a for `adr` and `spec` targets, amended by an `adr`, with no rule
change (the pair, reciprocity and provisional claim already exist). 2b is a separate, later decision.

Migration: none required (additive); the 101 existing sections stay.

Open questions: may an accepted ADR's header gain `Amended-by` without an `ADR-14` amendment, and is
that amendment itself the first use of the new route? Does an amendment of a spec bump its version
(it should, under `revision-log.version-bump-required`)? Should `urzua explain` print the amended
record's current rule by following `Amended-by` to its end?

### 3. Done-means items point to their governing record

What exists. `ADR-96`'s item grammar has item text, an evidence clause and a waiver clause. The
evidence clause already takes `spec:` locators, and `Specified` is in `resolving_tiers` (`ADR-96`
§2). Nothing distinguishes "the record this item delivers" from evidence.

Count. Over the 174 milestones with `Status: Done`, 1,043 top-level Done-means items. 131 name a
`SPEC-`, `ADR-`, `BUG-` or `RFC-` id in the item text; 115 carry a `spec:`/`adr:`/`bug:`/`rfc:`
locator in the evidence clause; 164 do either. About 879 (84%) do neither and restate their
requirement in prose. Counted with a script over `docs/milestones/*.md`: the `## Done means` section
of each `Done` milestone, lines starting with `- ` or `N. ` at column zero, item text taken as the
text before ` — `. It is an upper bound on "points": naming an id is not the same as pointing at the
governing record, and some named ids are context, not the governor. 135 Done milestones carry an
`Implements` header, so a governing record is usually one hop away.

Options:

- **3a. A convention, no engine change.** New items read "as `SPEC-2` §Next action specifies" with a
  `spec:` locator. Cost: unchecked; drifts back to restating.
- **3b. A declared governing clause.** Extend the `ADR-96` grammar with a third declared marker
  (`governed:`, in configuration like the other two), holding one record id and an optional section
  anchor. The engine resolves it (`pointer.resolution`) and reports the governing record's status;
  an item governed by a `Superseded` record, or by one that has gained `Amended-by` since the item
  resolved, is a finding that points the reader to the current rule. Cost: a grammar change to a
  shipped ADR (an `ADR-96` amendment), parser and rule work, and a `SPEC-6` update.
- **3c. Reuse the evidence clause.** A `spec:`/`adr:` locator as the first evidence locator is read as
  the governing record. Cost: overloads evidence with "which rule"; a `spec:` locator already means
  "the spec documents it" (`MILE-72` line 35), which is a different claim.

Narrowest buildable reading: 3a now for new items, with 3b as a milestone once point 4 shows how often
it matters.

Migration: none forced. The maintainer said items with no governing record stay as they are. A
voluntary rewrite of the ~879 restating items would edit Done records' item text, which
`AGENTS.md:250-251` forbids today ("never rewrite the item to fit what was built"); any rewrite would
need that sentence changed first and should be limited to items where a governing record exists.

Open questions: is a section anchor (`SPEC-2#next-action`) resolvable today, or is the id enough?
Does an item that points to a governing record still need code and test locators to resolve (yes,
under `ADR-101` principle 2, as the maintainer's wording says)?

### 4. A stop on silent narrowing

The engine already knows which revision rows a tool wrote: `diff.pin-changed-unattested`
(`MILE-288`) accepts a pin change only with a row of the tool's shape
(`rust/crates/urzua-core/src/rules/pin_attestation.rs:598-613`), and each type declares a
`tool_class` (`.urzua/config.yaml:422-425`). A `--base` run already compares a record with its
merge-base copy (`revision-log.version-bump-required`, `BUG-51`).

Options:

- **4a. `diff.done-record-annotated` (smallest).** On a `--base` run, a record whose status is in the
  type's delivered set (`Done`), unchanged in status, that gains a revision row the tooling did not
  write, is reported (`warn`, or `error` in this repository), naming the row. The finding's message
  names the prescribed route ("a scope change amends the governing record"). Cost: one rule over data
  the engine already parses; it fires on every hand-written row on a Done record, including honest
  ones (a typo fix, a link), so a reviewer reads each. It judges presence, not meaning, which is the
  most `AGENTS.md:28-31` allows. It would have fired on #647's row; it would not have fired on the
  harness re-pins, whose rows the tool wrote.
- **4b. Re-pin reasons are read too.** The re-pin reason is an agent's prose inside a tool row
  (lines 85, 99). A rule cannot judge whether "no existing command changed" is a scope statement.
  Not proposed as a rule; the review loop reads re-pin reasons already (`AGENTS.md:318-328`).
- **4c. Evidence that enumerates.** The narrowing went unseen because the pinned test runs a fixed
  list. urzua's own test can enumerate the clap command tree (#647 generates `describe` from it) and
  require each subcommand to carry the keys or appear in a declared, reviewed exemption list. Cost:
  a test change in this repository only; it catches the next command, not the next narrowing in
  general. It is the honest fix for `MILE-72`'s evidence either way.
- **4d. Freeze Done-means text.** A `--base` diff that edits a Done item's text is an error. Cost:
  covers rewriting, not annotating, so it misses this case; `AGENTS.md` already forbids rewriting.

Narrowest buildable reading: 4a, config-driven (the delivered set and `tool_class` are declared), plus
4c for `MILE-72` itself.

Migration: none; a `--base` rule sees only new rows.

Open questions: should 4a exempt rows that cite a quoted maintainer decision? (No rule can tell a
quote from a fabricated one, so exempting it reopens the hole.) Should 4a also cover `Fixed` bugs
and `Accepted` ADRs, where `## Amendment` sections are written by hand by design?

### 5. Reconcile `AGENTS.md:250-251`

The sentence prescribes an item waiver for a scope change; the engine calls such a waiver stale
once the item resolves (Motivation, item 5), and point 1 says a waiver is a local exception, not a
change of rule. Proposed wording, for the maintainer to adopt, change or reject (not applied here;
`SPEC-19`'s revision log would record it in the same PR):

> When the maintainer changes the scope of a requirement a Done item delivers, the change is an
> override, not a waiver: record it on the governing record the item points to (an amending record
> with `Amends`, which the amended record names back with `Amended-by`), with the maintainer's
> decision quoted. Never rewrite the item and never narrow it in a revision row. A Done item with no
> governing record keeps its text; its milestone gains an amending record that names it, or the
> maintainer decides the item is reopened. An item waiver is only for an item that is not yet
> delivered, and is removed when its evidence resolves (`ADR-96`).

Options: (5a) the wording above, which depends on point 2's route existing for the governing type;
(5b) until point 2 is built, say a scope change on a Done item is reopened (status back to
`InProgress` with an unresolved item) and filed for the maintainer, which works with today's engine;
(5c) leave the sentence and accept that it cannot be followed for delivered items. 5b is the
buildable interim.

Migration: the corpus has item waivers on unresolved items only (a resolved one is already a
`status.waived-item-stale` warning), so none move.

Open question: for `MILE-72` specifically, is the right outcome an amendment of `SPEC-2` §Next action
("every command that prints a report"), or keys on the harness and `describe` output? The contract
text in Motivation item 2 bears on `describe` only.

### 6. How the question reaches the maintainer (option)

What exists. `next_action` is `null` when the only blocker needs a human (`SPEC-2:892`). `ADR-105`
already has urzua write one question for a person to answer in one case (a gated status move under
`in_session: ask`).

Options:

- **6a. A `decision` object beside `next_action`.** When a finding can only be resolved by a person,
  the report carries `decision: {question, route, records}`: one line answerable yes or no ("Amend
  `SPEC-2` §Next action so it covers only commands that print a report?"), the route from the repo's
  rules (the finding's declared route, e.g. "an ADR with `Amends: SPEC-2`"), and the records named.
  The question text comes from configuration per rule, with the record ids filled in, so it stays
  structured (`ADR-64`) and the engine never composes judgement prose. Cost: an output-contract
  change on every report (`ADR-46`, and the agent CLI contract's report schema, which may not allow
  an extra key); per-rule question templates to maintain.
- **6b. A `suggested_action` kind `ask-maintainer`.** Per finding, inside the closed set of
  `ADR-73`. Cost: an `ADR-73` amendment; per finding, not per report, so several findings give
  several questions.
- **6c. Documentation only.** `AGENTS.md` tells an orchestrating agent to put one yes-or-no question
  and the prescribed route to the maintainer. Cost: unchecked, and this case shows an agent framed
  it as a menu.

Weighed, not committed. 6c is free; 6b fits the existing structure; 6a is the fullest and the most
expensive.

Open questions: which findings are "only a person can resolve" (a declared list per rule, or every
finding with no `run-command` action)? Does a yes-or-no line hide a real third option the maintainer
would want to see?

## Open questions

Per point, above. Across them:

- Order: point 4's rule and the 5b interim need no new record route and can come first; points 2 and
  3 change shipped ADRs (`ADR-14`, `ADR-96`) and need their own decisions.
- Whether `MILE-72` is resolved by amending `SPEC-2`, by adding the keys to the harness commands, or
  by reopening it, is the maintainer's; this RFC records the case, not the answer.
- Whether point 2's "an amendment is not a section" applies retroactively to how this repository
  reads its 101 existing ADR amendment sections.

## Non-goals

- Judging whether a revision row's prose narrows scope. A rule reports presence and shape, never
  meaning (`AGENTS.md:28-31`).
- Rewriting existing Done items or ADR amendment sections.
- Editing `AGENTS.md`, `.urzua/config.yaml` or `MILE-72` in this pull request. All three are the
  maintainer's call; the first two are protected paths.

## References

- `MILE-72`, `SPEC-2` §"Next action (`MILE-72`)", the open `urzua describe` pull request on branch `feat/describe` (`MILE-359`; cited by number in Motivation).
- `ADR-96` (Done-means items and item waivers), `ADR-101` (mission principles), `ADR-64` (structured
  in and out), `ADR-14` and `SPEC-16` (amendment sections), `ADR-73` (suggested actions), `ADR-105`
  (one question to a person), `SPEC-10` (waiver records), `SPEC-19` (governs `AGENTS.md`).
- `MILE-154`, `MILE-155`, `MILE-163` (the `amendment` relation pair); `MILE-288`
  (`diff.pin-changed-unattested`).
- The published agent CLI contract, RFC-4 §6, `v1.1.0-rc.4`.
- `rust/crates/urzua-core/src/rules/delivery_items.rs:824-834`,
  `rust/crates/urzua-cli/tests/next_action_integration.rs:137-191`,
  `rust/crates/urzua-cli/src/main.rs:1047-1062`, `.urzua/config.yaml:178-180, 214, 598-610, 841`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
