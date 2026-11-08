---
Stable-Id: 01M6CXZT9ANCTR6QP7ZQGAYKQJ
Status: Done
Phase: '0'
Track: governance-process
Realized-by: code:Makefile@sha256:fb9fda4c85083447873cd372df238eae66f507575c2257c8f275f2253e4ae436, test:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d, code:.urzua/config.yaml#record_types::waiver@sha256:69b0fd1326ab53372115db3407353c470622aff43f41c010de7586382d1ca4ba, test:rust/crates/urzua-cli/tests/suppression_type_integration.rs#a_suppression_missing_a_required_field_does_not_suppress_and_is_reported_observed_failing@sha256:a5809f8dda440e8d090bc779577c05b426fc498e321347a8eef6156bb1e70a7c, test:rust/crates/urzua-cli/tests/waiver_stale_integration.rs#a_whole_run_reports_the_stale_and_the_expired_waiver_but_not_the_live_one@sha256:57d141fb74f1a71d166722df1ebaa3183a462af600b6d3c970df438eca635d3b, code:.urzua/config.yaml#rules::embodiment.locator-promotion-candidate@sha256:f03e6caf54e4c133f4c70a42e5486808294a3abf805c3f96f44ad90a8bf8c2c1, code:.urzua/config.yaml#fail_on@sha256:0e6d72b8b4e2400601c97e1f94ef7d69ee00a254f628dbf97ab894e85252147f, test:rust/crates/urzua-cli/tests/fail_on_integration.rs#config_fail_on_warning_makes_a_warning_exit_1_with_unchanged_json_observed_failing@sha256:d1e8e19c3842276d697a34cbf386f69ca04921417026f4d0cc1a5f9a86219b04
---
# 116 — Bring this repository's own check output to zero unexplained warnings

## What

Bring `urzua check` (the repository gate `make records` runs) on this repository to zero unexplained warnings: each current warning is
fixed at its source, or waived through a `waiver` record with a reason and an expiry, so `check`
itself annotates it (`SPEC-10`). A warning merely mentioned by another record does not count: it stays
indistinguishable from a new finding in `check`'s output. The count today is 69, all `warn`-level, by rule: `embodiment.consistency` 34,
`embodiment.locator-promotion-candidate` 18, `pointer.target-status` 6, `field.pending` 5,
`narrative-field.stale` 5, `header.field-set-consistency` 1.

Also in scope: inconsistencies between records and the specs that define their type that no rule
reports yet. Known today: `SPEC-6` defines a milestone's `Implements` as RFC/ADR lineage, but
`MILE-100` to `MILE-104` point it at specs; either the spec or those records changes.

## Why

A bootstrap corpus that carries standing warnings teaches everyone, agents included, to ignore the
output of the tool whose whole job is to be believed. The warnings exit 0, so nothing forces a
decision about them, and a new finding on a new record is indistinguishable from the 69 already there.
`AGENTS.md` already says a live finding is signal ("Don't patch around a finding on your own new
work"); this milestone is the work of making that rule cheap to follow by clearing the backlog it
would otherwise be buried in. Counts were taken by running `urzua check` on this branch (`urzua check docs/` gave the same result).

## Done means

- `urzua check` over this repository, which `make records` and `checks.yml` run, reports zero unwaived findings. — evidence: code:Makefile@sha256:fb9fda4c85083447873cd372df238eae66f507575c2257c8f275f2253e4ae436, test:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d
- Each warning is fixed at its source or waived by a `waiver` record whose reason is required and whose expiry `check` itself annotates. — evidence: code:.urzua/config.yaml#record_types::waiver@sha256:69b0fd1326ab53372115db3407353c470622aff43f41c010de7586382d1ca4ba, test:rust/crates/urzua-cli/tests/suppression_type_integration.rs#a_suppression_missing_a_required_field_does_not_suppress_and_is_reported_observed_failing@sha256:a5809f8dda440e8d090bc779577c05b426fc498e321347a8eef6156bb1e70a7c, test:rust/crates/urzua-cli/tests/waiver_stale_integration.rs#a_whole_run_reports_the_stale_and_the_expired_waiver_but_not_the_live_one@sha256:57d141fb74f1a71d166722df1ebaa3183a462af600b6d3c970df438eca635d3b
- `embodiment.locator-promotion-candidate` is advisory-only in `.urzua/config.yaml` by maintainer decision (turned off on 2026-11-04, since an `info`-level rule that never fails `fail_on: warning`). — evidence: code:.urzua/config.yaml#rules::embodiment.locator-promotion-candidate@sha256:f03e6caf54e4c133f4c70a42e5486808294a3abf805c3f96f44ad90a8bf8c2c1
- CI fails when a new finding appears. — evidence: code:.urzua/config.yaml#fail_on@sha256:0e6d72b8b4e2400601c97e1f94ef7d69ee00a254f628dbf97ab894e85252147f, test:rust/crates/urzua-cli/tests/fail_on_integration.rs#config_fail_on_warning_makes_a_warning_exit_1_with_unchanged_json_observed_failing@sha256:d1e8e19c3842276d697a34cbf386f69ca04921417026f4d0cc1a5f9a86219b04, test:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d

## References

- `SPEC-10` -- the `waiver` record type, the sanctioned way to keep a finding visible but non-blocking.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `check docs/` exits 0 with 69 standing warnings across six rules; filed so a new finding on a new record is visible instead of lost among them. | **substantive** |
> | 2026-11-04 | Turned `embodiment.locator-promotion-candidate` off in `.urzua/config.yaml` (advisory only; several ADRs citing one rule module is expected after the MILE-131 split), and applied three record fixes (`MILE-56`, `RFC-39`/`ADR-59`, `BUG-38`). **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #161, #159, #139: `urzua check` over this repository reports zero unwaived findings: each warning is fixed or waived by a waiver record with a reason and an expiry. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml#record_types::waiver` (lines 436-484) 40c9440519e0 -> 69b0fd1326ab. Reason: Read the diff of each pinned span since the old pin and the claim still holds. config.yaml#record_types::waiver: the type's revision_log was expanded from the inline marker and required pair to also declare classes, class_column, date_column and tool_class (the class vocabulary of ADR-94). **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: Makefile@sha256:fb9fda4c8508, .github/workflows/checks.yml@sha256:4afa5fe0db59 Reason: Pinned the files as they are now after checking each against the record it evidences: report.rs still discloses every non-absent exclusion in a rule's population (MILE-111), catalog.rs still registers header.case-variant-key (MILE-113), export.rs still emits each record as plain JSON and generate-dashboard.py still reads it (MILE-115), the Makefile and checks.yml still run the zero-warning gate (MILE-116), AGENTS.md and config.rs still state and read the declared-adopter premise (MILE-118), diff_scope.rs still selects the records a --base run examines (MILE-123), boundary.rs, back_pointer.rs and the corpus-h expectation still implement and prove the declared published-path boundary (MILE-125), the claim_expiry.rs, reverify.rs and commands/reverify.rs files and the changeset still implement ADR-97 beside embodiment.rs, discovery.rs and fix.rs (MILE-129), discovery.rs, new.rs and the corpus-i files still give one identity namespace across directories (MILE-130), and the rules.rs module index and the per-family rule modules still exist as the split MILE-131 describes. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
