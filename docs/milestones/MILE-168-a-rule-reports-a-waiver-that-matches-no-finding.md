---
Stable-Id: 01M6JW47RYART844WMEJMRVWTD
Status: Done
Phase: '0'
Track: governance-process
Implements: SPEC-1
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/waiver_stale_integration.rs#a_whole_run_reports_the_stale_and_the_expired_waiver_but_not_the_live_one@sha256:57d141fb74f1a71d166722df1ebaa3183a462af600b6d3c970df438eca635d3b, code:.urzua/config.yaml@sha256:594d115541e37b3e3c0e602a4ef18d82eaf99100174a048133003a2bd80ee78b, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-waiver-rules-run.yaml@sha256:840cd42fc61580cdbdb5c482b066b4ef4c3501b8641496aac46f04730cd6d531
---
# 168 — A rule reports a waiver that matches no finding

## What

Add a rule that reports a waiver which matches no current finding.

## Why

A stale waiver is a silent no-op today: the finding it covered is gone but the exception stays on file, hiding that the exception is no longer needed and leaving room to cover a future finding by accident. The no-silent-no-op principle applies to the waiver record as much as to a rule.

## Done means

- The rule names each waiver that matches nothing and the rule and target it claimed. — evidence: test:rust/crates/urzua-cli/tests/waiver_stale_integration.rs#a_whole_run_reports_the_stale_and_the_expired_waiver_but_not_the_live_one@sha256:57d141fb74f1a71d166722df1ebaa3183a462af600b6d3c970df438eca635d3b
- A waiver that matches a finding is not reported. — evidence: test:rust/crates/urzua-cli/tests/waiver_stale_integration.rs#a_whole_run_reports_the_stale_and_the_expired_waiver_but_not_the_live_one@sha256:57d141fb74f1a71d166722df1ebaa3183a462af600b6d3c970df438eca635d3b
- The rule is enabled in urzua's own `.urzua/config.yaml`. — evidence: code:.urzua/config.yaml@sha256:594d115541e37b3e3c0e602a4ef18d82eaf99100174a048133003a2bd80ee78b
- `urzua check` is clean over this repository's waivers, not only in unit tests. — evidence: test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-waiver-rules-run.yaml@sha256:840cd42fc61580cdbdb5c482b066b4ef4c3501b8641496aac46f04730cd6d531

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Built as `waiver.matches-nothing` (opt-in, enabled in this repository's config). Reports an expired waiver, a waiver naming an unknown rule, and an active waiver whose rule ran over the whole corpus and found nothing in its scope; a partial run does not judge a live waiver. **Why:** the Done-when contract; scope semantics chosen so `--base` and path-scoped runs produce no false stale report. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged on `main`: opt-in rule `waiver.matches-nothing`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: .urzua/config.yaml@sha256:151830e5926d, rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-waiver-rules-run.yaml@sha256:840cd42fc615 Reason: Pinned the files as they are now after checking each against the record it evidences: config.yaml still declares test_paths, the statuses.pending pattern entries, the pending acknowledgement field behind provisional claims, the index format placeholders, drift_audit_field, the revision-log marker, waiver.matches-nothing and pending_until_required that these milestones dogfood (MILE-159 to MILE-169), commands/fix.rs still takes paths, proposed content and apply without a log (MILE-159), back_pointer.rs still matches test_paths as globs (MILE-160), config.rs still reads the declared pending-acknowledgement field (MILE-163), and the corpus-zero embodiment, relation, index, revision-log, waiver and field expectation files still exist and run this repository's own config. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 151830e5926d -> 0b59c6dfc8b9. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. config.yaml: config.yaml gained Kind among the milestone type's known fields and a kind declaration (build and decision) under statuses.delivery.items (ADR-96 amendment of 2026-11-07), and every other declaration the record relies on is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0b59c6dfc8b9 -> 0a00038a6388. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: this change declares status.done-items-shape (error), status.waived-item-stale (warn) and embodiment.claim-renewal-unattested (warn) and declares relation.cross-type-disallowed, section.shape and diff.source-and-records-mixed as off; every other declaration is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0a00038a6388 -> 3d51c2fac8b7. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: only the comment above the three rules declared off changed, to say they report `not-enabled` exactly as an undeclared rule does; no declaration changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 3d51c2fac8b7 -> 653bb0f54ca6. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: the only change is a new optional pending list (two placeholder patterns) under record_types::milestone statuses.delivery.items; nothing else in the file changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 653bb0f54ca6 -> 594d115541e3. Reason: config.yaml: read the diff since the previous pin; the only change is one added line, outside_writes: shared, under the harness key (this repository declares the new harness.outside_writes option, ADR-89 amendment of 2026-11-09). No record type, rule, status, relation or other declaration this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
