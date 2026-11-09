---
Status: Done
Stable-Id: 01M1Z8QDN72FCDTD839NSCEYBC
Phase: '1'
Track: schema-governance
Implements: ADR-42
Blocked-on: —
Realized-by: code:.urzua/config.yaml@sha256:594d115541e37b3e3c0e602a4ef18d82eaf99100174a048133003a2bd80ee78b, code:rust/crates/urzua-core/src/rules/pointer.rs@sha256:2d164d27a2e9d20a589adeee671eaae8db4d5e6529b3e705e9df1d1b0ed6e5ae, test:rust/crates/urzua-core/src/rules/narrative.rs#a_blocked_on_pointer_to_a_terminal_bug_is_a_stale_finding@sha256:6a29cc6cc16f6aa9b5b2ac71337ca9514d6e800f1ee8593067ed50ba0fb72265, code:rust/crates/urzua-core/src/rules/narrative.rs@sha256:b34355340066c63614438add0c4da8230a9aafd84c991834a59414eaa09eaa74, test:rust/crates/urzua-core/src/rules/narrative.rs#a_blocked_on_pointer_to_a_non_terminal_record_is_not_stale@sha256:0ab576c334c88153e9f5dcb7fbbd8fbe88c9fd7a1a0b41ad035d6e9bca42633a
---
# 83 — A staleness drift rule for Blocked-on and similar prose pointers

## What

Two things, decided together since the second gates the first: (1) whether `Blocked on` (and any
similar field describing a dependency in prose) should be required to cite a real record ID rather
than free text, the same way `Implements`/`Derives-from`/`Parent` already must; and (2) a drift rule
that flags a `Blocked on` pointer whose target has since reached a terminal state (`Fixed`,
`Accepted`, `Done`) as likely-stale and needing a human recheck — the same shape of problem
`embodiment.consistency` (ADR-32) already solved for `Embodiment` via git-blame-based drift
detection, applied to a different field.

## Why

Found live during a backlog triage: MILE-2 and MILE-3 both list `Blocked on: Milestone: Fix urzua
new's template-priority bug` — that bug (BUG-3) has been `Fixed` for a while, but nothing caught the
blocker going stale, because `Blocked on` is free-text prose, not a checkable pointer. If it had
been `Blocked on: BUG-3`, a drift rule could have flagged it the moment BUG-3's `Status` changed. A
milestone whose stated blocker silently resolved and nobody noticed defeats the purpose of tracking
the blocker at all.

## Done means

- `Blocked-on` is a checked header field of the milestone type, declared a narrative field in config. — evidence: code:.urzua/config.yaml@sha256:594d115541e37b3e3c0e602a4ef18d82eaf99100174a048133003a2bd80ee78b
- `pointer.resolution` reports a `Blocked-on` reference that does not resolve. — evidence: code:rust/crates/urzua-core/src/rules/pointer.rs@sha256:2d164d27a2e9d20a589adeee671eaae8db4d5e6529b3e705e9df1d1b0ed6e5ae
- The opt-in rule `narrative-field.stale` reports a resolved reference whose target has reached a terminal status; a fixture covers a stale blocker. — evidence: test:rust/crates/urzua-core/src/rules/narrative.rs#a_blocked_on_pointer_to_a_terminal_bug_is_a_stale_finding@sha256:6a29cc6cc16f6aa9b5b2ac71337ca9514d6e800f1ee8593067ed50ba0fb72265, code:rust/crates/urzua-core/src/rules/narrative.rs@sha256:b34355340066c63614438add0c4da8230a9aafd84c991834a59414eaa09eaa74
- `narrative-field.stale` is silent on a live blocker; a fixture covers one. — evidence: test:rust/crates/urzua-core/src/rules/narrative.rs#a_blocked_on_pointer_to_a_non_terminal_record_is_not_stale@sha256:0ab576c334c88153e9f5dcb7fbbd8fbe88c9fd7a1a0b41ad035d6e9bca42633a
- Urzua's own milestones carry `Blocked-on` in the header and run the rule. — evidence: code:.urzua/config.yaml@sha256:594d115541e37b3e3c0e602a4ef18d82eaf99100174a048133003a2bd80ee78b

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Fixed and shipped (ADR-42): `Blocked-on` moved from a body section into `milestone`'s header (all 84 files migrated); `pointer.resolution` gained `Blocked-on` for dangling-reference checking; a new `blocked_on_stale` rule flags a resolved `Blocked-on` reference whose target reached a terminal status. **Why:** design review found `Blocked on` was the one field-shaped exception across the entire schema still living as unchecked prose, not just this one field's problem in isolation -- moving it into the header was more consistent than building a second, body-text-specific checking path. Verified against the real, live MILE-2/3 case: the rule was confirmed firing before the actual fix was applied, not only against synthetic fixtures. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: .urzua/config.yaml@sha256:151830e5926d, rust/crates/urzua-core/src/rules/pointer.rs@sha256:e4d8db46f2e3, rust/crates/urzua-core/src/rules/narrative.rs@sha256:1b08f9fad547 Reason: Pinned the files as they are now after checking each against the record it evidences: section.rs, document_model.rs, table.rs and config.yaml still hold the decisionless-status exemption (MILE-7), config.yaml and the milestone template still carry the WontDo terminal status (MILE-71), the spec template and config.yaml still make urzua new spec work (MILE-74), field_state.rs, config.yaml, checks.yml and the Makefile still report a returning placeholder (MILE-78), config.yaml still declares the milestone type (MILE-8), table.rs, pointer.rs, commands/rules.rs, generate-rule-table.py and config.yaml still give every rule a configurable level (MILE-80), config.yaml still declares the per-type field sets (MILE-81), config.yaml, pointer.rs and narrative.rs still hold the opt-in narrative-field.stale rule (MILE-83), schema_version.rs, the corpus-a expectation and config.yaml still show the removed shapes refused (MILE-85), and new.rs, new_refs.rs, open_pr.rs, report.rs, config.rs, discovery.rs, check.rs, config.yaml and table.rs still hold the open pull request numbering check (MILE-89). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 151830e5926d -> 0b59c6dfc8b9. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. config.yaml: config.yaml gained Kind among the milestone type's known fields and a kind declaration (build and decision) under statuses.delivery.items (ADR-96 amendment of 2026-11-07), and every other declaration the record relies on is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0b59c6dfc8b9 -> 0a00038a6388. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: this change declares status.done-items-shape (error), status.waived-item-stale (warn) and embodiment.claim-renewal-unattested (warn) and declares relation.cross-type-disallowed, section.shape and diff.source-and-records-mixed as off; every other declaration is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0a00038a6388 -> 3d51c2fac8b7. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: only the comment above the three rules declared off changed, to say they report `not-enabled` exactly as an undeclared rule does; no declaration changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 3d51c2fac8b7 -> 653bb0f54ca6. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: the only change is a new optional pending list (two placeholder patterns) under record_types::milestone statuses.delivery.items; nothing else in the file changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/pointer.rs` e4d8db46f2e3 -> 2d164d27a2e9, `rust/crates/urzua-core/src/rules/narrative.rs` 1b08f9fad547 -> b34355340066. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: pointer.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged; narrative.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 653bb0f54ca6 -> 594d115541e3. Reason: config.yaml: read the diff since the previous pin; the only change is one added line, outside_writes: shared, under the harness key (this repository declares the new harness.outside_writes option, ADR-89 amendment of 2026-11-09). No record type, rule, status, relation or other declaration this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
