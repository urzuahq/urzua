---
Status: Accepted
Stable-Id: 01M1ZHBEPZ8S9XNM9WYRCA0JP3
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/config.rs, code:rust/crates/urzua-core/src/rules.rs, code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-core/src/rules.rs, test:rust/crates/urzua-core/src/config.rs
Date: 2026-09-08
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: ADR-41
---
# 43 — Type coverage is declared per type not inferred from spec prose

## Context

Reviewing the spec set against the ADR set (MILE-77/ADR-41) found `adr`, `rfc`, and `spec`-the-type
had no schema spec at all, discovered by manual review — the same class of gap ADR-41 itself named
for `bug`/`waiver` before SPEC-9/SPEC-10 were written. Nothing makes "which configured record types
currently have no governing spec" visible without a human re-reading the whole spec set by eye each
time, the exact failure mode this project's own tooling otherwise exists to eliminate.

ADR-41 already settled that "does this type need a spec" is an editorial judgment, not a mechanical
formula — a proposed alternative (infer coverage from a spec's title or prose mentioning the type by
name) was considered and rejected here for the same reason majority-vote inference was rejected for
`header_layout` (ADR-38) and `known_fields` (ADR-39): a spec's title matching a type name by
substring is exactly the kind of fragile, un-declared signal this project's rules consistently avoid
building on.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| No mechanism; rely on manual review to notice a type has no spec | No new code | Already produced the exact gap this ADR responds to |
| Infer coverage from a spec's title/prose mentioning the type name | No new config | Fragile substring matching — rejected for the same reason inference was rejected elsewhere in this schema |
| Declare a `spec` pointer per type in `.urzua/config.toml`, flag types with none declared | Explicit, diffable, matches `header_layout`/`known_fields`'s existing precedent exactly | One more optional config key |

## Decision

In the context of a real, undetected gap (three founding types with no spec) found only by manual
review, facing a choice between inferring coverage and declaring it, we decided: **`RecordTypeConfig`
gains an optional `spec: Option<String>` field**, and a new rule, `type.no-declared-spec`, emits a
`Warning` for every configured record type with no `spec` declared. This is an inventory signal, not
a mandate — ADR-41's editorial judgment stays the only thing that decides whether a type actually
needs one; a type can permanently have `spec` undeclared if that's the right call. The rule examines
`Config` directly, not `&[Record]` — the first rule in this codebase to do so, since this is a
question about the schema itself, not about any individual record.

Verified against the real, live gap before writing any of the three missing specs: the rule was
confirmed to fire for all six configured types (including `milestone`/`bug`/`waiver`, which already
had `SPEC-6`/`SPEC-9`/`SPEC-10` — the pointer itself had simply never been wired into config, the
same class of miss as MILE-59's waiver-type registration gap) before any `spec` pointer was declared,
and confirmed silent afterward.

## Reversibility

Fully additive: one optional config field, one rule examining config rather than records. Removing
either leaves every other rule and the existing schema untouched.

## Consequences

- A new rule, `type.no-declared-spec`, joins `check`'s rule set at `Warning` severity.
- `RecordTypeConfig` gains `spec: Option<String>`.
- This repo's own config now declares `spec` for all six types: `milestone -> SPEC-6`,
  `bug -> SPEC-9`, `waiver -> SPEC-10`, `adr -> SPEC-16`, `rfc -> SPEC-17`, `spec -> SPEC-18`.
- The declared `spec` pointer is not itself resolved/verified against the real spec set by this ADR
  — a real, named follow-up gap: a typo'd or dangling `spec` value would currently go undetected the
  same way a `Blocked-on` reference did before ADR-40/42 extended `pointer.resolution`. Worth
  revisiting once a real case demands it, the same staging discipline used throughout this schema.

## References

- ADR-41 — the "coherent feature area, decided editorially" principle this rule makes visible
  without ever attempting to automate.
- ADR-38/ADR-39 — the "declared, not voted/inferred" precedent this rule follows exactly, applied to
  a new axis (spec coverage) rather than layout or field set.
- ADR-40/ADR-42 — the `pointer.resolution` extensions this ADR's own named follow-up gap would
  eventually parallel, if `spec` pointers are ever worth resolving mechanically.
