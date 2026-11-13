---
Stable-Id: 01M6NNNZJ92WC419AF3NQP471H
Status: Shipped
Embodiment: Verified
Realized-by: code:.urzua/config.yaml@sha256:323884ad0b6e1ee7c8485e6f75052f0266a742e29a2f43dababf665573a2a2cc, code:rust/crates/urzua-core/src/rules/roadmap.rs@sha256:aeadbc1397d70dc3f7ab6422c160c4dc009592306c8a3743da44ad020983fb02, test:rust/crates/urzua-cli/tests/condition_type_integration.rs@sha256:69d41012dc54852a87ebaf2a7999b04179023ffbf4a7712240899f7816a1ae0f, test:rust/crates/urzua-cli/tests/work_types_integration.rs@sha256:a319f14dc8d4b79971183ee16cf78de55366234377abe3d77e1fb3b5a0ebe7b2
Date: 2026-11-05
Version: '0.16'
Author: beauwilliams
Subject: 'The `condition` record type -- a dependency on a fact that is observed rather than built, declared in configuration alone.'
Parent: —
---
# SPEC-23 — The condition record type

## Purpose

A `condition` is a record for a fact nobody builds: an open question, or a signal such as an adopter
asking for a feature. `RFC-48` sorts what can stop a record into work, which a `Depends-on` link to a
record already expresses, and facts, which no record expressed. A `condition` is that record, so a
milestone, ADR or RFC that waits on a fact points at it with `Depends-on`, and `urzua roadmap`
(`SPEC-3`) and the derived `blocked` value read the wait like any other. The type is the first built
entirely from configuration: it uses a declared directory, prefix and identity, a revision log, a status vocabulary
with success and failure sets, `transitions`, `statuses.require` and `statuses.forbid`, and a required
section, and nothing else. The engine has
no knowledge of the name `condition`; the shape under any other name behaves the same.

## Schema

```yaml
record_types:
  condition:
    dir: "docs/conditions"
    prefix: "COND"
    identity: {from: filename, pattern: '^(?P<prefix>COND)-(?P<number>\d{4})-(?P<slug>.+)$'}
    revision_log: {marker: "**Revision log**", required: true}
    required_fields: ["Status", "Stable-Id"]
    header_shape: "yaml-frontmatter"
    known_fields: ["Met-by"]
    statuses:
      values: ["Open", "Met", "Dropped"]
      success: ["Met"]
      failure: ["Dropped"]
      initial: ["Open"]
      transitions:
        Open: ["Met", "Dropped"]
      require:
        - {when: ["Met"], fields: ["Met-by"]}
      forbid:
        - {when: ["Open"], fields: ["Met-by"]}
    sections: {from: headings, required: [{name: Signal}]}
    pointer_fields: ["Met-by"]
    narrative_fields: []
    spec: "SPEC-23"
```

This is the type as this repository's `.urzua/config.yaml` declares it.

Files are `COND-NNNN-slug.md` (`ADR-36`), with a four-digit padded number, and the H1 opens
`COND-NNNN`. `COND-1` and `COND-0001` name the same record.

| Field | Values | Notes |
|---|---|---|
| `Status` | `Open`, `Met`, `Dropped` | `Open` is the only initial state: a condition is filed `Open`, never already `Met`. It moves to `Met` or `Dropped`, and both are terminal (`ADR-72`); a mistaken closure is a new condition, not a reopening. |
| `Met-by` | a record reference | The record that shows the signal happened, such as the ADR that records the decision or the milestone that names the adopter's ask. Required once `Status` is `Met` (`status.required-field-missing`) and forbidden while it is `Open` (`status.forbidden-field-present`): it records the fact, and the fact has not happened yet, even where the record that will hold it is already known. A pointer field, so it must resolve to a record. |

`Met` is the type's success state and `Dropped` its failure state, which is all the sequencing rules
and the roadmap read.

## The Signal section

Every condition carries a `## Signal` section (`section.required`). It says exactly what counts as
met: the observable fact, who reports it and where it is recorded, written so that a reader who has
never seen the record can tell whether it has happened. Structural presence is all `check` can verify;
whether the Signal is precise is a reviewer's judgement, as for any section (`SPEC-1`).

## How a condition is read

- **`Depends-on` resolves to it.** A record of any type that declares `Depends-on` as a pointer field
  and takes part in the sequencing pair points at a condition as at any record (`ADR-65`,
  `cross_type: true`). `sequence.cycle` and `sequence.unresolved-blocker` examine the edge, and a
  reference to a condition that does not exist is reported (`pointer.resolution`). A target type that
  declares no status is reported by `relation.target-status-undeclared`, never read as satisfied
  (`ADR-60`).
- **Blocked.** A dependent of an `Open` condition is blocked and out of the ready set. A dependent of
  a `Met` one is not blocked. A dependent of a `Dropped` one stays out of the ready set and is listed
  in the roadmap's `needs_replan`, as a dependent of any failure-state record is.
- **Not work.** A condition is met or dropped, never started or ordered. A repository that leaves the
  type out of `roadmap.work_types` (`SPEC-3`) keeps the condition out of the roadmap's waves, ready
  set, critical path, records and groups, while it still gates its dependents as above and is listed
  under the roadmap's `waiting_on` until it is `Met` (`MILE-196`). The engine reads no type name; the
  setting lists the types that are work. Without the setting a condition is ordered like any record.
- **Who moves it.** A person moves a condition to `Met` or `Dropped`. An agent drafting the work that
  waits on it files the condition `Open` and does not decide it (`AGENTS.md`).

## What's deliberately not built

- **`condition.review-due` and the `Review-by` field.** A rule that reports a long-`Open` condition is
  new engine behaviour that `RFC-48` proposes and no accepted ADR covers; it needs its own ADR first.
- **`sequence.failed-dependency`.** The same: a rule reporting a dependent of a `Dropped` condition is
  a new rule, and the roadmap's `needs_replan` is the only place a dropped condition surfaces today.
- **A condition for a decision.** `RFC-48` points a wait on a decision at the RFC or a decide-milestone
  instead. The four waits that `MILE-187` migrated (`MILE-13`, `MILE-21`, `MILE-23`, `MILE-30`) were
  all named for migration by the maintainer, so two of them are conditions on a decision (`COND-1`,
  `COND-4`) whose `Signal` cites it.
- **`Depends-on` on this repository's ADRs and RFCs.** The type resolves from any record that
  declares the field; this repository's `adr` and `rfc` types do not declare it, so only milestones
  point at conditions here.

## References

- `RFC-48` — the roadmap is computed from records; proposes the condition type.
- `ADR-72` — allowed status transitions, declared per type.
- `ADR-60` — a cross-record status read is gated on the target type declaring status.
- `ADR-65` — relation pairs are declared top-level.
- `ADR-36` — filenames carry the type prefix.
- `SPEC-3` — configuration, including `roadmap` and the sequencing pair.
- `MILE-187` — the milestone that declared the type and migrated the four waits.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Initial spec. **Why:** `MILE-187`, maintainer decision of 2026-11-05: the `condition` type of `RFC-48` is declared in configuration so waits on facts are structured and the computed roadmap sees them. | **structural** |
> | 2026-11-05 | `Met-by` is unset while `Open`. **Why:** review of `MILE-187`: `Met-by` names the record that shows the signal happened, so naming a record before then asserts evidence that does not exist; `COND-1` no longer sets `Met-by: RFC-9`. | **structural** |
> | 2026-11-05 | Bumped to `0.2`. A condition is read as non-work when a repository leaves it out of `roadmap.work_types`: it gates but is not ordered, and is listed under `waiting_on`. **Why:** `MILE-196` builds the `ADR-82` Amendment of 2026-11-05, which this repository's own configuration uses so `COND-1` to `COND-4` leave the ready set. | **substantive** |
> | 2026-11-07 | Bumped to `0.3`. Checked against `.urzua/config.yaml` and the tests, and corrected: the schema excerpt now shows the type as declared (`revision_log`, `Stable-Id` required, and `statuses.forbid` of `Met-by` while `Open`), the Purpose lists what the type uses, and the `Met-by` row says the field is forbidden while `Open`, with its rule (`status.forbidden-field-present`), where it said only that it is left unset. **Why:** the maintainer approved moving this spec to `Shipped` (2026-11-07), which required verifying each section against the built type. | **substantive** |
> | 2026-11-07 | Bumped to `0.4`. Status changed from Draft to Accepted. **Why:** Maintainer decision, 2026-11-07: the maintainer approved moving this spec to `Shipped`, and the lifecycle moves one step at a time (`Draft` to `Accepted` to `Shipped`). | **substantive** |
> | 2026-11-07 | Bumped to `0.5`. Realized-by names the implementing code and tests, ready for `Shipped` (`MILE-7`). **Why:** Maintainer decision, 2026-11-07: moving this spec to `Shipped` requires `Embodiment` and `Realized-by`. | **structural** |
> | 2026-11-07 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Embodiment` from `Realized-by: code:.urzua/config.yaml, code:rust/crates/urzua-core/src/rules/roadmap.rs, test:rust/crates/urzua-cli/tests/condition_type_integration.rs, test:rust/crates/urzua-cli/tests/work_types_integration.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-07 | Bumped to `0.6`. Status changed from Accepted to Shipped. **Why:** Maintainer decision, 2026-11-07: its `Realized-by` evidence computes `Verified`, and each section was checked against the built code and tests. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 4 locators pinned, 4 file-level, 0 scoped. Evidence: .urzua/config.yaml@sha256:151830e5926d, rust/crates/urzua-core/src/rules/roadmap.rs@sha256:c00dd724a3cd, rust/crates/urzua-cli/tests/condition_type_integration.rs@sha256:fbf4c6b9927d, rust/crates/urzua-cli/tests/work_types_integration.rs@sha256:a319f14dc8d4 Reason: Pinned the files as they are now after checking each against the record it evidences: config.yaml and the own-config test still declare the adr, rfc, spec and condition types that SPEC-16, SPEC-17, SPEC-18 and SPEC-23 describe, with spec_shipped_integration.rs, rules/roadmap.rs, condition_type_integration.rs and work_types_integration.rs behind them, check.rs, rules.rs, check_integration.rs and fail_on_integration.rs still implement urzua check (SPEC-2), the workflows, the install-knope action and the release and gate test scripts still make up the pipeline SPEC-20 describes, the Makefile and checks.yml still dispatch the repository layout of SPEC-21, the harness modules, the bundled hook script and the harness integration tests still implement the request, verdict and adapter layout of SPEC-24, config.rs and the own-config and field-levels tests still implement SPEC-3, and acceptance.rs and acceptance_property.rs still implement SPEC-4. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Bumped to `0.7`. The `code:` and `test:` locators of `Realized-by` are now hash-pinned by `urzua reverify --pin-only` (the `Pinned` row above) and, where the record stated `Drift detected`, its `Embodiment` was recomputed (the tool-authored row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97` section 10: a claim is renewed only over pinned code and test evidence, so the corpus is pinned once before expiry is enabled. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 151830e5926d -> 0b59c6dfc8b9, `rust/crates/urzua-core/src/rules/roadmap.rs` c00dd724a3cd -> 5e9616cd49c2. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. config.yaml: config.yaml gained Kind among the milestone type's known fields and a kind declaration (build and decision) under statuses.delivery.items (ADR-96 amendment of 2026-11-07), and every other declaration the record relies on is unchanged; roadmap.rs: the delivery context is now built with the record, one added argument, and the roadmap rule is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0b59c6dfc8b9 -> 0a00038a6388. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: this change declares status.done-items-shape (error), status.waived-item-stale (warn) and embodiment.claim-renewal-unattested (warn) and declares relation.cross-type-disallowed, section.shape and diff.source-and-records-mixed as off; every other declaration is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.8`. The pins this spec's `Realized-by` holds on files changed in this change were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. **Why:** a revision-log entry that adds a structural change needs a `Version` bump (`revision-log.version-bump-required`). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0a00038a6388 -> 3d51c2fac8b7. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: only the comment above the three rules declared off changed, to say they report `not-enabled` exactly as an undeclared rule does; no declaration changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/roadmap.rs` 5e9616cd49c2 -> aeadbc1397d7. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: roadmap.rs: split compute_roadmap into the Order struct and its methods (exclusions, assign_waves, record_entries, critical_path and others) and the scoped_graph, read_standings and find_cycles helpers. Behaviour unchanged, outputs byte-identical (check, check --base and fix --rebaseline JSON compared before and after). **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 3d51c2fac8b7 -> 653bb0f54ca6. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: the only change is a new optional pending list (two placeholder patterns) under record_types::milestone statuses.delivery.items; nothing else in the file changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 653bb0f54ca6 -> 594d115541e3. Reason: config.yaml: read the diff since the previous pin; the only change is one added line, outside_writes: shared, under the harness key (this repository declares the new harness.outside_writes option, ADR-89 amendment of 2026-11-09). No record type, rule, status, relation or other declaration this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.11`. The pin this spec's `Realized-by` holds on `.urzua/config.yaml` was re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 594d115541e3 -> 2f0e023ed5a5. Reason: Read the diff of each pinned file or span since the old pin against what this record evidences. .urzua/config.yaml: embodiment.claim-expired gains `drift: {code: block, test: block, spec: acknowledge}` and its explanatory comment; nothing else in the file changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.12`. The pin this spec's `Realized-by` holds on `.urzua/config.yaml` was re-baselined (the `Re-baselined` row above) after the config gained the `drift` declaration under `embodiment.claim-expired`; no behaviour this spec describes changed. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 2f0e023ed5a5 -> cbb1c67c40a0. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: read the diff since the old pin; the only change is the added key authority.decider_table: base with its two-line comment (this repository declares the new decider_table option, ADR-89 amendment of 2026-11-09). No record type, rule, status, relation or other declaration this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.13`. The pins this spec's `Realized-by` holds on `.urzua/config.yaml` were re-baselined (the `Re-baselined` row above) after the config gained `authority.decider_table: base`; no behaviour this spec describes changed. **Why:** the version-bump rule requires a bump beside a structural revision row. | **structural** |
> | 2026-11-09 | Bumped to `0.14`. Re-pin rows in this revision log were reworded to name who re-verified, matching the tool's wording since 2026-11-09; hashes, reasons and dates are unchanged. **Why:** Maintainer decision, 2026-11-09. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` cbb1c67c40a0 -> 21e9ab57a37a. Reason: Read the diff of .urzua/config.yaml: the authority section gains `waivable: never` and a two-line comment; no other key changed. The pinned evidence still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.15`. Evidence pins on `.urzua/config.yaml` were re-pinned (row above) after the config's authority section gained `waivable: never`; the specification's content is unchanged. **Why:** `ADR-89` amendment of 2026-11-09 (gate waivers). | **substantive** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 21e9ab57a37a -> dd656d471f38. Reason: Read the diff of each pinned file since the previous pin. config.yaml gained section.duplicate, section.level-skipped and section.order at error under rules, and unique_headings, no_skipped_levels and order (from: template) under each type's sections, with depth: 2 added where a type had none; the change is additive; the claim holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Bumped to `0.16`. The pinned `.urzua/config.yaml` evidence was re-baselined for `MILE-104`; the text of this specification is unchanged. **Why:** the revision log gained a structural re-baseline row, which `revision-log.version-bump-required` asks a spec to answer with a version bump. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` dd656d471f38 -> 323884ad0b6e, `rust/crates/urzua-cli/tests/condition_type_integration.rs` fbf4c6b9927d -> 69d41012dc54. Reason: `.urzua/config.yaml` gained only the `drift_baseline: {ref: v0.3.0}` declaration and its comment under `embodiment.consistency`; no other declaration changed. condition_type_integration.rs: the fixture takes the repository's config through `common::own_config_for_fixture`, which drops the `drift_baseline` line a throwaway repository cannot resolve; the assertions are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
