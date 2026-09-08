---
Status: Done
Stable-Id: 01M1Z2ZV7XGR7GSMNPWDACBX5J
Phase: '1'
Track: schema-governance
Implements: ADR-39
Blocked-on: —
---
# 81 — A field-set consistency rule: undeclared fields per type

## What

A rule catching field-set drift: a header field present on some records of a type but not others,
distinct from `header.layout-consistency` (MILE-75/ADR-38, which checks how existing fields are
arranged, never which ones exist). `required_fields` alone can't drive this -- ADRs and RFCs already
carry fields (`Stable-Id`, `Realized-by`, `Derives-from`, `Supersedes / Superseded-by`) that are
legitimately common but never marked required, so "only required_fields may appear" would flag the
entire existing corpus as a false positive. Needs a declared, per-type **known/optional fields**
list, separate from `required_fields` -- same "declared, not voted" principle `header_layout`
already uses, applied to a different axis (which fields exist, not how they're laid out). A field
outside both `required_fields` and the new declared list is the actual finding.

## Why

Found live: SPEC-1 carries `Embodiment`/`Author`/`Derives-from` that SPEC-2 through SPEC-6 don't --
neither `header.required-fields` (only flags a *missing* required field) nor
`header.layout-consistency` (checks arrangement, not field existence) catches it. The MILE-75/ADR-38
design work already rejected majority-vote inference for the layout axis for good reason (ratifies
whatever's already drifted in); the same reasoning applies here, so this needs the same declared-set
treatment rather than inferring a type's "normal" fields from whatever's already in the corpus.
`spec`'s own `known_fields` declaration stays blocked on MILE-74 (does `Author` become required? is
SPEC-1's `Embodiment`/`Derives-from` project-wide or SPEC-1-specific?) -- the rule simply doesn't
check `spec` yet, the same additive-skip shape as every undeclared type.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Fixed and shipped (ADR-39): a `known_fields` config field, additive to `required_fields`, checked by a new `header.field-set-consistency` rule. **Why:** majority-vote inference was rejected for the same reason ADR-38 rejected it for the layout axis -- it would ratify existing drift instead of catching it. Declared `known_fields` for `adr`/`rfc`/`milestone`/`bug`/`waiver`; `spec` deliberately left undeclared pending MILE-74. Verified against a planted violation in an isolated scratch corpus, not only unit tests. | **substantive** |
