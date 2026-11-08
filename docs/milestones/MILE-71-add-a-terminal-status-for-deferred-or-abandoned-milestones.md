---
Status: Done
Stable-Id: 01M1YS1MJX3Z91EJ5MXZ1XEGQA
Phase: '0'
Track: schema-governance
Implements: ADR-34, SPEC-6
Blocked-on: —
Realized-by: code:.urzua/config.yaml@sha256:653bb0f54ca6c2a3ac5009060305356e6ea00960928a6d8f50fb991d63314a5d, spec:docs/specs/SPEC-6-the-milestone-record-type.md, code:.urzua/templates/milestone.md@sha256:bebac786c6a93c577c831898921f16df98e304560bfbcd3ad36fe38acb687559
---
# 71 — Add a terminal status for deferred or abandoned milestones

## What

Extend milestone's `Status` enum (`Planned | InProgress | Blocked | Done`, ADR-34/SPEC-6) with a
terminal state for a milestone that's deliberately deprioritized or decided against, distinct from
`Blocked`'s "waiting on something specific." `bug` already has `WontFix`; `adr` already has
`Rejected`/`Superseded`. Whether any record type's `Status` should be validated against a closed
set at all is adjacent scope, not decided here (see MILE-55).

## Why

Found by red-teaming a real Planned milestone (MILE-2) against "what if we decide not to do this
after all." No documented value fits: `Blocked` implies a nameable blocker, `Planned` left in place
misrepresents a reversed decision as still intended, and there's no `Superseded`-style pointer to a
replacement either. This is new capability for the state machine, not broken behavior -- nothing
crashes or emits wrong output today, the enum is just incomplete for a real case.

## Done means

- `WontDo` is a value of the milestone `Status` enum, declared as a failure status in urzua's own `.urzua/config.yaml`. — evidence: code:.urzua/config.yaml@sha256:653bb0f54ca6c2a3ac5009060305356e6ea00960928a6d8f50fb991d63314a5d
- `WontDo` is documented in `SPEC-6`. — evidence: spec:docs/specs/SPEC-6-the-milestone-record-type.md
- `WontDo` is listed in the milestone template. — evidence: code:.urzua/templates/milestone.md@sha256:bebac786c6a93c577c831898921f16df98e304560bfbcd3ad36fe38acb687559
- A deliberately abandoned milestone has an honest terminal state distinct from `Blocked`, which is derived and never a `Status` value. — evidence: code:.urzua/config.yaml@sha256:653bb0f54ca6c2a3ac5009060305356e6ea00960928a6d8f50fb991d63314a5d, spec:docs/specs/SPEC-6-the-milestone-record-type.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Fixed and shipped: `WontDo` added via ADR-34 amendment + SPEC-6 v0.2, template updated. **Why:** every other record type in this corpus already has a terminal "decided against" state (`bug`'s `WontFix`, `adr`'s `Rejected`/`Superseded`); milestone didn't, so a deferred or abandoned milestone had no honest representation. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: .urzua/config.yaml@sha256:151830e5926d, .urzua/templates/milestone.md@sha256:bebac786c6a9 Reason: Pinned the files as they are now after checking each against the record it evidences: section.rs, document_model.rs, table.rs and config.yaml still hold the decisionless-status exemption (MILE-7), config.yaml and the milestone template still carry the WontDo terminal status (MILE-71), the spec template and config.yaml still make urzua new spec work (MILE-74), field_state.rs, config.yaml, checks.yml and the Makefile still report a returning placeholder (MILE-78), config.yaml still declares the milestone type (MILE-8), table.rs, pointer.rs, commands/rules.rs, generate-rule-table.py and config.yaml still give every rule a configurable level (MILE-80), config.yaml still declares the per-type field sets (MILE-81), config.yaml, pointer.rs and narrative.rs still hold the opt-in narrative-field.stale rule (MILE-83), schema_version.rs, the corpus-a expectation and config.yaml still show the removed shapes refused (MILE-85), and new.rs, new_refs.rs, open_pr.rs, report.rs, config.rs, discovery.rs, check.rs, config.yaml and table.rs still hold the open pull request numbering check (MILE-89). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 151830e5926d -> 0b59c6dfc8b9. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. config.yaml: config.yaml gained Kind among the milestone type's known fields and a kind declaration (build and decision) under statuses.delivery.items (ADR-96 amendment of 2026-11-07), and every other declaration the record relies on is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0b59c6dfc8b9 -> 0a00038a6388. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: this change declares status.done-items-shape (error), status.waived-item-stale (warn) and embodiment.claim-renewal-unattested (warn) and declares relation.cross-type-disallowed, section.shape and diff.source-and-records-mixed as off; every other declaration is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0a00038a6388 -> 3d51c2fac8b7. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: only the comment above the three rules declared off changed, to say they report `not-enabled` exactly as an undeclared rule does; no declaration changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 3d51c2fac8b7 -> 653bb0f54ca6. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: the only change is a new optional pending list (two placeholder patterns) under record_types::milestone statuses.delivery.items; nothing else in the file changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
