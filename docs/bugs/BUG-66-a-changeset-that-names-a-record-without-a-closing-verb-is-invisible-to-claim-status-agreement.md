---
Stable-Id: 01M2WDKB3Y7SCHKRWPN87BXVZ6
Status: Fixed
Found-in: "Five bug records shipped as Status: Open in the same branch that fixed them, with a changeset naming all five; claim.status-agreement ran over the changeset and reported nothing"
Realized-by: code:rust/crates/urzua-core/src/rules/claim.rs@sha256:0b8792841cf03432266520fcf7605956c5ac0160a2855a8b4d5ea2b6f50cf750, code:rust/crates/urzua-core/src/rules/claim_field.rs@sha256:59b53f0404d700a2579c633fc73aa0a5b0c2ec72cdb43d5d1fab79c74f92028b, test:rust/crates/urzua-core/src/rules/claim.rs#a_fragment_listing_a_still_open_record_is_an_error_observed_failing@sha256:866f9c61e1ea9fce289c5e1fc37000a79f54c5cd3460ebea61cf7b9f413b8753, test:rust/crates/urzua-core/src/rules/claim.rs#strict_asks_every_fragment_for_the_field_observed_failing@sha256:551e874f2721cb80aa3a0122094a9434b87c11b9e7ca27b9bf9563d883498833, test:rust/crates/urzua-cli/tests/claim_field_integration.rs#a_fragment_listing_a_still_open_record_fails_the_check_observed_failing@sha256:adc89f4b33d11468890e70de08331c4c93331401a500266bf9d09c9abc314b73, test:rust/crates/urzua-cli/tests/claim_field_integration.rs#strict_catches_the_incident_shape_a_fragment_naming_a_record_and_declaring_nothing_observed_failing@sha256:acea6bca303661501361f0b49276f6400eaa60dff2968878d5a4e33012193f48
---
# 66 — A changeset that names a record without a closing verb is invisible to claim.status-agreement

## What was wrong

`claim.status-agreement` exists to stop a changeset announcing a record closed while the record says
otherwise -- the defect `BUG-36` shipped. It recognizes a claim only when a reference follows a
closing verb.

A changeset that names the record without one is invisible to it. The round-6 changeset reads

```text
`check`'s path argument narrows what is reported on, not what a pointer resolves against (BUG-60).
```

for each of five bugs, all of which were still `Status: Open`. The rule read the changeset, examined
2 claim inputs -- `records_examined` counts the claim files it was handed, not corpus records -- and
reported no findings.

The inverse of `BUG-36`: that was a claim to close an open record, caught. This is the same
disagreement, stated in a form the rule does not parse, and reported as agreement.

## Why nothing caught it

The rule's tests supply changesets written with a closing verb, because that is the shape the rule
recognizes -- so they exercise the parser's successes and never its silence. `records_examined: 2` was
visible in every run on this corpus. It counts claim files read, so it was two whether or not a claim
was found in either, and it read as confirmation the rule had done its job.

Deciding what counts as a claim is the open question, not an oversight to patch: a changeset may
legitimately name a record it does not close, so treating a bare mention as a claim trades this
silence for false positives. What should not survive is the rule reporting agreement when it found no
claims at all.

## References

- `BUG-36`, the defect this rule was built for, and the shape it does catch.
- `BUG-56`, which stopped this same rule going inert when its input was unreachable. This is the same
  rule going inert with its input in hand.
- `RFC-36`, which takes the open question here as its subject and measures it: 44 of 49 references in
  this repository's changeset history are bare, and most are citations rather than unstated claims.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Triage: still reproduces on main; `Status` stays `Open`. A changeset reading `Names ADR-1 without closing it.` gives `claim.status-agreement` `eligible: 0, examined: 0` and no finding. **Why:** what counts as a claim is `RFC-36`, still Draft. | **substantive** |
> | 2026-11-05 | Waits on `RFC-36`; `Status` stays `Open`. **Why:** Maintainer decision, 2026-11-05: what counts as a claim is `RFC-36`'s to decide, so this bug is not worked until it is decided. | **substantive** |
> | 2026-11-05 | Unblocked by `ADR-81` (Proposed), which records the decision to accept `RFC-36`: keep the closing verb and add an opt-in rule for a bare reference to an open record in declared text. `Status` stays `Open`. **Why:** the block was on `RFC-36`'s decision; the fix waits only for the rule to be built. | **substantive** |
> | 2026-11-05 | `ADR-81` is Rejected, so the 2026-11-05 unblock above no longer stands. This bug closes once `ADR-85` is built (a changeset declares the records it closes in a structured field, and a listed record that is still open is an error). `Status` stays `Open`. **Why:** maintainer decision of 2026-11-05; the fix is the structured field, not a rule over bare references. | **substantive** |
> | 2026-11-05 | Fixed. `claim.status-agreement` takes the `closes` and `strict` options: a fragment lists the records it closes in a one-line frontmatter key, and a listed record that is still open is an error, so the incident's own shape (five bugs named `(BUG-60)` while `Open`) is caught under `strict` because the fragment lacks the key. This repository enables both and every fragment carries `closes:`. `Blocked-on` is removed: `RFC-36` is not what decided this. **Why:** `ADR-85`, the structured field decided on 2026-11-05; a declared claim is exact and a citation is never mistaken for one. | **substantive** |
> | 2026-11-05 | `Regression-test` retired (ADR-84); the 4 tests that cover the change are now cited as `test:` locators in `Realized-by`, and the field is removed. **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/rules/claim.rs@sha256:37a3c2d45909, rust/crates/urzua-core/src/rules/claim_field.rs@sha256:59b53f0404d7 Reason: Pinned the files as they are now after checking each against the record it evidences: claim.rs and claim_field.rs still read a changeset's declared closes field as a claim (BUG-66), release-invariants.test.sh still plants a release-prep version bump and an unmatched level and asserts the guards fail, and checks.yml now runs it on the unattended path (BUG-92, BUG-93, BUG-96), new.rs and new_record.rs still take the configured header shape over a template (MILE-1), filename.rs still compares the filename's type prefix and padded number with the H1 (MILE-10), config.yaml still declares config.scope-matches-nothing and checks.yml runs the gate (MILE-106, MILE-110), and header.rs, narrative.rs, pointer.rs, field.rs, fix.rs, the Makefile and checks.yml still read the declared relation field names with relation_field_literals.rs failing on a restated literal (MILE-107, MILE-109). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/claim_field_integration.rs#a_fragment_listing_a_still_open_record_fails_the_check_observed_failing` (lines 84-104) fe21e3074531 -> adc89f4b33d1, `rust/crates/urzua-cli/tests/claim_field_integration.rs#strict_catches_the_incident_shape_a_fragment_naming_a_record_and_declaring_nothing_observed_failing` (lines 134-153) 02a6306774dd -> acea6bca3036. Reason: claim_field_integration.rs: the only change since the pinned version is that fixtures now clean up through common::TempDir (the repo helper returns it instead of a PathBuf) and the explicit remove_dir_all calls were dropped (#439); no assertion or tested behaviour changed **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/claim.rs` 37a3c2d45909 -> 70f4024a7f40. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: claim.rs: split claim_status_agreement into verb_occurrences, listed_claims, claim_candidates and the ClaimJudge methods verb_findings and listed_findings. Behaviour unchanged, outputs byte-identical (check, check --base and fix --rebaseline JSON compared before and after). **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/claim.rs` 70f4024a7f40 -> 0b8792841cf0. Reason: Read the diff of each re-pinned file against main. core/rules/claim.rs: the Notice literals gain detail: None; no behaviour change. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
