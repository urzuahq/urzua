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

- A waiver whose `Reason` is `—`, `-` or other punctuation with no letter or digit does not take effect and is reported, shown by a test. — evidence: test:rust/crates/urzua-core/src/waiver.rs, code:rust/crates/urzua-core/src/waiver.rs#SuppressionType::field_state
- The override log and the waiver judgement share one definition of a blank value. — evidence: code:rust/crates/urzua-core/src/field_state.rs#is_blank_value, code:rust/crates/urzua-core/src/rules/override_log.rs#reviewer_state, test:rust/crates/urzua-core/src/field_state.rs

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (2 of 2 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
