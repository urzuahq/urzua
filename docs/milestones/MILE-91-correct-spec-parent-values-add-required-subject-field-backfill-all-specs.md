---
Stable-Id: 01M23VWVTFGHCT37FXR977BRQV
Status: Done
Phase: '0'
Track: schema-governance
Implements: BUG-10
Blocked-on: —
Realized-by: code:.urzua/config.yaml@sha256:cbb1c67c40a04562b517932dd635c9f9083378df4d05deeb58771286cc1487bb, code:.github/workflows/checks.yml@sha256:a201f4e57f5f91a353ee0a7449831c0d03263200ac8e83a8f2a7a5b54fbe95a9, spec:docs/bugs/BUG-10-parent-field-defaults-to-spec-1-across-most-specs-without-a-real-relationship.md, spec:docs/specs/SPEC-18-the-spec-record-type.md
---
# 91 — Correct spec Parent values, add required Subject field, backfill all specs

## What

Two changes to `spec`'s own schema, landed together since both touch every spec's header:

- `spec` gains a new required field, `Subject`: one line, free text, no cross-reference or backlink
  semantics -- what feature, record type, schema, or practice the spec is actually about, readable
  at a glance without opening the full `Purpose` section. Added to `.urzua/config.toml`'s
  `required_fields` for `spec`, backfilled onto all 18 real spec files in the same commit (a required
  field and its corpus backfill never land separately, same discipline as `MILE-74`'s `Author`
  backfill).
- `Parent` corrected per `BUG-10`'s audit: cleared to `—` on the seven specs where it never expressed
  a real relationship (`SPEC-6`/`9`/`10`/`16`/`17`/`18`/`19`), kept on the ten where `SPEC-1`'s own
  prose documents a genuine split-off relationship (`SPEC-1` itself, the 18th spec, carries no
  `Parent` field at all -- it's the root).
- `SPEC-18` updated to document `Subject` as required and to state `Parent`'s real, narrower meaning
  now that its data is corrected.
- `.urzua/templates/spec.md` corrected at the source: its throwaway header hardcoded
  `Parent: SPEC-1 (v0 CLI).` as literal boilerplate -- discarded by the live code path
  (`render_synthetic_yaml` only fills `required_fields`) but still the thing a human or agent would
  copy by eye when hand-authoring a new spec, which is almost certainly how the wrong default
  actually propagated. Replaced with an instructional comment.

## Why

`BUG-10` found `Parent: SPEC-1` was a mechanically-applied default on 7 of 18 specs, not a real
relationship -- their actual lineage was already fully expressed by their own `Implements`/
`Derives-from` fields. Fixing wrong data and adding a genuinely useful new field both touch every
spec's header, so they land in one pass rather than two separate corpus-wide edits.

No preceding RFC: this is the same kind of direct, `spec`-type-own schema decision `MILE-74`
made through live discussion without one -- a config/schema-only change to an already-config-driven
type, no new mechanism or rule.

## Done means

- `spec` requires a one-line `Subject` field in `.urzua/config.yaml`. — evidence: code:.urzua/config.yaml@sha256:cbb1c67c40a04562b517932dd635c9f9083378df4d05deeb58771286cc1487bb
- Every spec carries a `Subject`, and `urzua check` reports no missing `Subject`. — evidence: code:.urzua/config.yaml@sha256:cbb1c67c40a04562b517932dd635c9f9083378df4d05deeb58771286cc1487bb, code:.github/workflows/checks.yml@sha256:a201f4e57f5f91a353ee0a7449831c0d03263200ac8e83a8f2a7a5b54fbe95a9
- The corrected `Parent` values from `BUG-10` are in place on all specs. — evidence: spec:docs/bugs/BUG-10-parent-field-defaults-to-spec-1-across-most-specs-without-a-real-relationship.md
- `SPEC-18` documents both. — evidence: spec:docs/specs/SPEC-18-the-spec-record-type.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-09 | Initial milestone, `Status: Done`. **Why:** `BUG-10`'s audit plus a live decision to add `Subject` as a required field -- both executed together since they touch the same 18 files' headers. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: .urzua/config.yaml@sha256:151830e5926d, .github/workflows/checks.yml@sha256:4afa5fe0db59 Reason: Pinned the files as they are now after checking each against the record it evidences: declaration.rs and config.yaml still declare pointer_fields and narrative_fields per type (MILE-90), config.yaml still requires Subject on specs with checks.yml gating it (MILE-91), document_model.rs, both section.rs files, schema_version.rs and config.yaml still read the declared identity, fields and sections (MILE-98), .git-hooks/pre-push, the Makefile and checks.yml still run urzua check (MILE-99), waiver.rs, rules/waiver.rs and waiver_stale_integration.rs still implement the waiver type SPEC-10 describes, audit.rs and the own-config and check integration tests still implement SPEC-11, new.rs, new_record.rs and new_integration.rs still implement SPEC-12, explain.rs, both graph.rs files and the own-config test still implement SPEC-13, both migrate.rs files and check_integration.rs still implement SPEC-14, and doctor.rs and its tests still implement SPEC-15. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 151830e5926d -> 0b59c6dfc8b9, `.github/workflows/checks.yml` 4afa5fe0db59 -> 3bd0a6795184. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. config.yaml: config.yaml gained Kind among the milestone type's known fields and a kind declaration (build and decision) under statuses.delivery.items (ADR-96 amendment of 2026-11-07), and every other declaration the record relies on is unchanged; checks.yml: the single rust job became parallel lint, build, three nextest test shards, four acceptance shards, an acceptance-gate and a result job (ADR-98), and every step the record relies on (fmt, clippy, deny, the records gate, the release guards, the rule table check, the gate action tests and harness status) is still run. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0b59c6dfc8b9 -> 0a00038a6388. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: this change declares status.done-items-shape (error), status.waived-item-stale (warn) and embodiment.claim-renewal-unattested (warn) and declares relation.cross-type-disallowed, section.shape and diff.source-and-records-mixed as off; every other declaration is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 0a00038a6388 -> 3d51c2fac8b7. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: only the comment above the three rules declared off changed, to say they report `not-enabled` exactly as an undeclared rule does; no declaration changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/checks.yml` 3bd0a6795184 -> a201f4e57f5f. Reason: checks.yml: the only change since the pinned version is the four tool-download curl lines (cargo-deny, cargo-machete), which gained --retry 5 --retry-all-errors --retry-delay 5 and changed -sSLfo to -sSLf with a separate -o (#441); no step, job or check changed, so the evidence still holds **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 3d51c2fac8b7 -> 653bb0f54ca6. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: the only change is a new optional pending list (two placeholder patterns) under record_types::milestone statuses.delivery.items; nothing else in the file changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 653bb0f54ca6 -> 594d115541e3. Reason: config.yaml: read the diff since the previous pin; the only change is one added line, outside_writes: shared, under the harness key (this repository declares the new harness.outside_writes option, ADR-89 amendment of 2026-11-09). No record type, rule, status, relation or other declaration this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 594d115541e3 -> 2f0e023ed5a5. Reason: Read the diff of each pinned file or span since the old pin against what this record evidences. .urzua/config.yaml: embodiment.claim-expired gains `drift: {code: block, test: block, spec: acknowledge}` and its explanatory comment; nothing else in the file changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 2f0e023ed5a5 -> cbb1c67c40a0. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml: read the diff since the old pin; the only change is the added key authority.decider_table: base with its two-line comment (this repository declares the new decider_table option, ADR-89 amendment of 2026-11-09). No record type, rule, status, relation or other declaration this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
