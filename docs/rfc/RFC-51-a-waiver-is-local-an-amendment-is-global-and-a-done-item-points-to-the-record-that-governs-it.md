---
Stable-Id: 01M865NQWD49P3J8XHT9CCM0DJ
Status: Draft
Date: 2026-11-24
Author: beauwilliams
Supersedes / Superseded-by:
---
# 51 — A waiver is local, an amendment is global, and a Done item points to the record that governs it

## Summary
A Done item's scope can be narrowed by an agent without any rule noticing: by a sentence in the
milestone's revision log, or by editing the spec the item points at. This RFC carries the
maintainer's direction of 2026-11-24 in six points and tests each against this repository's own
records. The evidence qualifies most of them. The lead proposal for points 3 and 4 is to anchor a
Done item to its governing record with a **pinned, section-scoped `spec:` locator**, so that an
in-place edit of the governing section makes the item's pin stale and the existing re-pin loop
(`diff.pin-changed-unattested` and a re-pin reason a reviewer reads) puts the change in front of a
person. Pointing at a spec without pinning it would only move the universal claim to a place
nothing checks. The route `AGENTS.md:250-251` prescribes, an item waiver of category
`scope-changed-by-decision`, exists and is used 6 times on Done milestones; its gap is that it
accepts an unquoted, agent-attributed decision. Each point gives options with costs, the narrowest
buildable reading, migration impact and open questions. Nothing here is decided.

## Motivation

### The case

1. `MILE-72` is `Done`. Its first Done-means item says "Every command's JSON output carries a
   top-level `next_action`"
   (`docs/milestones/MILE-72-add-a-top-level-next-action-to-every-command-s-output.md:33`). On
   `main`, `SPEC-2` §"Next action (`MILE-72`)" says "Every command's JSON, not only `check`'s,
   carries two top-level keys" (`docs/specs/SPEC-2-urzua-check.md:866-868`). That paragraph opens
   "Maintainer decision, 2026-11-14." with no quote, the form `AGENTS.md:247-250` says cannot be
   checked later.
2. Which commands carry it. `harness run` reaches `emit`, which adds the keys
   (`rust/crates/urzua-cli/src/harness/run.rs:367`, `1686-1691`;
   `harness/run/question.rs:378`, `415`). `harness hook` and `harness record` do not
   (`rust/crates/urzua-cli/src/main.rs:1047-1048`); `record` relays the reports of the commands it
   runs. `urzua describe` (pull request #647, open) prints the agent CLI contract's `describe`
   object. The published agent CLI contract, RFC-4 §6 at `v1.1.0-rc.4` (vendored by #647 at
   `rust/crates/urzua-cli/tests/fixtures/agent-cli-contract/VERSION`), says: "`describe` does not
   list itself. Its answer is not a report, so it fits no profile."
3. How the scope was narrowed. On `main`, `MILE-72`'s revision log mentions `harness hook`/`record`
   and `harness run question`/`answer` only in agent re-pin reasons ("no existing command changed",
   line 85; "nothing else", line 99). At #647's head `17dd218e`, `SPEC-2` is narrowed in place to
   version `0.196` ("`next_action` and `next_action_reason` apply to every command that prints a
   report, not to every command: `describe`, `harness hook`, `harness record`, `hooks run` and
   `mcp`'s own stream print none", that branch's `SPEC-2` line 1748; line 868 now reads "Every
   command that prints a report"), and `MILE-72` gains a row: "The first Done means item is
   unchanged. Which commands carry `next_action` is specified in `SPEC-2` section Next action
   (version `0.196`)" (that branch's `MILE-72` line 105). The milestone's item text is untouched and
   still reads "Every command"; its meaning moved into the spec, which was edited in place.
4. Why nothing fired. `spec:` locators are unpinned by policy: "A `spec:` locator cites a record
   that has its own lifecycle and is left unpinned" (`.urzua/config.yaml:826-830`;
   `require_pin: [code, test]` at 824). Across `docs/`, 385 `spec:` locator occurrences (65
   distinct) carry 1 pin and no section scope (`grep -ohE "spec:[^ ,)]+" docs/*/*.md`). Specs are
   edited in place (`AGENTS.md:51-53`). The item's pinned tests run fixed lists of commands
   (`rust/crates/urzua-cli/tests/next_action_integration.rs:137-165`, `172-191`), so a new command
   is never examined. `ADR-101` rejected recording its principles as a spec for this same reason:
   "a change to a principle would be an edit rather than a decision" (its option B, line 31).
5. How the question reached the maintainer. #647's row quotes the question an agent asked and the
   answer "Yes". The question joined two propositions ("`next_action` applies to commands that print
   a report, and ... `describe` and the three harness commands (hook, record, run) don't print one")
   and the second carried a false premise: `harness run` prints through `emit` (item 2). The
   resulting spec text then reaches `hooks run` and `mcp`, which the question did not name. The
   framing was the orchestrating agent's, in chat; urzua printed nothing about it. `SPEC-2:892`
   makes `next_action` `null` "when the only blocker needs a human". Asking the maintainer a
   multiple-choice question is normal practice here (`ADR-105`; `MILE-49` line 26 quotes a chosen
   option "Fixture only (Recommended)"), so the form was not the failure; the content was.

### What already exists for a scope change

`ADR-96` §3 declares item-waiver categories `not-applicable` and `superseded-by-decision` (cite
required) (lines 136-145), and its amendment of 2026-11-14, accepted 2026-11-16 (lines 280-282,
308), adds `scope-changed-by-decision`: "Its reason quotes the maintainer's instruction or cites the
pull request or review where they gave it, says the item is unmet as written, and the item keeps its
original text." "A waiver without an evidence clause is valid" (line 60). On Done milestones these
three categories waive 16 items (5, 6 and 5), across `MILE-9`, 47, 49, 66, 75, 143, 145, 165, 169,
175, 182, 190 and 278. So `AGENTS.md:250-251` can be followed: an item unmet as written keeps its
text, drops or never has a resolving evidence clause, and is waived. It cannot be combined with
resolving evidence on the same item, because `status.waived-item-stale` then reports "its evidence
now resolves: remove the waiver" (`rust/crates/urzua-core/src/rules/delivery_items.rs:824-834`), by
`ADR-96`'s design. `MILE-72`'s item is the partial case: true of most commands, false of a few.

The gap is the quote. `MILE-145` line 29 waives "by decision of the orchestrator on 2026-11-14,
pending maintainer review", and `MILE-278` lines 21-24 do the same; nothing distinguishes these from
a quoted maintainer decision such as `MILE-165` line 26 ("MILE-165: exercise it, then amend the
item"), because the category has "no machine-checkable staleness" (`ADR-96` line 282).

`AGENTS.md:226-229` offers a second route for an accepted record: a dated `## Amendment` section,
or "a new record that narrows/supersedes it and say so explicitly in both places".

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

The wording above is the brief given to the drafting agent, not a transcript.

### Where the corpus does not split into local and global

- `MILE-66` line 25 waives an item `superseded-by-decision (ADR-31)`, and the reason names "the
  ADR-31 amendment of 2026-09-10": a local waiver pointing at a global change written as a section
  inside the amended ADR. `MILE-145`'s waiver reason likewise rests on a decision recorded elsewhere
  (`ADR-87`).
- `ADR-96`'s own 2026-11-14 amendment changed the rule for every milestone in the repository (a new
  category) and is a section inside `ADR-96`: a global change written locally.
- `deferred-to` (cite and expiry) and `process-outcome` (25 uses on Done milestones) are neither: one
  says the item moves to another record, the other that no file can evidence it.
- `MILE-49` and `MILE-165` are local scope changes with no governing record: the maintainer decided
  what that one milestone needed.
- Milestones are amended in place too: `MILE-329` has six `## Amendment` sections, and `MILE-278`,
  `MILE-322`, `MILE-324` and `MILE-330` one each.

## Proposal

### 1. A waiver is local

What exists. An item waiver is a clause on the item's own line (`ADR-96` §1 rule 6), so it is read
where the item is read; `ADR-96` §3 chose inline over a separate record for exactly that reason. A
`waiver` record (`SPEC-10`) excuses a rule finding from another file. All 16 in `docs/waiver` have a
`Scope` naming a bug file; none targets a milestone.

Qualification from the evidence: the direction already holds for Done items. The separate-file
problem is real only for record-level waivers, today only on bugs.

Options:

- **1a. `explain` and the MCP `urzua_show_record` list the waiver records whose `Scope` is the
  record.** Cost: small, read-side only; a text editor still shows nothing.
- **1b. A `Waived-by` back-pointer on the excused record.** Cost: needs rule code, not only config:
  relation pairs read record ids, and a waiver's `Scope` is a path or `*` (`SPEC-10` line 59), so
  reciprocity would need a path-to-record mapping and a rule for `*`. 16 targets to update.
- **1c. Record-scoped findings waived inline in the record.** Cost: a waiver syntax for every type,
  and a reversal of `SPEC-10`'s separate-record model for bugs.

Narrowest buildable reading: 1a. Migration: none.

Open questions: is the separate waiver record on a bug a problem in practice, given none targets a
milestone? Does a `*`-scoped waiver have any record to be visible on?

### 2. Amends is a global override

What exists. `Amends`/`Amended-by` on RFCs only (`.urzua/config.yaml:178-180`, `214`, `598-610`).
Amendment sections are written inside records: 101 `## Amendment` sections across 48 ADRs; outside
ADRs, 7 in 5 RFCs and specs (9 in 6 counting `SPEC-2`'s two `#### Amendment` headings at lines 908
and 922); and 10 in 5 milestones (above). `appendable_sections` declares the dated section as an
allowed append (`.urzua/config.yaml:721`).

Qualification: `ADR-14` does not stand in the way of 2a. Its model freezes the decision text, not the
header, and superseded ADRs gained `Superseded-by` after acceptance. The real conflicts are
`MILE-155`'s config comment ("an ADR cannot echo, since an ADR is amended by a dated section",
`.urzua/config.yaml:602-603`), `SPEC-16:76` and `88-93` (the ADR's "Why amendments, not revisions"),
and `appendable_sections`, which keeps the in-record route valid. With both routes valid, no rule
says which one a given change must take.

Options:

- **2a. Widen the `amendment` pair to `adr` and `spec` targets.** Config, plus the `MILE-155`
  comment and `SPEC-16` text. Cost: two routes coexist and nothing chooses between them.
- **2b. 2a, and the in-record section is no longer appendable for new amendments** (existing ones
  stay as history). Cost: `SPEC-16`, the `amend` command and `appendable_sections` change; every
  future ADR clarification becomes a record.
- **2c. Keep sections, but require an `Amended-by` header naming the section's date.** Rejected by
  the direction: the section is not a separate record.

Narrowest buildable reading: 2a, with 2b as its own later decision. Migration: none forced.

Open questions: should a section remain right for a clarification that changes no rule, and a
separate record required only for a change that alters what other records rely on (the global case)?
Is a spec amended by a separate record at all, or is a pinned pointer (point 3) enough to make its
in-place edits visible?

### 3. Done-means items point to their governing record

Qualification: as written, pointing an item at a spec moves the universal claim into a record that
is edited in place and whose locators are unpinned (Motivation, item 4). "The milestone stays true"
then holds by definition, which is what #647 did. The pointer has to be anchored.

Count. Over 174 milestones with `Status: Done`, 1,043 top-level Done-means items; 131 name a
`SPEC-`, `ADR-`, `BUG-` or `RFC-` id in the item text, 115 carry a `spec:` or `record:` locator, 164
do either, about 879 (84%) neither. Method: a script over `docs/milestones/*.md`, the `## Done means`
section of each `Done` milestone, lines starting `- ` or `N. ` at column zero, item text taken before
` — `. Upper bound on "points": a named id may be context, not the governor. 131 Done milestones
carry `Implements`, 148 values, 36 of them `SPEC-1`, so `Implements` is too coarse to stand in for a
per-item governor. Locator kinds are `spec`, `code`, `test` and `record`
(`rust/crates/urzua-core/src/locator.rs:215`).

Options:

- **3a (lead). The governing pointer is a pinned, section-scoped `spec:` locator.** For example
  `spec:docs/specs/SPEC-2-urzua-check.md#Next action (MILE-72)@sha256:<pin>`. Needs: a `markdown`
  entry in `locator_resolvers` (the resolver engine already has an `end: heading` kind,
  `locator.rs:807-815`, so this may be configuration; whether `{symbol}` can match a heading with
  spaces is open); and `spec` pinned on Done items (today `require_pin` is `[code, test]` and the
  `spec` drift policy is `acknowledge`, `.urzua/config.yaml:824-830`, an `ADR-97` amendment). An
  in-place edit of that section then makes the item's pin stale, CI fails on a stale pin, and the
  re-pin needs a reason a reviewer reads (`diff.pin-changed-unattested`, `SPEC-8`). This anchors
  the claim instead of moving it, and is also point 4's stop. Cost: every governing-section edit
  forces a re-pin on every item that points at it; an `ADR-97` amendment; 1 of 385 `spec:` locators
  pinned today, so pinning is opt-in per item, not a migration.
- **3b. A declared `governed:` clause in the `ADR-96` grammar.** Cost: a grammar amendment and a
  parser, and without a pin it has 3c's weakness.
- **3c. Convention only ("as `SPEC-2` §Next action specifies").** Cost: moves the claim without
  anchoring it. Not recommended on its own.

Narrowest buildable reading: 3a for new items that name a governing section, starting with the
resolver. Migration: none forced; the maintainer said items with no governing record stay as they
are, and rewriting Done item text is forbidden by `AGENTS.md:250-251`.

Open questions: does the item still restate the requirement briefly, so a reader without the spec
knows what was promised? Is a stale pin on a governing section a blocking finding or a warning?

### 4. A stop on silent narrowing

Qualification: the engine does not know which revision rows a tool wrote. It recognises a re-pin
row by its text prefix (`REBASELINE_ROW_PREFIX`, `rust/crates/urzua-core/src/fix.rs:935`; read by
`rebaseline_row_pins`, `rules/pin_attestation.rs:122`), which anyone can type, and narrowing written
inside a re-pin `--reason` passes as a tool row.

Options:

- **4a (lead). 3a's pinned governing locator.** An in-place narrowing of the governing section turns
  red at the pin, and the re-pin reason is reviewed. Covers the #647 case; does not cover a
  narrowing written only in a milestone row.
- **4b. `diff.done-item-touched`.** On a `--base` run, report a change to a Done item's line or its
  waiver clause (added, edited or removed). Cheap, and aimed at the place a scope lives. Cost: misses
  narrowing written in a revision row, as on #647; `AGENTS.md` already forbids rewriting item text.
- **4c. `diff.done-record-annotated`.** Report any non-tool row added to a Done record. The review
  found it would have fired on 318 rows in 37 of 703 commits over all history, 32 rows since
  2026-11-10, about 4 of them scope-relevant (the reviewer's probe; not reproduced for this
  revision). Noisy, and gameable through the row prefix. Demoted.
- **4d. Evidence that enumerates the command set.** urzua's own test walks the clap tree and
  requires each subcommand to carry the keys or appear in a reviewed exemption list. Fixes
  `MILE-72`'s evidence; not a general stop.

Narrowest buildable reading: 4a, with 4b as a cheap complement and 4d for `MILE-72`. Migration:
none.

Open question: should a re-pin reason on a governing section be required to cite an amending record
or a quoted decision?

### 5. Reconcile `AGENTS.md:250-251`

Correction to this RFC's first draft: the prescribed route can be followed (Motivation, "What already
exists"); it cannot coexist with resolving evidence on the same item.

Options:

- **5a. Keep the waiver route; require its evidence.** `scope-changed-by-decision` cites a pull
  request, a review or an `Accepted` ADR (a declared `cite: required` with a cite form that accepts
  a PR number, or a record id). This rejects the "decided by the orchestrator ... pending maintainer
  review" form of `MILE-145` and `MILE-278`. Cost: an `ADR-96` amendment (today "no cite ... because
  the decision is not always a record") and the cite grammar, which today says "A pull request or an
  issue number is not a cite" (`ADR-96` §1 rule 6).
- **5b. Route a scope change to the governing record where one exists**, by 3a: amend or edit the
  governing section, re-pin with a reason quoting the decision. The waiver stays for items with no
  governing record (`MILE-49`, `MILE-165`). Proposed wording, for the maintainer:

  > When the maintainer changes the scope of a requirement a Done item delivers, record the decision
  > quoted (or cite the pull request or review where they gave it). Where the item points at a
  > governing section, change that section and re-pin the item with a reason that quotes the
  > decision; never narrow it in a revision row. Where it has none, keep the item's text and waive it
  > `scope-changed-by-decision` (`ADR-96`). Never rewrite the item to fit what was built.

- **5c. Waivers only for not-yet-delivered items.** Retires `not-applicable`,
  `superseded-by-decision` and `scope-changed-by-decision` and needs an `ADR-96` amendment. Dropped:
  the 16 existing uses show they do work the direction wants done.

The first draft's interim, reopening the milestone by hand, does not work: status is computed from
the items (`ADR-96` §2), so `status.delivery-agreement` writes it back. Narrowest buildable reading:
5a's cite requirement plus 5b's wording once 3a exists.

### 6. How the question reaches the maintainer (option)

Qualification: a one-line yes-or-no question was asked in this case and went wrong (Motivation, item
5): two propositions in one line, one false, and an answer later stretched to commands it did not
name. A menu was not the failure.

Option, weighed and not committed: urzua writes the question, from configuration, only when a
declared rule's finding can be resolved only by a person. Constraints for it to be safe:

- one proposition per line, each answerable alone;
- each proposition built from the finding's own data (the commands, the record, the section), never
  agent prose, so a false premise cannot enter;
- each line says what "No" does as well as "Yes", and names the route the repository prescribes;
- the answer binds only to the propositions shown, recorded the way `ADR-105` binds a person's answer
  to one gated move.

Cost: a report key, which the contract's `report.schema.json` permits (it has no top-level
`additionalProperties: false`), per-rule question templates, and a binding store. Alternatives:
`AGENTS.md` wording for orchestrating agents (free, unchecked), or nothing.

Open question: which findings are "only a person can resolve"?

## Open questions

Per point, above. Across them:

- Order: 3a/4a (resolver, pinned `spec:` on Done items) and 5a (cite) are independent and can come
  first; 2b and 6 are larger decisions.
- Whether `MILE-72` is resolved by #647's spec edit, by keys on the remaining commands, or by a
  waiver with a quoted decision, is the maintainer's.
- Whether "local versus global" is the right cut at all, given the cases above, or whether "where
  is the claim anchored, and does a change to it turn something red" is the operative question.

## Non-goals

- Judging whether a revision row's prose narrows scope. A rule reports presence and shape, never
  meaning (`AGENTS.md:28-31`).
- Rewriting existing Done items or amendment sections.
- Editing `AGENTS.md`, `.urzua/config.yaml`, `MILE-72` or `SPEC-2` here.

## References

- `MILE-72`; `SPEC-2` §"Next action (`MILE-72`)"; the open `urzua describe` pull request, branch
  `feat/describe` (`MILE-359`; cited by number in Motivation).
- `ADR-96` (items, item waivers, the 2026-11-14 amendment), `ADR-97` (spec drift policy), `ADR-101`,
  `ADR-64`, `ADR-14` and `SPEC-16`, `ADR-73`, `ADR-105`, `SPEC-8` (pins and re-pins), `SPEC-10`,
  `SPEC-19`.
- `MILE-9`, `MILE-49`, `MILE-66`, `MILE-145`, `MILE-155`, `MILE-165`, `MILE-278`, `MILE-288`,
  `MILE-329`.
- The published agent CLI contract, RFC-4 §6, `v1.1.0-rc.4`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Revised after an independent review of the first draft: points 3 and 4 lead with a pinned, section-scoped `spec:` locator; the existing `scope-changed-by-decision` waiver route is described and the claim that `AGENTS.md:250-251` cannot be followed is corrected; local and global are tested against `MILE-49`, 66, 145, 165 and `ADR-96`; 4c is demoted; point 6 is restated around the question asked in #647; counts, `harness run`'s output and #647's head (`17dd218e`) are corrected. Status unchanged. **Why:** review findings, each checked against the repository. | **substantive** |
