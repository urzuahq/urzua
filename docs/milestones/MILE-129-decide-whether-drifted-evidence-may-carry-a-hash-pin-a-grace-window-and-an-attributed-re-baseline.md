---
Stable-Id: 01M6CXZYQQ4ZT1R16B9JVYZ90W
Status: InProgress
Phase: '0'
Track: embodiment-model
Realized-by: spec:docs/adr/ADR-67-evidence-pins-a-drift-grace-window-and-a-claim-age-are-opt-in-refinements-of-git-history-drift.md, code:rust/crates/urzua-core/src/rules/embodiment.rs, code:rust/crates/urzua-cli/src/discovery.rs, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_pin_that_still_matches_overrides_the_git_history_heuristic_observed_failing@sha256:588b76ea5dc8e4bf74dd71eac977b87691f3a0c5bb955b44f5881167cd54c19f, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_pin_that_no_longer_matches_is_drift_even_when_the_record_is_newer_observed_failing@sha256:86c2466cfc75ecc8e0496fed7ec585658604fc34c452e4aaffc36b65fea31773, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_fresh_locator_change_inside_the_grace_window_does_not_report_observed_failing@sha256:4059213a5f587afed56cd674f702611c8fe85554b022352e17c1db5e6341f535, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_locator_change_older_than_the_grace_window_reports@sha256:f0faff54e7c2113f7dab7f2fd78352e329a51807d1a366581e1b571e7167228a, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_claim_older_than_the_configured_age_warns_observed_failing@sha256:9d5451b42517e62e86cb9502b5335fcb629af38de9446ce55ee3171c72ebbc22, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_claim_younger_than_the_configured_age_is_silent@sha256:507e887241b5c986cfec623447c9690ef9155620d07fd0bb33fbfc0f4cc73624, code:rust/crates/urzua-core/src/fix.rs#apply_rebaseline@sha256:88aa628c1aa4182144a8a5ec1c67e21fc0890d54575226b70274a362d5ca65b4, code:rust/crates/urzua-cli/src/commands/fix.rs, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#apply_re_pins_only_the_chosen_record_with_who_and_why_then_is_a_no_op_observed_failing@sha256:5c4d251e51ffe05cb113f79f727e2624dcf302077bb319f879c60de4f3fec007, code:.urzua/config.yaml#rules::embodiment.consistency@sha256:2c6819d6375e3b4f59b879cd38d1a3b458329447e047bbb64ab06de0a1240886, spec:docs/bugs/BUG-139-record-id-and-filename-number-independently-re-implement-the-same-filename-id-extraction.md
---
# 129 — Decide whether drifted evidence may carry a hash pin, a grace window, and an attributed re-baseline

## What

Decide whether drifted evidence may carry a hash pin, a grace window before drift blocks, an attributed
re-baseline (logged with who and why), and a claim age (how long a stated `Embodiment` has gone
unchanged). Each is independently decidable; this milestone is where the combination is settled or
split.

## Why

`ADR-32` decided drift detection reads git history on the `Realized-by` line and stores no hash, so
drift is unconditional and there is nowhere to record "re-verified, still holds". `RFC-2` (Accepted)
proposed the opposite on every point: recorded and current hashes, a `graceExpiresAt`, and an
attributed refresh via `--by`; its own text says content-hash drift detection "is not built". The two
records pull in opposite directions and neither has been reconciled. A corpus of several hundred records
is large enough that unconditional drift would block constantly with no sanctioned way to re-baseline, so the
tension has to be resolved before `embodiment.consistency` can be promoted past `warn` on it.

## Decision (2026-11-04)

All three, each opt-in and off by default: content-hash pins on evidence, a configurable grace window in days, and a warning when a claim exceeds a configured age.

## Done means

- `ADR-67`, with its amendment of 2026-11-04, is Accepted by the maintainer, and `Status` and `Realized-by` moved only after it. — evidence: spec:docs/adr/ADR-67-evidence-pins-a-drift-grace-window-and-a-claim-age-are-opt-in-refinements-of-git-history-drift.md
- The opt-in `@sha256:` evidence pins are built. — evidence: code:rust/crates/urzua-core/src/rules/embodiment.rs, code:rust/crates/urzua-cli/src/discovery.rs, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_pin_that_still_matches_overrides_the_git_history_heuristic_observed_failing@sha256:588b76ea5dc8e4bf74dd71eac977b87691f3a0c5bb955b44f5881167cd54c19f, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_pin_that_no_longer_matches_is_drift_even_when_the_record_is_newer_observed_failing@sha256:86c2466cfc75ecc8e0496fed7ec585658604fc34c452e4aaffc36b65fea31773
- `drift_grace_days` is built. — evidence: test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_fresh_locator_change_inside_the_grace_window_does_not_report_observed_failing@sha256:4059213a5f587afed56cd674f702611c8fe85554b022352e17c1db5e6341f535, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_locator_change_older_than_the_grace_window_reports@sha256:f0faff54e7c2113f7dab7f2fd78352e329a51807d1a366581e1b571e7167228a
- `claim_max_age_days` is built. — evidence: test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_claim_older_than_the_configured_age_warns_observed_failing@sha256:9d5451b42517e62e86cb9502b5335fcb629af38de9446ce55ee3171c72ebbc22, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_claim_younger_than_the_configured_age_is_silent@sha256:507e887241b5c986cfec623447c9690ef9155620d07fd0bb33fbfc0f4cc73624
- The attributed re-baseline, re-baselining from `MILE-181`, is built. — evidence: code:rust/crates/urzua-core/src/fix.rs#apply_rebaseline@sha256:88aa628c1aa4182144a8a5ec1c67e21fc0890d54575226b70274a362d5ca65b4, code:rust/crates/urzua-cli/src/commands/fix.rs, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#apply_re_pins_only_the_chosen_record_with_who_and_why_then_is_a_no_op_observed_failing@sha256:5c4d251e51ffe05cb113f79f727e2624dcf302077bb319f879c60de4f3fec007
- `drift_grace_days` is exercised in urzua's own `.urzua/config.yaml` and the pins on its own records. — evidence: code:.urzua/config.yaml#rules::embodiment.consistency@sha256:2c6819d6375e3b4f59b879cd38d1a3b458329447e047bbb64ab06de0a1240886, spec:docs/bugs/BUG-139-record-id-and-filename-number-independently-re-implement-the-same-filename-id-extraction.md
- `ADR-97`, which replaces the rule-wide claim age as this repository's use of claim expiry, is Accepted by the maintainer.
- `embodiment.claim-expired` and `embodiment.claim-renewal-unattested` are built as `ADR-97` sections 1 to 3, 6 and 7 decide, off until declared.
- `urzua reverify` is built as `ADR-97` section 4 decides: a review report, the refusals, the attributed `Re-verified` row and the anchor date.
- `SPEC-2`, `SPEC-3`, `SPEC-8` (or a new spec) and `SPEC-18` document the rules, options, command and field with revision rows, and a minor changeset names them.
- urzua's own `.urzua/config.yaml` declares `embodiment.claim-expired` with the periods of `ADR-97` section 10, and the `adr`, `milestone` and `bug` types declare `Last-audited`; a first run reports the 0 expired records `ADR-97` measured.

## Decision (2026-11-07)

Reopened by the maintainer: the claim age must be usable here, with an expiry configured per record
type, where re-verifying a claim and refreshing its hashes is a recorded act. `ADR-97` (Proposed) drafts
it; Done now means implementing it.

## References

- `ADR-32` -- drift via git history, not a stored hash.
- `RFC-2` -- review-before-apply refresh; grace window and attribution.
- `ADR-97` -- the per-type claim expiry and `urzua reverify` that this milestone now builds.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `ADR-32` (no stored hash, unconditional drift) and `RFC-2` (hash, grace window, attributed refresh) conflict and the tension needs resolving before drift can be promoted past `warn`. | **substantive** |
> | 2026-11-04 | Decision recorded: all three, each opt-in and off by default: content-hash pins on evidence, a configurable grace window in days, and a warning when a claim exceeds a configured age. `Status` stays `Planned`. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | Built: opt-in `@sha256:` evidence pins, `drift_grace_days` and `claim_max_age_days` on `embodiment.consistency` (`SPEC-2`, `SPEC-3`, `SPEC-8`); the attributed re-baseline is left out and the pin is re-set by hand. `Status` stays `Planned`. **Why:** the 2026-11-04 decision names three opt-in pieces and none of them is a re-baseline command. | **substantive** |
> | 2026-11-04 | Fixed a defect in the built grace window (`BUG-179`): inside `drift_grace_days` a record now agrees with either its tier or a stated `Drift detected`, and `urzua fix` no longer flips a stated `Drift detected` while the drift is only inside the window. `Status` stays `Planned`. **Why:** the window computed the tier value, so it contradicted records that honestly state `Drift detected`. | **substantive** |
> | 2026-11-04 | Evidence so far, no `Realized-by` yet. **Why:** pins, the grace window and the claim age are built as opt-in options (`embodiment.rs`, `evidence_pins_integration.rs`, merged in #174), drafted in `ADR-67`, which is still Proposed; the attributed re-baseline is not decided or built. A decision milestone is `Done` once the decision is accepted, which the maintainer owns. `Status` stays Planned. | **substantive** |
> | 2026-11-04 | Checked against its Done condition after re-baselining shipped (#238, `MILE-181`): the three decided opt-ins are built and match `ADR-67`, and so is the attributed re-baseline the earlier row left out. `Status` stays `Planned`, with no `Realized-by`. **Why:** a decision milestone is `Done` once the decision is accepted, and `ADR-67` is still `Proposed`; acceptance is the maintainer's. What remains: accepting `ADR-67` (with its amendment of 2026-11-04), then setting `Realized-by` and `Done` here. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the maintainer accepted `ADR-67` on 2026-11-04, which is this record's Done condition; pins, the grace window, the claim age and the attributed re-baseline are built (#174, #182, #238). Checked against the Done means and the code. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | `Status` `Done` -> `InProgress`. Status set from its Done-means items (ADR-96, MILE-215): unresolved item 7. **Why:** the item has no evidence clause, and `DoneWithUnresolvedItems` forbids `Done` while an item is unresolved; `fix` never downgrades, so the status is set by hand to the computed value. | **structural** |
> | 2026-11-07 | Done means item 7 (`claim_max_age_days` exercised in this repository's config) replaced by five items that implement `ADR-97` (Proposed): the two rules, `urzua reverify`, the specs and changeset, and this repository's declaration; a Decision section records why. `Status` stays `InProgress`. **Why:** the rule-wide option cannot be declared here without 14 ADR and spec warnings, reaches only records stating `Embodiment`, and treats any edit to the `Realized-by` line as a renewal; maintainer direction of 2026-11-07. | **substantive** |
