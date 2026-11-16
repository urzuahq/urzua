---
Stable-Id: 01M7HVZZ7RE9TDFS6WV4BMHSW7
Status: InProgress
Phase: '1'
Track: embodiment-model
Blocked-on: null
---
# 288 — A pin that changes without a re-pin row is a finding

## What

Report a record whose evidence pin changed between the base and the head when no revision-log row written by the re-pin tooling accompanies the change.

## Why

Found on 2026-11-16 while urzua's own builders and reviewers used the tool to land a week of pull requests. Three reviewers reproduced the same gap: changing a pin's hash by hand to the file's current hash turns `check` green, under `--base` too, with no finding and no reason. The rule that a re-pin needs a reason holds only for the command, not for the record.

## Done means

- `urzua check --base` reports a record whose pin hash differs from the base without an accompanying re-pin row, naming the locator, shown by a test with a hand-edited pin. — evidence: test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#a_hand_edited_pin_fails_check_base_and_a_re_pin_with_the_tool_does_not@sha256:9c4f8cb13120e8628e68d618f833337ff7bf54ffd4b8fe47eab3132bc0717107, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#a_row_for_another_change_does_not_account_for_a_hand_edited_pin@sha256:3e22cc81a81817e46852212db3b9054d65589e27682064b9a6c1bf37fe1a3f1a, code:rust/crates/urzua-core/src/rules/pin_attestation.rs#diff_pin_changed_unattested@sha256:e0f06e5d5cc270a1aebf8c6c9ddda29ea4369eec86d640fde127ab88e79f246b
- A re-pin made with `urzua fix --rebaseline --apply`, a renewal and a pin recomputed by `urzua fix --conflicts` produce no such finding. — evidence: test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#a_hand_edited_pin_fails_check_base_and_a_re_pin_with_the_tool_does_not@sha256:9c4f8cb13120e8628e68d618f833337ff7bf54ffd4b8fe47eab3132bc0717107, test:rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#a_record_both_merged_sides_re_pinned_is_resolved_on_merge_observed_failing@sha256:729e1d29142afe808a2a65e6a93c5b5977e8dd182a65bda162a0e5c8230c3661, test:rust/crates/urzua-core/src/rules/pin_attestation.rs@sha256:737bfcf122fe1fe1fad4995059d964962b2a6b28d568bff209a667578c212e79, code:rust/crates/urzua-core/src/rules/pin_attestation.rs#moved@sha256:02977a22a0ddec3a913c2fd731a2b149970134633d04f6ab69e8c6b2b11e2f2d
- A pin added to a locator that had none is treated as the tool treats a first pin today, and that is stated in the spec. — evidence: spec:docs/specs/SPEC-2-urzua-check.md, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#a_removed_pin_is_reported_and_a_pin_on_a_new_record_is_not@sha256:d2af42d09ea02bf6b14c9e5c240d858305f789f4ea832821d9fdf6c7bcfb7b38, code:rust/crates/urzua-core/src/rules/pin_attestation.rs#unattested@sha256:01d9503a9c03844bda3e4cf6b364f67abf64cac55971964827d7ee3909bcb549
- The capability is exercised in urzua's own `.urzua/config.yaml` and corpus, and `urzua check` reports 0 unwaived findings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (3 of 4 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `InProgress`. | **structural** |
> | 2026-11-16 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 7 locators pinned, 1 file-level, 6 scoped. Evidence: rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#a_hand_edited_pin_fails_check_base_and_a_re_pin_with_the_tool_does_not@sha256:9c4f8cb13120, rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#a_row_for_another_change_does_not_account_for_a_hand_edited_pin@sha256:3e22cc81a818, rust/crates/urzua-core/src/rules/pin_attestation.rs#diff_pin_changed_unattested@sha256:e0f06e5d5cc2, rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#a_record_both_merged_sides_re_pinned_is_resolved_on_merge_observed_failing@sha256:729e1d29142a, rust/crates/urzua-core/src/rules/pin_attestation.rs@sha256:737bfcf122fe, rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#a_removed_pin_is_reported_and_a_pin_on_a_new_record_is_not@sha256:d2af42d09ea0, rust/crates/urzua-core/src/rules/pin_attestation.rs#unattested@sha256:01d9503a9c03 Reason: Pinned the tests and code each delivered Done-means item cites, as they stand in this pull request. **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-16 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 0 file-level, 1 scoped. Evidence: rust/crates/urzua-core/src/rules/pin_attestation.rs#moved@sha256:02977a22a0dd Reason: Pinned moved, the function that follows a chain of rows from the base's hash to the head's, which item 2 cites. **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
