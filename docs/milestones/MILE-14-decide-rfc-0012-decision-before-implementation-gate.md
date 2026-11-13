---
Status: InProgress
Stable-Id: 01M1Y5JKGMK9VX40WZQFBQ6TTR
Phase: '1'
Track: governance-process
Implements: RFC-12
Blocked-on: —
---
# 14 — Decide RFC-12: decision-before-implementation gate

## What

Refuse the merge, not the edit.

## Why

Still Draft; genuinely undecided.

## Done means

- An ADR decides whether merges are refused when a change lands ahead of its decision (the merge, not the edit), names the CI step that enforces it, and `RFC-12` points at it.
- If adopted, a planted change citing an unaccepted decision fails that step in urzua's own `.github/workflows/`, and a decided one passes. — evidence: code:.github/workflows/checks.yml, test:rust/crates/urzua-cli/tests/diff_gate_integration.rs#this_repositorys_own_gate_holds_a_source_change_until_a_decision_fix_or_link_changes@sha256:5b44d257559b5bfc2212cb8a1c8de6f0b2c865502fce2068f82a165860de3f91

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | Tool-authored (by beauwilliams, identity: gh-verified, actor: unspecified): recomputed `Status` from `the Done means items (1 of 2 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `InProgress`. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/diff_gate_integration.rs#this_repositorys_own_gate_holds_a_source_change_until_a_decision_fix_or_link_changes` (lines 594-652) 5fa13c8a322e -> 5b44d257559b. Reason: diff_gate_integration.rs: the fixture takes the repository's config through `common::own_config_for_fixture`, which drops the `drift_baseline` line a throwaway repository cannot resolve; the assertions are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
