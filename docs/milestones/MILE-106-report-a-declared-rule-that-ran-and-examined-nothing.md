---
Stable-Id: 01M2YMFN53X5QB064ZV60ZSQJT
Status: Done
Phase: '0'
Track: schema-governance
Realized-by: test:rust/crates/urzua-core/src/rules/type_scope.rs#a_scope_reaching_nothing_is_reported_observed_failing@sha256:6aa76d4b05ffe2115bf2fbd6081fba0061b319485748e8b945a2932bc43fec77, code:rust/crates/urzua-core/src/rules/type_scope.rs#config_scope_matches_nothing@sha256:efe532e758481bae74c46f9941419de4e161c9e8de2d468e8a1b3edeaf24f754, test:rust/crates/urzua-cli/tests/check_integration.rs#a_config_level_finding_names_a_repository_relative_config_path_observed_failing@sha256:c99cb628975b7ba8dffc0118b9eea84b33feae5372ce8480f6e68ab27f4681d2, test:rust/crates/urzua-core/src/rules/type_scope.rs#a_rule_with_nothing_written_yet_is_not_reported@sha256:9f41325bad3978ef05716e507d35f03755a352167e33ad526299ea77857d2770, code:.urzua/config.yaml@sha256:b6ecfd818489fca2a671cb4d5a140bc5d066fc6e3285b60bf7216ed00d8d6d1c, code:.github/workflows/checks.yml@sha256:ad6ab57882dfac0d3a819e549bccd9ba6f81abafe0699d09d26f828a5714398b
---
# 106 — Report a declared rule that ran and examined nothing

## What

An opt-in rule that reports a **declared** rule which ran and examined nothing, so the engine detects
in itself the defect class `ADR-55` names: a check that reports success without looking.

The input is already there. `check`'s report carries a `RuleExecution` per rule with
`records_examined` and `status`, and the rule reads that table rather than the corpus.

The discriminator is the work. `records_examined: 0` is legitimate when a corpus holds nothing the
rule addresses -- a declared type not yet used, a scope matching nothing -- and reporting those would
make the rule noise, which is the same as not having it. `type.dir-matches-nothing` already draws the
equivalent line, treating an absent directory as a misdeclaration and an empty one as a type not yet
used, and is the shape to follow: report when the corpus contained something the rule should have
examined and it examined none of it.

## Why, restated: this is the feedback loop that makes the configuration usable

The original framing below argues from `ADR-55` -- the engine should detect in itself the defect class
it exists to catch. True, and it undersells the milestone.

`ADR-53` decided that *"a repository's governance lives in its configuration and the engine ships only
general-purpose primitives"*, motivated by a foreign corpus receiving nine errors it never asked for.
The `README` puts it as *"a record engine, not a record format: you declare what a record type looks
like, and Urzua validates against that declaration."*

An engine you configure must tell you when your configuration is wrong. When a declared rule produces
nothing there are three causes and three **opposite** responses:

| state | what is true | what the adopter does |
|---|---|---|
| the corpus has none of this yet | legitimate, and common on a fresh adoption | nothing |
| the records are there and unreadable | the corpus is malformed | fix the records |
| the declared scope matches nothing | **the configuration is wrong** | fix the config |

Today all three arrive as one number pair and the adopter cannot tell which applies.

The third row is `BUG-61`: `init` wrote a `prefix` that matched nothing, five rules examined zero
records, and nothing said so. **That failure mode is unique to being configurable** -- a linter with
compiled-in rules cannot have it -- which makes it the most important diagnostic this engine has and
the one it currently lacks.

Read this way, the family is not eight lint bugs. It is the engine having no vocabulary for the
outcomes of its own configuration, surfacing eight times and from both directions: `BUG-84` and
`BUG-99` are a correct config reported as failure, `BUG-61` and `BUG-88` a wrong config reported as
success.

## What the design already specified, and the implementation dropped

The `0.4.0` population work was planned with **four** candidate states:

> `Examined` (verdict reached) · `Absent` (the declared slot or section is not there) · `Unreadable`
> (present, header did not parse) · `OutOfPopulation` (never a candidate, counted nowhere).
> `eligible` counts the first three; `examined` the first. **Keep all four internally so the collapse
> happens once, at serialization.**

What shipped is `Outcome::Examined | NotExamined`, and its doc comment carries the conflation openly:
*"the declared slot is absent, **or** the header it needed did not parse."*

So the field this milestone needs in order to discriminate was specified and then collapsed in the
code rather than at the wire. This is `BUG-40` one level down -- that bug was one number holding three
denominators, fixed by attaching the unit; this is one `NotExamined` holding several causes, and it
needs the reason attached. Every guard attempted so far has been a heuristic reconstructing
information the design said to keep.

**So the first work here is restoring the four states**, not choosing a fifth predicate.

*(Shipped differently, and the difference matters. Restoring the four states surfaced a fifth the plan
did not have -- `OutOfScope`, for a record the configuration cannot reach -- and it is the state this
milestone actually needed. It is carried on the wire as a third `population` field, `out_of_scope`, so
the report contract does move after all. The predicate is `out_of_scope > 0`, not the
`eligible > 0 && examined == 0` proposed below: that form fires on a corpus that has simply not
written a revision log yet, which is legitimate and which no configuration change fixes. The design
below is kept as filed, and is historical from here down.)*

It shares a root cause with `RFC-39` one layer up: a type that cannot express a distinction forces
every consumer to guess it.

**The concrete shape is a second number.** A rule reports what it examined; it cannot say what it was
*eligible* to examine. With both, the signal is `eligible > 0 && examined == 0` rather than
`examined == 0`:

| rule | eligible population | zero examined means |
|---|---|---|
| `relation.supersession-reciprocity` | records carrying a supersession field | the corpus has none -- clean |
| `filename.title-consistency` | records whose filename yields an identifier | the corpus has records and none yield one -- `BUG-61` |

So each rule declares its population predicate, not only its count. `RuleExecution` already carries
`records_examined` and `scope` (`BUG-81`, `BUG-83`); `records_eligible` is the missing third.

*(Historical. `records_examined` and `scope` were deleted by `BUG-40`'s fix, and the shipped shape is
`population` with `eligible`, `examined` and `out_of_scope` in a named unit.)*

`BUG-84` is the evidence for doing it here rather than approximating it elsewhere. Guarding on
`examined > 0` alone made `audit` unsatisfiable on exactly the configuration `init` generates, while
guarding on "a record-scoped rule ran" -- the form shipped -- accepts a rule that could not address
anything. Both are proxies for this distinction, and each is wrong in one direction.

## Why

Roughly twenty instances of this defect were found across seven review rounds of the 0.4.0 diff, and
**every one was visible in the tool's own output when it shipped**. `records_examined` was introduced
precisely to distinguish a rule that ran over nothing from one that ran cleanly, and nothing has ever
read it.

Rounds 2, 4, 6 and 7 each found a defect introduced by the previous round's fix. Review catches this
class, but only after shipping and at the cost of a full adversarial pass each time. A rule that
reads the table turns that into a check the tool runs on itself -- which is Phase 0's thesis, the
engine bootstrapped with the engine, applied to the engine's own blind spot.

Filed now rather than later because the rate is not falling: round 7 found three instances in code
round 6 wrote to fix two others.

## Done means

- The opt-in rule `config.scope-matches-nothing` reports a declared rule whose configuration cannot reach its candidates (the `out_of_scope` state); a fixture covers it. — evidence: test:rust/crates/urzua-core/src/rules/type_scope.rs#a_scope_reaching_nothing_is_reported_observed_failing@sha256:6aa76d4b05ffe2115bf2fbd6081fba0061b319485748e8b945a2932bc43fec77, code:rust/crates/urzua-core/src/rules/type_scope.rs#config_scope_matches_nothing@sha256:efe532e758481bae74c46f9941419de4e161c9e8de2d468e8a1b3edeaf24f754, test:rust/crates/urzua-cli/tests/check_integration.rs#a_config_level_finding_names_a_repository_relative_config_path_observed_failing@sha256:c99cb628975b7ba8dffc0118b9eea84b33feae5372ce8480f6e68ab27f4681d2
- `config.scope-matches-nothing` stays silent when the corpus simply holds nothing the rule addresses (a declared type not yet used); a fixture covers it. — evidence: test:rust/crates/urzua-core/src/rules/type_scope.rs#a_rule_with_nothing_written_yet_is_not_reported@sha256:9f41325bad3978ef05716e507d35f03755a352167e33ad526299ea77857d2770
- The rule is declared at `warn` in urzua's own `.urzua/config.yaml` and reports nothing on this repository. — evidence: code:.urzua/config.yaml@sha256:b6ecfd818489fca2a671cb4d5a140bc5d066fc6e3285b60bf7216ed00d8d6d1c, code:.github/workflows/checks.yml@sha256:ad6ab57882dfac0d3a819e549bccd9ba6f81abafe0699d09d26f828a5714398b

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-21 | Marked the pre-implementation design historical where it contradicts what shipped. **Why:** the section said nothing in the report contract moves and named `records_eligible` as the missing field; the release carries a third serialized field, `out_of_scope`, and fires on that rather than on `eligible > 0 && examined == 0`. A milestone whose own body describes a design the code does not have is the drift `AGENTS.md` requires grepping for. | **substantive** |
> | 2026-09-21 | `Status: Planned` -> `Done`. Shipped as `config.scope-matches-nothing`, declared `warn` on this repository. **Why:** the discriminator this milestone asked for is `out_of_scope`, the candidate state restored alongside it -- a rule fires when the configuration cannot reach its candidates, and stays silent when the corpus simply has not written them. On a prefixless corpus `identity.collision` and `revision-log.change-class-required` both report `2/0`; only the first is reported, which is the distinction the milestone existed to draw. Silent on `Unreadable`, because `header.required-fields` already reports the parse error per record. | **substantive** |
> | 2026-09-21 | Restated the Why as configuration feedback, and recorded that the discriminator was specified and dropped. **Why:** designing the rule stalled because every candidate predicate felt arbitrary, and the reason is that `Outcome` collapsed the plan's four states into two, so the milestone was trying to answer with two states a question the design said needed four. Reframing also corrects the scope: this is not only `ADR-55` self-detection, it is the diagnostic an engine owes an adopter who can misconfigure it -- the failure mode `BUG-61` found and the one a compiled-in linter cannot have. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: .urzua/config.yaml@sha256:151830e5926d, .github/workflows/checks.yml@sha256:4afa5fe0db59 Reason: Pinned the files as they are now after checking each against the record it evidences: claim.rs and claim_field.rs still read a changeset's declared closes field as a claim (BUG-66), release-invariants.test.sh still plants a release-prep version bump and an unmatched level and asserts the guards fail, and checks.yml now runs it on the unattended path (BUG-92, BUG-93, BUG-96), new.rs and new_record.rs still take the configured header shape over a template (MILE-1), filename.rs still compares the filename's type prefix and padded number with the H1 (MILE-10), config.yaml still declares config.scope-matches-nothing and checks.yml runs the gate (MILE-106, MILE-110), and header.rs, narrative.rs, pointer.rs, field.rs, fix.rs, the Makefile and checks.yml still read the declared relation field names with relation_field_literals.rs failing on a restated literal (MILE-107, MILE-109). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 151830e5926d -> 0b59c6dfc8b9, `.github/workflows/checks.yml` 4afa5fe0db59 -> 3bd0a6795184. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. config.yaml: config.yaml gained Kind among the milestone type's known fields and a kind declaration (build and decision) under statuses.delivery.items (ADR-96 amendment of 2026-11-07), and every other declaration the record relies on is unchanged; checks.yml: the single rust job became parallel lint, build, three nextest test shards, four acceptance shards, an acceptance-gate and a result job (ADR-98), and every step the record relies on (fmt, clippy, deny, the records gate, the release guards, the rule table check, the gate action tests and harness status) is still run. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0b59c6dfc8b9 -> 0a00038a6388. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: this change declares status.done-items-shape (error), status.waived-item-stale (warn) and embodiment.claim-renewal-unattested (warn) and declares relation.cross-type-disallowed, section.shape and diff.source-and-records-mixed as off; every other declaration is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0a00038a6388 -> 3d51c2fac8b7. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: only the comment above the three rules declared off changed, to say they report `not-enabled` exactly as an undeclared rule does; no declaration changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/checks.yml` 3bd0a6795184 -> a201f4e57f5f. Reason: checks.yml: the only change since the pinned version is the four tool-download curl lines (cargo-deny, cargo-machete), which gained --retry 5 --retry-all-errors --retry-delay 5 and changed -sSLfo to -sSLf with a separate -o (#441); no step, job or check changed, so the evidence still holds **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 3d51c2fac8b7 -> 653bb0f54ca6. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: the only change is a new optional pending list (two placeholder patterns) under record_types::milestone statuses.delivery.items; nothing else in the file changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/type_scope.rs#config_scope_matches_nothing` (lines 261-318) 25f8c409bcd7 -> efe532e75848. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: type_scope.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 653bb0f54ca6 -> 594d115541e3. Reason: config.yaml: read the diff since the previous pin; the only change is one added line, outside_writes: shared, under the harness key (this repository declares the new harness.outside_writes option, ADR-89 amendment of 2026-11-09). No record type, rule, status, relation or other declaration this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 594d115541e3 -> 2f0e023ed5a5. Reason: Read the diff of each pinned file or span since the old pin against what this record evidences. .urzua/config.yaml: embodiment.claim-expired gains `drift: {code: block, test: block, spec: acknowledge}` and its explanatory comment; nothing else in the file changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 2f0e023ed5a5 -> cbb1c67c40a0. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: read the diff since the old pin; the only change is the added key authority.decider_table: base with its two-line comment (this repository declares the new decider_table option, ADR-89 amendment of 2026-11-09). No record type, rule, status, relation or other declaration this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` cbb1c67c40a0 -> 21e9ab57a37a. Reason: Read the diff of .urzua/config.yaml: the authority section gains `waivable: never` and a two-line comment; no other key changed. The pinned evidence still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-10 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/checks.yml` a201f4e57f5f -> ad6ab57882df. Reason: checks.yml gained one additive, report-only step in the build job (no stale evidence pins: runs fix --rebaseline without --apply and fails unless the report status is ok); no existing step changed, so the claim still holds **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 21e9ab57a37a -> dd656d471f38. Reason: Read the diff of each pinned file since the previous pin. config.yaml gained section.duplicate, section.level-skipped and section.order at error under rules, and unique_headings, no_skipped_levels and order (from: template) under each type's sections, with depth: 2 added where a type had none; the change is additive; the claim holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` dd656d471f38 -> 323884ad0b6e. Reason: `.urzua/config.yaml` gained only the `drift_baseline: {ref: v0.3.0}` declaration and its comment under `embodiment.consistency`; no other declaration changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 323884ad0b6e -> b6ecfd818489. Reason: `.urzua/config.yaml`: only the comment above the `drift_baseline: {ref: v0.3.0}` declaration was reworded (drift a record already had at the ref); the declaration and every other rule are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
