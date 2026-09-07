# 81 — A field-set consistency rule: undeclared fields per type

> Status: Planned
> Stable-Id: 01M1Z2ZV7XGR7GSMNPWDACBX5J
> Phase: 1
> Track: schema-governance
> Implements: —

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

## Blocked on

MILE-74 -- deciding `spec`'s own canonical field set (does `Author` become required? is SPEC-1's
`Embodiment`/`Derives-from` project-wide or SPEC-1-specific?) is a live instance of exactly what this
rule would need a declared list for. Resolving MILE-74 first gives this rule a real, decided target
for `spec` rather than a second guess.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
