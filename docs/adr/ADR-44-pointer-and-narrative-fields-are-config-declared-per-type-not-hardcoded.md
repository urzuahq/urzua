---
Stable-Id: 01M21GQHRW1JGSVYTVMRD0R7HB
Status: Accepted
Date: 2026-09-08
Accepted-on: 2026-09-09
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: RFC-23
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/config.rs#RecordTypeConfig::pointer_fields@sha256:50b69240d87e065ffdc0b2b5b04640d9228965709b73b44d8eb9e0765ffe2781, code:rust/crates/urzua-core/src/config.rs#RecordTypeConfig::narrative_fields@sha256:55042eff1f13b6b1cc2aff63d442ce6e175fc6df18b0cd5addde3844aa29000f, code:rust/crates/urzua-core/src/rules/pointer.rs@sha256:2d164d27a2e9d20a589adeee671eaae8db4d5e6529b3e705e9df1d1b0ed6e5ae, code:rust/crates/urzua-core/src/rules/narrative.rs@sha256:b34355340066c63614438add0c4da8230a9aafd84c991834a59414eaa09eaa74, code:rust/crates/urzua-core/src/graph.rs@sha256:03f7eed2679cc43971411a8874bb09e17c284a7f1b3a9c4accc15ace4cd518f6, test:rust/crates/urzua-core/src/rules/pointer.rs@sha256:2d164d27a2e9d20a589adeee671eaae8db4d5e6529b3e705e9df1d1b0ed6e5ae, test:rust/crates/urzua-core/src/graph.rs@sha256:03f7eed2679cc43971411a8874bb09e17c284a7f1b3a9c4accc15ace4cd518f6, test:rust/crates/urzua-core/src/config.rs#pointer_and_narrative_fields_round_trip@sha256:b016c902b0e06dfd2bbf1385294f9302ec0846b162bbd33e439620357ebf0dcc
---
# 44 — Pointer and narrative fields are config-declared per type, not hardcoded

## Context

`pointer_resolution`, `blocked_on_stale`, and `header.pointer-field-clean` each hardcode a fixed
list of field names (`Implements`/`Derives-from`/`Parent`/`Blocked-on`) directly in `urzua-core`'s
Rust source, never read from `.urzua/config.toml`. This contradicts the engine's own pitch --
proven false live (`BUG-8`): the README's lineage section claims an org can add a field like
`Feeds-into` and have it just work, but `pointer_resolution` would never check a field outside its
hardcoded array, no matter what `known_fields` declares.

The gap is sharper than "make one list configurable." Two genuinely different kinds of pointer
field are already implicit in the code: plain resolving pointers (`Implements`/`Derives-from`/
`Parent` -- exist, resolve, nothing else checked) and staleness-aware, prose-tolerant pointers
(`Blocked-on` -- resolves if a reference is present, tolerates free text, and is separately checked
for the target's terminal status). Making the list configurable without naming this distinction
would leave `header.pointer-field-clean` unable to tell which behavior a newly-declared field
should get.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Leave hardcoded | No work | README's own claim stays false; an adopting org can't add relationship vocabulary without forking `urzua-core` |
| One config list, one kind | Simple | Can't express `Blocked-on`'s prose-tolerant, staleness-checked behavior alongside clean pointers |
| Two per-type lists (`pointer_fields`/`narrative_fields`), no default | Matches both real kinds; matches `header_layout`/`known_fields`'s own "declared, not inferred, undeclared means skip" precedent; no compatibility-shim cruft for a tool with no real external adopters yet | Every one of this repo's own six types needs an explicit declaration -- a real, visible migration, not free |

## Decision

In the context of an engine that already lets an org declare its own record types with zero code
changes, but silently doesn't extend that to relationship vocabulary, facing a README claim that
was false the moment it was checked, we decided: **add `pointer_fields` and `narrative_fields` as
per-type config concepts**, resolved and checked exactly as `pointer_resolution`/
`header.pointer-field-clean`/`blocked_on_stale` already do today, but reading the field list from
`.urzua/config.toml` instead of a hardcoded Rust array -- to make the engine's own "declared, not
hardcoded" pitch actually true for relationship fields, not just record types -- accepting that
every type in this repo's own corpus needs an explicit declaration with no implicit default,
since there is no real backward compatibility to preserve.

`narrative_fields` (not `blocking_fields`) names the mechanism -- tolerates narrative text
alongside an optional embedded reference -- not today's one use case, so the name stays honest if
reused for a future field that isn't about blocking at all.

## Reversibility

Cheap to reverse in principle (remove the config keys, fall back to the current hardcoded arrays),
but every type's config would need to lose its explicit declaration too, and any adopting org that
had started relying on custom `pointer_fields`/`narrative_fields` would lose that capability
outright. Cost is low today (zero real external adopters), but grows the longer this ships before
being reconsidered.

## Consequences

- `pointer_resolution`, `header.pointer-field-clean`, and `blocked_on_stale` each read their field
  list from config instead of a hardcoded array.
- `urzua graph` gains a `kind` field per edge (`"pointer"` or `"narrative"`), so a consumer can
  distinguish the two without inferring it from the field name.
- Every one of this repo's own six types (`adr`, `rfc`, `spec`, `milestone`, `bug`, `waiver`) must
  declare `pointer_fields`/`narrative_fields` explicitly in `.urzua/config.toml` -- an undeclared
  type gets zero fields of either kind checked, a real behavior change from today's implicit
  coverage, not just a config addition.
- `SPEC-2` (`check`)'s rule table and `SPEC-13` (`explain`/`graph`)'s `graph` output contract both
  need updating to describe the new, config-driven mechanism and the `kind` field. The six type
  specs (`SPEC-6`/`9`/`10`/`16`/`17`/`18`) each need their own `pointer_fields`/`narrative_fields`
  declaration added to their `Schema` sections, matching whatever this repo's own config ends up
  declaring.
- The README's lineage section's `Feeds-into` example becomes literally true once this ships,
  closing `BUG-8`.
- Raised, not resolved by this decision: whether other backward-compatibility shims exist elsewhere
  in `urzua-core` for the same reason this one did (imagined future adopters, not real ones) --
  scoped as a separate research task before any bug gets filed for it.
- **Note (2026-11-05): the "zero real external adopters" cost holds through Phase 0 only.** It expires at the Phase 1
  switchover (`MILE-102`), when a real adopter's corpus exists and `RFC-49`'s compatibility contract (proposed in
  `ADR-77`) takes over; a break after that point is decided against a nonzero population. The Decision above is
  unchanged (`MILE-118`).

## Amendment (2026-09-09): three validation checks added; `narrative_fields` clarified

Review found four real gaps in this decision, resolved in full on RFC-23's own amendment (see
References). Consequence for this ADR:

- **Three new checks ship alongside the config-driven rules**, all Error severity, all part of this
  decision's own build, not deferred follow-up: `config.pointer-declaration-missing` (a type must
  declare **both** `pointer_fields` and `narrative_fields` explicitly, even as empty arrays --
  declaring only one and omitting the other must fail visibly, not read as "zero fields, on
  purpose"), `config.pointer-field-not-known` (a field named in either list must also appear in that
  type's `required_fields`/`known_fields` -- no automatic exception, checked explicitly), and
  `config.pointer-narrative-overlap` (a field cannot be declared in both lists for one type).
- **`narrative_fields` are staleness-checked by definition**, for any field in the list, not a
  `Blocked-on`-specific behavior incidentally reused. No change to the mechanism this ADR already
  decided -- `blocked_on_stale` generalizing to read its field list from config already implied
  this; this amendment only makes it explicit.
- `required_fields`/`known_fields` together remain the source of truth for "which fields a type's
  header may carry" -- membership in `pointer_fields`/`narrative_fields` was considered as an
  automatic exception to that and rejected, to keep one pair of config lists answering one question
  each rather than either silently extending the other.

## Amendment (2026-09-09): shipped

`MILE-90` built this decision in full: `pointer_fields`/`narrative_fields` on `RecordTypeConfig`,
all three rules reading them per type, the three new validation checks, `urzua graph`'s `kind`
field, and all six types declaring real values in `.urzua/config.toml`. `Embodiment: Verified` and
`Realized-by` added above, pointing at the code and tests this decision is realized by.

`blocked_on_stale` — named throughout this ADR's frozen Context/Decision/Consequences text, left
as originally decided rather than silently edited — shipped as `narrative_field_stale`
(`narrative-field.stale`): the same mechanism, renamed because it now generalizes to any
`narrative_fields` entry, not `Blocked-on` specifically. Every other reference in this document to
`blocked_on_stale` should be read as that rule under its current name.

## Amendment (2026-11-04): the configuration file is `.urzua/config.yaml`

*Status of this amendment: accepted 2026-11-04 by the maintainer (drafted 2026-11-04 from the drift review the maintainer approved). The Decision text above is unchanged.*

Wherever the Decision and its amendments say `.urzua/config.toml`, read `.urzua/config.yaml`.
`ADR-52` moved the configuration to YAML, and the `pointer_fields` and `narrative_fields`
declarations now live there. The shape of the declarations and the rest of the Decision are unchanged.

## References

- RFC-23 -- the proposal this ADR decides; the design's own open questions and rejected
  alternatives (`blocking_fields` as a name, a global field list, a backward-compat default), and
  its own amendment resolving the four gaps this ADR's amendment reflects.
- BUG-8 -- the concrete defect (README overstatement) this decision closes.
- BUG-9 -- the separate audit this decision's Consequences names as "raised, not resolved"; filed
  in the same PR as this ADR, it found legacy pre-`ADR-36` filename support as the one other real
  instance of the same shim pattern.
- ADR-38/39/43 -- the "declared, not voted or inferred" precedent this decision extends to pointer
  fields.
- RFC-20 -- the CLI/command taxonomy RFC, whose own table asserts pointer fields are "plain field
  names an org declares" -- this decision is what makes that literally true.
- MILE-90 -- the milestone that builds this decision, including the three checks this amendment
  adds.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | `Realized-by` locators pinned by content hash (`@sha256:`) after re-verifying each cited file against the Decision. **Why:** Re-verified against current code; maintainer decision 2026-11-04. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit c922db8ee. | **structural** |
> | 2026-11-04 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/config.rs` f8fe5e070ddb -> 251b70ec14f1, `rust/crates/urzua-core/src/rules/pointer.rs` 4248952dc30f -> f25158f2e1d0, `rust/crates/urzua-core/src/rules/narrative.rs` ab1662309844 -> 9fff38b1c9f6, `rust/crates/urzua-core/src/graph.rs` 1638485fe98c -> 33b1c278a9ff. Reason: Re-verified: pointer_fields and narrative_fields resolution is unchanged; diffs are severity, exclude_files and fixtures. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-04 | Amendment added: `.urzua/config.toml` is now `.urzua/config.yaml` (`ADR-52`). **Why:** drift review of 2026-11-04, approved by beauwilliams; proposed, pending maintainer acceptance. The Decision is unchanged. | **substantive** |
> | 2026-11-04 | Amendment of 2026-11-04 accepted. **Why:** Maintainer decision, 2026-11-04; the Decision text is unchanged. | **substantive** |
> | 2026-11-05 | Dated note added to Consequences: the zero-adopters premise holds through Phase 0 and expires at the Phase 1 switchover. **Why:** Maintainer decision, 2026-11-05 (`MILE-118`); the Decision text is unchanged. | **substantive** |
> | 2026-11-05 | Narrowed to the item the decision concerns; re-pin approved by the maintainer's 2026-11-04 drift decision (`ADR-74`, `MILE-185`): `config.rs` to `RecordTypeConfig` (lines 424-556); `config.rs` to `pointer_and_narrative_fields_round_trip` (lines 2895-2923). **Why:** an unrelated edit to a volatile file must not stale this pin again; the spans were chosen by the author of this change and not reviewed line by line by the maintainer. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/rules/pointer.rs` f25158f2e1d0 -> a66a71521611, `rust/crates/urzua-core/src/rules/narrative.rs` 9fff38b1c9f6 -> 132aafa70a19, `rust/crates/urzua-core/src/graph.rs` 33b1c278a9ff -> 4ac57e6f2018. Reason: Drift review of 2026-11-04, recommendation (a). pointer.rs and narrative.rs changed only by adding `action_source` to their findings (ADR-73). graph.rs changed in `explain` (governing records now read through the ADR-74 locator parser and carry `scope`) and in test fixtures that gained new config fields; the edge building from `pointer_fields` and `narrative_fields` is untouched. How the fields are read from config and resolved is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/config.rs#RecordTypeConfig` (lines 432-566) ff7dbbe0876e -> f5e9550b598f. Reason: RecordTypeConfig changed only in a doc comment (the MILE-118 dated note on the adopter premise); no field or behaviour changed, so the decision still holds **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/config.rs#RecordTypeConfig` (lines 443-583) f5e9550b598f -> cf828abc3ffa, `rust/crates/urzua-core/src/graph.rs` 4ac57e6f2018 -> c54bd8275562. Reason: BUG-74 renamed Waiver to Suppression (and load_waivers to load_suppressions) in this file's internals; the behaviour this record cites is unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (BUG-177, attested by this PR's review): re-pinned `rust/crates/urzua-core/src/config.rs#RecordTypeConfig` (lines 443-592) cf828abc3ffa -> 1e191cf68941. Reason: BUG-177 added the optional `effective_requires` field to `RecordTypeConfig`; the behaviour this record cites is unchanged **Why:** the pinned evidence changed and the PR review is the human re-verification that the claim still holds. | **structural** |
> | 2026-11-05 | Narrowed again, to the fields this decision is about (re-pinned by beauwilliams; the maintainer confirmed the narrowing on 2026-11-05): `config.rs#RecordTypeConfig` (lines 443-592, 1e191cf68941) is replaced by `config.rs#RecordTypeConfig::pointer_fields`, `config.rs#RecordTypeConfig::narrative_fields`, each a struct field addressed by the rust resolver's `struct` container and `indent` field member (`ADR-74`, `MILE-185`): `pointer_fields` (lines 522-534, 50b69240d87e), `narrative_fields` (lines 535-544, 55042eff1f13). Reason: both declarations and their doc comments (undeclared means skip; declaring either requires declaring both) are unchanged since the whole-struct pin. **Why:** a pin on the whole struct went stale on every new per-type option (`BUG-177` was the latest) although the claim here concerns only `pointer_fields` and `narrative_fields`; each span was read against the declaration it names. | **structural** |
> | 2026-11-05 | Cited the `ADR-74` amendment of 2026-11-05 (accepted) as the recorded decision for the resolver grammar the field pins above use: a list-valued `member` and a container that matches a struct, addressing a field by the `indent` end kind. No pin changed. **Why:** the review of #319 asked for the grammar extension to be a recorded decision; the narrowing itself was confirmed by the maintainer on 2026-11-05. | **structural** |
> | 2026-11-05 | Corrects the row of 2026-11-05 that narrowed the `config.rs#RecordTypeConfig` pin to `pointer_fields` and `narrative_fields` (`#319`). The line numbers that row quotes (`pointer_fields` 522-534, `narrative_fields` 535-544) are those of `config.rs` at `#295`; at the merge commit of `#319` the spans are `pointer_fields` 683-695 and `narrative_fields` 696-705. The narrowing was pure: the text of both fields is byte-identical since `#295` (the content hashes quoted by that row are unchanged), so no pin was stale and none is changed here. **Why:** an independent check of the re-pin reasons found the earlier row incomplete or inaccurate; corrected here without rewriting it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/pointer.rs` a66a71521611 -> e4d8db46f2e3, `rust/crates/urzua-core/src/rules/narrative.rs` 132aafa70a19 -> 1b08f9fad547, `rust/crates/urzua-core/src/graph.rs` c54bd8275562 -> 7d43a868a856. Reason: Read the diff of each pinned span since the old pin and the claim still holds. rules/pointer.rs: pointer_resolution gained an untracked-record argument and variant (BUG-13) and tests were added, the rules still read the declared pointer_fields; rules/narrative.rs: terminal statuses are now declared (the type's declared terminals, else the rule's terminal_statuses option) in place of a hardcoded set, matching the ADR's declared-field direction; urzua-core/src/graph.rs: added read_population (the population block and a notice naming records whose header cannot be read) and explain now reads evidence through evidence_text so Done-means clauses count, the relationship edges are unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Correction to the `Re-baselined` row of 2026-11-08 that re-pinned `rules/narrative.rs` 132aafa70a19 -> 1b08f9fad547. That reason said terminal statuses are now declared in place of a hardcoded set, which reads as a code change; the earlier row stays as written. The only change to `narrative.rs` since the old pin is its doc comment (+4/-5), which now says terminal statuses are declared (a type's declared status terminals, otherwise the rule's `terminal_statuses` option) where it said they were a small hardcoded set per record type. No code in the file changed. The claim still holds: `narrative_field_stale` reads the declared `narrative_fields` and judges a target by its declared terminals, falling back to `terminal_statuses`. **Why:** review of PR 433: the reason stated for a re-pin has to describe the real change between the old pinned content and the new. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/graph.rs` 7d43a868a856 -> 03f7eed2679c. Reason: Read the diff of each re-pinned file against main. core/src/graph.rs: Notice struct literals gained detail: None; no behaviour change. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/pointer.rs` e4d8db46f2e3 -> 2d164d27a2e9, `rust/crates/urzua-core/src/rules/narrative.rs` 1b08f9fad547 -> b34355340066. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: pointer.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged; narrative.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
