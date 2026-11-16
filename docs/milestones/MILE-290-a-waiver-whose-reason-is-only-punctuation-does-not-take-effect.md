---
Stable-Id: 01M7HW0JV72KQ2GD2JTRCQHEQM
Status: Done
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 290 — A waiver whose reason is only punctuation does not take effect

## What

Treat a required waiver field that holds only a dash or other punctuation as blank, as the override log already does.

## Why

Found on 2026-11-16 while urzua's own builders and reviewers used the tool to land a week of pull requests. Two reviewers reproduced that a waiver with a reason of a single em dash takes effect and suppresses its finding, while the override log's check reads dashes as blank.

## Done means

- A waiver whose `Reason` is `—`, `-` or other punctuation with no letter or digit does not take effect and is reported, shown by a test. — evidence: test:rust/crates/urzua-core/src/waiver.rs@sha256:d6cc43b080a82784348e1bc8acb0878119491029b3bea8542f4b8417d17101a6, code:rust/crates/urzua-core/src/waiver.rs#SuppressionType::field_state@sha256:9721f54466cb8a90bc28ba4328f0fda4b07c84f7ec5f68d81477669165a4eb95
- The override log and the waiver judgement share one definition of a blank value. — evidence: code:rust/crates/urzua-core/src/field_state.rs#is_blank_value@sha256:68d194272a2d0231da621533dc2e8726a72b099533f7abfd7d7cb4e5eb46a217, code:rust/crates/urzua-core/src/rules/override_log.rs#reviewer_state@sha256:a0bb6f40f1f85826489245aaa4d46f5847f2afd1b69a112c9ca440c8df72ddb2, test:rust/crates/urzua-core/src/field_state.rs@sha256:cb856694a3b6b55d6dea39f71543ef1cc9e179319d26f8374adb22c34b224efa

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (2 of 2 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
