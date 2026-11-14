---
Stable-Id: 01M6ME4QBH5GKHM9RP6E3E12KT
Status: Fixed
Found-in: 'review of the declared transitions (MILE-144, ADR-72), 2026-11-04'
Realized-by: test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-j/expectations/a-renamed-record-keeps-its-committed-status.yaml@sha256:2f20a9f2875364ce5288e1628e1e663e303553869a2d2dba3baba9627a617fff, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-j/expectations/a-renamed-record-may-take-an-allowed-edge.yaml@sha256:c9d9ffe93c3c9b93a2f651f468cc1134c732fde802256d79fd61466b34a7ad1c, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-j/expectations/a-renamed-record-reset-to-an-initial-status-is-reported.yaml@sha256:13f701d0f00238154b90a93bf52f6c0247ac0fecfe16352b1f519244b54cd854
---
# 191 — A renamed record is judged as new by status.transition-allowed because the diff reports a rename as a deletion plus an addition

## What was wrong

`urzua check --base` reads a rename as a deletion plus an addition (`changed_paths_since` runs git with
`--no-renames`). `status.transition-allowed` looks the changed record up at its own path in the merge
base, finds nothing, and judges it new. A record renamed (a retitled slug, a moved directory) and moved
from `Accepted` to `Superseded` in the same change is then reported as a new record starting outside
`initial`, instead of being judged against the status it had before the rename. A rename that changes
nothing else is reported the same way when the record's status is not an initial one.

The record's `Stable-Id` is the identity that survives a rename, so the committed side should be
matched by `Stable-Id` across the deleted paths of the diff before the record is called new. This is a
Phase 0 defect: the rule is wrong for a normal editing action, not only an unusual one.

## Why nothing caught it

The acceptance cases write a record in place or add a new file; none renames a tracked record. The
rule's description names the limitation, which documents it without testing it.

## Fix

Fixed 2026-11-05. `committed_statuses` reads the merge base's copy at the record's own path; when there is none, it matches the record by `Stable-Id` against the records the diff deleted (same type) and takes that copy's status. No `Stable-Id`, or no match, still reads as new (from 2026-11-09 a match also needs the same record id, so a renumber reads as new); several matches count the record unreadable. The base text read added for `BUG-51` (`#279`) is what makes the lookup possible.

## References

- `MILE-144`, `ADR-72`; `status.transition-allowed` in `rust/crates/urzua-core/src/rules/status.rs`;
  `committed_statuses` in `rust/crates/urzua-cli/src/diff_scope.rs`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed, `Open`. **Why:** found while building the declared transitions; the fix (match a renamed record by `Stable-Id`) is deliberately not part of that change. | **substantive** |
> | 2026-11-05 | Fixed. **Why:** `committed_statuses` now matches a record the diff reports as added to a deleted record by `Stable-Id`; three acceptance cases in `corpus-j` (rename with the status unchanged, rename with an allowed edge, rename with a reset to an initial status) failed before and pass after. `SPEC-2` and `SPEC-3` document it. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 3 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/tests/acceptance/corpora/corpus-j/expectations/a-renamed-record-keeps-its-committed-status.yaml@sha256:026250566ae4, rust/crates/urzua-cli/tests/acceptance/corpora/corpus-j/expectations/a-renamed-record-may-take-an-allowed-edge.yaml@sha256:12de34e10183, rust/crates/urzua-cli/tests/acceptance/corpora/corpus-j/expectations/a-renamed-record-reset-to-an-initial-status-is-reported.yaml@sha256:61497dc3f3fc Reason: Pinned the files as they are now after checking each against the record it evidences: config.rs still reads the YAML config and init.rs still writes it (ADR-52), discovery.rs still carries no diagnostic for a format the tool no longer reads (ADR-54), header.rs still parses the header as one closed structure and reports duplicate keys (ADR-8), main.rs still hands the paths argument to check (BUG-1), report.rs and main.rs still hold the shared Report and emit contract (BUG-19), the corpus-j renamed-record expectation files still exist (BUG-191), tests/common/mod.rs still owns the TempDir that removes fixture directories and the test-no-leak scripts still fail on a leftover (BUG-193), init.rs and migrate.rs still emit JSON reports (BUG-20), report.rs still declares ScopeSource with the SPEC-2 values and discovery.rs maps to it (BUG-25), and prepare-release.yml still fails loudly with the knope.toml release steps behind it (BUG-27). BUG-2 and BUG-23 are not in this batch because the code they cite moved out of rules.rs. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-09 | Rename behaviour narrowed (`ADR-89` amendment, rename pairing, accepted by the maintainer on 2026-11-10). **Why:** matching by `Stable-Id` alone let a new record that copied a deleted record's `Stable-Id` be judged against that record's status, so a non-decider could add an Accepted record unseen by `diff.decider-required`. A record added where the base has none now pairs with a deleted record only when the record id (type and number) is also the same: a slug rename still pairs, a renumber reads as new. Tests: `a_slug_rename_with_the_same_number_and_stable_id_is_still_paired_with_its_base_copy` and `a_renumber_that_keeps_the_stable_id_is_a_new_record_not_a_move` in `diff_scope_integration.rs`. | **substantive** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-cli/tests/acceptance/corpora/corpus-j/expectations/a-renamed-record-keeps-its-committed-status.yaml` 026250566ae4 -> 2f20a9f28753, `rust/crates/urzua-cli/tests/acceptance/corpora/corpus-j/expectations/a-renamed-record-may-take-an-allowed-edge.yaml` 12de34e10183 -> c9d9ffe93c3c, `rust/crates/urzua-cli/tests/acceptance/corpora/corpus-j/expectations/a-renamed-record-reset-to-an-initial-status-is-reported.yaml` 61497dc3f3fc -> 13f701d0f002. Reason: Read the diff of each pinned case file since the pin: each gained one line, bugs: [BUG-191], the tag the bug gate reads; the planted rename, the commits, the invocation and every expectation are unchanged, so each case still reproduces the rename fix. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
