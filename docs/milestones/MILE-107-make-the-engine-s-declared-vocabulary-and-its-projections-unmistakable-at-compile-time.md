---
Stable-Id: 01M31GYJ2C8HXXB3J7W5AQYR6G
Status: Done
Phase: '1'
Track: schema-governance
Implements: RFC-39
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/values.rs#FieldName@sha256:2439138e335e66b788ca09228afc9d65f6b905c134ff3e4f6be3a92ef7259146, code:rust/crates/urzua-core/src/values.rs#RecordId@sha256:69ee1d61e1cbdd00c6ef7cd7d39e6f216164d47f5bc8849f37ac26635d2f6856, test:rust/crates/urzua-core/src/values.rs#field_name_compares_exactly@sha256:f6081fa549c6345549f582e86b3a32d772c63e97e2ae4268c3c141d54d9b7d1c, test:rust/crates/urzua-core/src/values.rs#record_id_compares_by_numeric_value@sha256:2e7b3ad606e780cf911ea4ca968583e6965edd3139e656283974644fb202a570, code:rust/crates/urzua-core/src/rules/header.rs@sha256:8daab0cdc431844d3709d53363612118131a256706059621d64448e5682f5511, code:rust/crates/urzua-core/src/rules/narrative.rs@sha256:b34355340066c63614438add0c4da8230a9aafd84c991834a59414eaa09eaa74, code:rust/crates/urzua-core/src/rules/pointer.rs@sha256:2d164d27a2e9d20a589adeee671eaae8db4d5e6529b3e705e9df1d1b0ed6e5ae, code:rust/crates/urzua-core/src/rules/field.rs@sha256:9a5be442821afdf5384e645955f304981803f39f6a749bd57567b41c2e314ec8, spec:docs/adr/ADR-59-domain-values-the-adopter-declares-are-types-not-bare-strings.md, test:rust/crates/urzua-cli/tests/relation_field_literals.rs#no_relation_field_literal_escapes_config_rs@sha256:ecbcb243a3aa835749ec347d6fe2de73f75b40c2b00f95c31cb18e3cf1c44856, test:rust/crates/urzua-cli/tests/relation_field_literals.rs#the_check_is_observed_failing_on_a_planted_violation@sha256:b257e0b6f38cccc990203d5420fb60339398c743e17e21e2994697b98bccf356, code:.github/workflows/checks.yml@sha256:e166badec9b1eac84bcd921aaf137fc7552a48d8a35b96a4c36fac4bd2b3cb84, code:Makefile@sha256:ccc04e61c2e32aad9ccd10166a12df24d31e8ccb1b24233f01c566d7058f382a
---
# 107 — Make the engine's declared vocabulary and its projections unmistakable at compile time

## What

`RFC-39`'s implementation, in two halves.

**Values.** `FieldName`, `RecordId`, `StatusValue` -- types whose `Eq`/`Hash` *is* their comparison
rule, so the rule is stated once instead of at every call site.

**Projections.** The ten rule signatures taking a bare collection derived from configuration converge
on `&Config`, which fifteen rules already do, or on named projection types. The rule to hold: a bare
collection derived from configuration does not cross a function boundary.

## Why

Not theoretical. Both halves shipped a defect in the `0.4.0` work, on the same day.

**Values.** `ADR-57` decided field names compare exactly. Implementing that one decision took three
passes, because the comparison lived at six call sites across three modules -- and between the first
pass and the last, `header.field-set-consistency` reported a field undeclared while
`header.required-fields` read the same field as satisfied. **The engine gave two answers about one
field**, caught only because a reviewer read past the diff.

`normalize_id` is the same shape and is load-bearing today: thirteen hand-applied call sites, correct
at all thirteen, with nothing enforcing the fourteenth. Missing it reintroduces `BUG-2`.

**Projections.** `field.untrimmed-value` was wired to `known_fields_by_type` where it needed
`declared_fields_by_type`. Both are `&HashMap<String, HashSet<String>>`; the first is absent for a type
declaring no `known_fields`, deliberately, and the second is populated for every type. Identical to the
compiler, opposite in meaning, and the distinction recorded in a comment. The rule silently skipped
every type declaring only `required_fields` -- a rule added to catch values nothing else checks, wired
so it would not check them.

Six further parameters are `&HashMap<String, Vec<String>>`, two of them adjacent arguments of
`pointer_resolution`. Swapping them compiles.

## The evidence that a type would have held

`Population` has private fields, one constructor and a `compile_fail` doctest. **It has never been
wrong.** Over the same release, `records_examined` -- a bare `usize` maintained beside a filter -- was
wrong four times (`BUG-78`, `BUG-81`, `BUG-83`, `BUG-90`), and `eligible`, derived structurally from
the candidate list, has never been wrong once.

The codebase already knows this. Fourteen enums exist where a string would have done. What it has not
done is apply the same reasoning to values the *adopter* declares, which is what `ADR-53` turned into
an open set -- and `urzua_core::Status` and `Embodiment` sit in `lib.rs` declared and never
constructed, the fossil of that handover.

## Sequencing

`FieldName` first: no existing normalization to preserve, and `ADR-57` already specifies its
comparison, so it is the cheap proof the pattern works.

The projections second, by converging on `&Config`. `RFC-39` decides this rather than leaving it open:
the safe pattern is already the majority in `rules.rs`, so it is demonstrated rather than proposed.

`RecordId` last and carefully. Thirteen working call sites, and the risk is a conversion that
normalizes where the old code did not or fails to where it did. Every current caller wants a test
pinning present behaviour **before** the type is introduced.

## Done means

- `FieldName` and `RecordId` exist as types whose `Eq` and `Hash` state the comparison rule once. — evidence: code:rust/crates/urzua-core/src/values.rs#FieldName@sha256:2439138e335e66b788ca09228afc9d65f6b905c134ff3e4f6be3a92ef7259146, code:rust/crates/urzua-core/src/values.rs#RecordId@sha256:69ee1d61e1cbdd00c6ef7cd7d39e6f216164d47f5bc8849f37ac26635d2f6856, test:rust/crates/urzua-core/src/values.rs#field_name_compares_exactly@sha256:f6081fa549c6345549f582e86b3a32d772c63e97e2ae4268c3c141d54d9b7d1c, test:rust/crates/urzua-core/src/values.rs#record_id_compares_by_numeric_value@sha256:2e7b3ad606e780cf911ea4ca968583e6965edd3139e656283974644fb202a570
- The ten rule signatures that took a bare collection derived from configuration take `&Config`. — evidence: code:rust/crates/urzua-core/src/rules/header.rs@sha256:8daab0cdc431844d3709d53363612118131a256706059621d64448e5682f5511, code:rust/crates/urzua-core/src/rules/narrative.rs@sha256:b34355340066c63614438add0c4da8230a9aafd84c991834a59414eaa09eaa74, code:rust/crates/urzua-core/src/rules/pointer.rs@sha256:2d164d27a2e9d20a589adeee671eaae8db4d5e6529b3e705e9df1d1b0ed6e5ae, code:rust/crates/urzua-core/src/rules/field.rs@sha256:9a5be442821afdf5384e645955f304981803f39f6a749bd57567b41c2e314ec8
- `StatusValue` was decided against in `ADR-59`, not deferred. — evidence: spec:docs/adr/ADR-59-domain-values-the-adopter-declares-are-types-not-bare-strings.md
- `relation_field_literals.rs` fails when a rule restates a literal. — evidence: test:rust/crates/urzua-cli/tests/relation_field_literals.rs#no_relation_field_literal_escapes_config_rs@sha256:ecbcb243a3aa835749ec347d6fe2de73f75b40c2b00f95c31cb18e3cf1c44856, test:rust/crates/urzua-cli/tests/relation_field_literals.rs#the_check_is_observed_failing_on_a_planted_violation@sha256:b257e0b6f38cccc990203d5420fb60339398c743e17e21e2994697b98bccf356
- The full test suite, `make ci` and `check` over a real corpus pass. — evidence: code:.github/workflows/checks.yml@sha256:e166badec9b1eac84bcd921aaf137fc7552a48d8a35b96a4c36fac4bd2b3cb84, code:Makefile@sha256:ccc04e61c2e32aad9ccd10166a12df24d31e8ccb1b24233f01c566d7058f382a

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-21 | Projection approach fixed to `&Config` per `RFC-39`'s decision, rather than left as two options. | **structural** |
> | 2026-09-21 | Filed. **Why:** `RFC-39` was a `Draft` with no work item, and both halves of it caused a shipped defect during the `0.4.0` review on the day it was filed. Filed so the work is scheduled rather than left as a proposal, and so the evidence stays attached to it while it is fresh. | **substantive** |
> | 2026-09-22 | `Status: Planned` → `Done`. **Why:** `ADR-59` decided the scope precisely (`FieldName` and `RecordId` built; the `&Config` convergence done for all ten signatures; `StatusValue` decided against, not deferred) and both were implemented and verified: full test suite, `make ci`, and a real-corpus `check` run reporting the same 70 findings before and after. `RecordTypeConfig`'s own fields stay `Vec<String>` -- narrower than this record's original "Values" section proposed, corrected in `ADR-59` before implementation once counting construction sites found over ninety of them, almost all test fixtures never implicated in a comparison bug. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 6 locators pinned, 6 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/rules/header.rs@sha256:2ee979c62600, rust/crates/urzua-core/src/rules/narrative.rs@sha256:1b08f9fad547, rust/crates/urzua-core/src/rules/pointer.rs@sha256:e4d8db46f2e3, rust/crates/urzua-core/src/rules/field.rs@sha256:c5fd645b5179, .github/workflows/checks.yml@sha256:4afa5fe0db59, Makefile@sha256:fb9fda4c8508 Reason: Pinned the files as they are now after checking each against the record it evidences: claim.rs and claim_field.rs still read a changeset's declared closes field as a claim (BUG-66), release-invariants.test.sh still plants a release-prep version bump and an unmatched level and asserts the guards fail, and checks.yml now runs it on the unattended path (BUG-92, BUG-93, BUG-96), new.rs and new_record.rs still take the configured header shape over a template (MILE-1), filename.rs still compares the filename's type prefix and padded number with the H1 (MILE-10), config.yaml still declares config.scope-matches-nothing and checks.yml runs the gate (MILE-106, MILE-110), and header.rs, narrative.rs, pointer.rs, field.rs, fix.rs, the Makefile and checks.yml still read the declared relation field names with relation_field_literals.rs failing on a restated literal (MILE-107, MILE-109). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/checks.yml` 4afa5fe0db59 -> 3bd0a6795184, `Makefile` fb9fda4c8508 -> ccc04e61c2e3. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. checks.yml: the single rust job became parallel lint, build, three nextest test shards, four acceptance shards, an acceptance-gate and a result job (ADR-98), and every step the record relies on (fmt, clippy, deny, the records gate, the release guards, the rule table check, the gate action tests and harness status) is still run; Makefile: the rust-test target now runs the tests through .github/scripts/ci-tests.sh (nextest, doctests and the acceptance runner under the leak guard) and the Makefile gained the ci-gate-test and nextest-pin targets, and the targets the record relies on are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/checks.yml` 3bd0a6795184 -> a201f4e57f5f. Reason: checks.yml: the only change since the pinned version is the four tool-download curl lines (cargo-deny, cargo-machete), which gained --retry 5 --retry-all-errors --retry-delay 5 and changed -sSLfo to -sSLf with a separate -o (#441); no step, job or check changed, so the evidence still holds **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/header.rs` 2ee979c62600 -> cd81c077715c. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: header.rs: split header_required_fields into record_scoped_header_findings. Behaviour unchanged, outputs byte-identical (check, check --base and fix --rebaseline JSON compared before and after). **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/header.rs` cd81c077715c -> 8daab0cdc431, `rust/crates/urzua-core/src/rules/narrative.rs` 1b08f9fad547 -> b34355340066, `rust/crates/urzua-core/src/rules/pointer.rs` e4d8db46f2e3 -> 2d164d27a2e9, `rust/crates/urzua-core/src/rules/field.rs` c5fd645b5179 -> 9a5be442821a. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: header.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged; narrative.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged; pointer.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged; field.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-10 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/checks.yml` a201f4e57f5f -> ad6ab57882df. Reason: checks.yml gained one additive, report-only step in the build job (no stale evidence pins: runs fix --rebaseline without --apply and fails unless the report status is ok); no existing step changed, so the claim still holds **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/checks.yml` ad6ab57882df -> 0a3547c3f12a. Reason: Read the diff of checks.yml since the pin: the only change is one env entry on the test job, URZUA_BUG_LISTS_MAY_GROW, set from the bug-lists-grow pull request label for acceptance_bug_gate.rs; no step, job, trigger or ordering changed, so the claim about the workflow still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/checks.yml` 0a3547c3f12a -> e166badec9b1. Reason: checks.yml: the stale-pin step's suggested re-pin command now includes --actor (agent or human), required under a gh-verified identity; the rest of the workflow this record cites is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
