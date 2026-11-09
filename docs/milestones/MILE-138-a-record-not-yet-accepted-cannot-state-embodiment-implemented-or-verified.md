---
Stable-Id: 01M6HZ0MQVAKV54TM17N8MY1HN
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Realized-by: code:rust/crates/urzua-core/src/rules/embodiment.rs#embodiment_requires_accepted_status@sha256:fde1673126b69fbcf7e17f8ed33bc8578b815fe3be3ef16ed1fd7c6783cb8588, test:rust/crates/urzua-core/src/rules/embodiment.rs#a_proposed_record_stating_implemented_or_verified_is_a_finding_observed_failing@sha256:4c3995bad5767984a1046fae0717d51c3ba39ba138caf91fb938cbbc8aba4811, test:rust/crates/urzua-core/src/rules/embodiment.rs#the_same_record_accepted_is_not_a_finding@sha256:54b48cca62c5925d8a62aaa4b5a81606e88433a5742ef6a279809158b31c4a67, test:rust/crates/urzua-core/src/rules/embodiment.rs#specified_and_inactive_on_a_proposed_record_are_not_flagged@sha256:d9e0546b27f802af21ec12627e370142d0887f8b92c9de37820157383c0cec88
---
# 138 — A record not yet accepted cannot state Embodiment Implemented or Verified

## What

A cross-field rule: a record whose Status is not an accepted-type status cannot state an Embodiment of `Implemented` or `Verified`. The rule reads the declared status vocabulary from `MILE-132` and fails with an error naming both values.

## Why

`embodiment.consistency` compares the stated Embodiment with the tier computed from evidence and never reads Status, so a Proposed record can claim to be Verified and pass. The adopter's linter blocks that combination, since a decision not yet made cannot have been built. Without the check the claim goes unnoticed until the record is accepted.

## Done means

- A planted record with a not-yet-accepted status and `Embodiment: Implemented`, and again with `Verified`, is an error. — evidence: code:rust/crates/urzua-core/src/rules/embodiment.rs#embodiment_requires_accepted_status@sha256:fde1673126b69fbcf7e17f8ed33bc8578b815fe3be3ef16ed1fd7c6783cb8588, test:rust/crates/urzua-core/src/rules/embodiment.rs#a_proposed_record_stating_implemented_or_verified_is_a_finding_observed_failing@sha256:4c3995bad5767984a1046fae0717d51c3ba39ba138caf91fb938cbbc8aba4811
- The same record accepted is not an error. — evidence: test:rust/crates/urzua-core/src/rules/embodiment.rs#the_same_record_accepted_is_not_a_finding@sha256:54b48cca62c5925d8a62aaa4b5a81606e88433a5742ef6a279809158b31c4a67
- `Specified` and `Inactive` on a not-yet-accepted record are not flagged. — evidence: test:rust/crates/urzua-core/src/rules/embodiment.rs#specified_and_inactive_on_a_proposed_record_are_not_flagged@sha256:d9e0546b27f802af21ec12627e370142d0887f8b92c9de37820157383c0cec88
- The rule reports how many records it examined. — evidence: test:rust/crates/urzua-core/src/rules/embodiment.rs#a_proposed_record_stating_implemented_or_verified_is_a_finding_observed_failing@sha256:4c3995bad5767984a1046fae0717d51c3ba39ba138caf91fb938cbbc8aba4811

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter blocks Implemented or Verified on a record that is not accepted, and urzua's embodiment rule never reads Status. | **substantive** |
> | 2026-11-04 | Built as the opt-in rule `embodiment.requires-accepted-status`. Accepted means the type's declared `statuses.success` (`MILE-132`); a type declaring none uses the rule's `accepted_statuses` option, and with neither it is not examined, the narrowest reading. **Why:** the rule needs a declared meaning of accepted rather than a guess from status names. | **substantive** |
> | 2026-11-04 | The rule also allows the claim at the type's declared `statuses.claim_retained` (`SPEC-3`); this repository's ADR type declares `[Superseded]`. **Why:** maintainer decision of 2026-11-04: a record may keep an Implemented or Verified claim in a status reached only after acceptance. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #178: opt-in rule `embodiment.requires-accepted-status`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/embodiment.rs#embodiment_requires_accepted_status` (lines 941-1018) c914dfe7f284 -> fde1673126b6. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: embodiment.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
