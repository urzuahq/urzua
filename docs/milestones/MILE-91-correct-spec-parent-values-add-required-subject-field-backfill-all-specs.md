---
Stable-Id: 01M23VWVTFGHCT37FXR977BRQV
Status: Done
Phase: '0'
Track: schema-governance
Implements: BUG-10
Blocked-on: —
Realized-by: code:.urzua/config.yaml@sha256:151830e5926d1887be39a723d110744ff5e2ae8c408bb114a90ba3e2b8316a2e, code:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d, spec:docs/bugs/BUG-10-parent-field-defaults-to-spec-1-across-most-specs-without-a-real-relationship.md, spec:docs/specs/SPEC-18-the-spec-record-type.md
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

- `spec` requires a one-line `Subject` field in `.urzua/config.yaml`. — evidence: code:.urzua/config.yaml@sha256:151830e5926d1887be39a723d110744ff5e2ae8c408bb114a90ba3e2b8316a2e
- Every spec carries a `Subject`, and `urzua check` reports no missing `Subject`. — evidence: code:.urzua/config.yaml@sha256:151830e5926d1887be39a723d110744ff5e2ae8c408bb114a90ba3e2b8316a2e, code:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d
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
