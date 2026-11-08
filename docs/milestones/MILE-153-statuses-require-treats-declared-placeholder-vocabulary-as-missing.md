---
Stable-Id: 01M6JKNS1NA8ECXQDEF25RA50M
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/placeholder_vocabulary_integration.rs#an_accepted_adr_whose_deciders_is_a_declared_placeholder_is_reported@sha256:d9525ed1a224569a925e8cf8bb4e889a07a334a1de25a658018bf39c901ce26f, test:rust/crates/urzua-cli/tests/placeholder_vocabulary_integration.rs#a_real_value_that_merely_contains_the_word_is_not_reported@sha256:b8701780b9e6be9918ba1b6c1b91fc4c5687b4decf0a2c123f24777f3e003ec7, code:rust/crates/urzua-core/src/field_state.rs@sha256:d46c56def01e7dbcfee70b6c994f022fb02a22b138bea50167028e46c232ceed, code:rust/crates/urzua-core/src/status_model.rs@sha256:744b341a578a23b1682b8d176147a340bdab44f3b0736d282295f3c039758094, test:rust/crates/urzua-core/src/rules/status.rs#a_declared_pending_value_does_not_satisfy_a_requirement_observed_failing@sha256:49ca989ed24942d6d1fdf94f3b5d539199273c15c7d875fae6d6f0437ec0f910, code:.urzua/config.yaml@sha256:151830e5926d1887be39a723d110744ff5e2ae8c408bb114a90ba3e2b8316a2e, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-status-rules-run.yaml@sha256:36481ec0e48ca6e0e543452b15701f75b1103a072591f0add6f8923f81f66fd3
---
# 153 — Statuses.require treats declared placeholder vocabulary as missing

## What

Make `statuses.require` count a declared placeholder vocabulary (`pending ...`, `none recorded`, a dash, `Unknown`) as a missing value, using the type's declared `pending` vocabulary.

## Why

Today only blank or unedited template text counts as missing, so an Accepted record whose Deciders or Reviewers read `(pending ...)` or `Unknown` passes. The type already declares a `pending` vocabulary, but it does not feed this rule: a record can claim a state its own fields contradict. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- A value matching the declared placeholder vocabulary in a required field of a required status is reported. — evidence: test:rust/crates/urzua-cli/tests/placeholder_vocabulary_integration.rs#an_accepted_adr_whose_deciders_is_a_declared_placeholder_is_reported@sha256:d9525ed1a224569a925e8cf8bb4e889a07a334a1de25a658018bf39c901ce26f
- A real value that merely contains the word is not reported. — evidence: test:rust/crates/urzua-cli/tests/placeholder_vocabulary_integration.rs#a_real_value_that_merely_contains_the_word_is_not_reported@sha256:b8701780b9e6be9918ba1b6c1b91fc4c5687b4decf0a2c123f24777f3e003ec7
- The vocabulary is declared once and shared with the existing pending check. — evidence: code:rust/crates/urzua-core/src/field_state.rs@sha256:d46c56def01e7dbcfee70b6c994f022fb02a22b138bea50167028e46c232ceed, code:rust/crates/urzua-core/src/status_model.rs@sha256:744b341a578a23b1682b8d176147a340bdab44f3b0736d282295f3c039758094, test:rust/crates/urzua-core/src/rules/status.rs#a_declared_pending_value_does_not_satisfy_a_requirement_observed_failing@sha256:49ca989ed24942d6d1fdf94f3b5d539199273c15c7d875fae6d6f0437ec0f910
- The capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml@sha256:151830e5926d1887be39a723d110744ff5e2ae8c408bb114a90ba3e2b8316a2e, test:rust/crates/urzua-cli/tests/placeholder_vocabulary_integration.rs#an_accepted_adr_whose_deciders_is_a_declared_placeholder_is_reported@sha256:d9525ed1a224569a925e8cf8bb4e889a07a334a1de25a658018bf39c901ce26f, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-status-rules-run.yaml@sha256:36481ec0e48ca6e0e543452b15701f75b1103a072591f0add6f8923f81f66fd3

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Implemented: `status.required-field-missing` counts a whole value of the type's declared `statuses.pending` as missing; this repository's `adr` type declares `pending: [pending, unknown, none recorded]` and `placeholder_vocabulary_integration.rs` runs the shipped config. `Status` stays `Planned`. **Why:** the Done-when contract; status is a human decision. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #191: `status.required-field-missing` counts the declared `statuses.pending` vocabulary as missing. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 4 locators pinned, 4 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/field_state.rs@sha256:d46c56def01e, rust/crates/urzua-core/src/status_model.rs@sha256:744b341a578a, .urzua/config.yaml@sha256:151830e5926d, rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-status-rules-run.yaml@sha256:36481ec0e48c Reason: Pinned the files as they are now after checking each against the record it evidences: fix.rs and fix_drift_integration.rs still detect, apply and keep idempotent the Drift detected write (MILE-15), discovery.rs still honours exclude_files (MILE-150), config.yaml still declares the identity pattern and field set that urzua new follows, the indexes and index regeneration, the declared placeholder vocabulary, provisional relation claims, per-type relation pairs, external_ids, date_order and the exclude_paths globs (MILE-151, MILE-152, MILE-153, MILE-154, MILE-155, MILE-156, MILE-157, MILE-158), commands/index.rs still regenerates the declared indexes (MILE-152), field_state.rs and status_model.rs still read the declared pending vocabulary (MILE-153), config.rs still reads the relation-pair declaration and the corpus-zero expectation files still exist for the index, status, relation, citation and date rule families (MILE-152 to MILE-157). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
