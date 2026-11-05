---
Status: Accepted
Stable-Id: 01M1ZHBEPZ8S9XNM9WYRCA0JP3
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/config.rs#RecordTypeConfig@sha256:f5e9550b598fe4e9b196fd0d012026362e86355ecb590293795fac07ae5417f8, code:rust/crates/urzua-core/src/rules/type_scope.rs@sha256:dbdcaf431cd2a6b7fce85c17f78bfe28fa7a8a22d154cafa74642ce81c3d6e1d, test:rust/crates/urzua-core/src/rules/type_scope.rs@sha256:dbdcaf431cd2a6b7fce85c17f78bfe28fa7a8a22d154cafa74642ce81c3d6e1d
Date: 2026-09-08
Accepted-on: 2026-09-08
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

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | `Realized-by` locators pinned by content hash (`@sha256:`) after re-verifying each cited file against the Decision. **Why:** Re-verified against current code; maintainer decision 2026-11-04. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 27929a050. | **structural** |
> | 2026-11-04 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/config.rs` f8fe5e070ddb -> 251b70ec14f1, `rust/crates/urzua-core/src/rules/type_scope.rs` e2c01a1981cd -> 8dddf55562e6. Reason: Re-verified: type.no-declared-spec is unchanged; the added exclude_files warning is a sibling rule in the same file. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Narrowed to the item the decision concerns; re-pin approved by the maintainer's 2026-11-04 drift decision (`ADR-74`, `MILE-185`): `config.rs` to `RecordTypeConfig` (lines 424-556). **Why:** an unrelated edit to a volatile file must not stale this pin again; the spans were chosen by the author of this change and not reviewed line by line by the maintainer. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/rules/type_scope.rs` 8dddf55562e6 -> dbdcaf431cd2. Reason: Drift review of 2026-11-04, recommendation (a). Since the pin, type_scope.rs gained `extra_dirs` handling (MILE-130: a type's directories are `all_dirs()`, `exclude_files` matches across them, and a missing `extra_dirs` entry is a warning) and every finding carries `suggested_action` and `action_source` (ADR-73). The `type.no-declared-spec` condition is unchanged (a warning for each configured type with no `spec`), which is what this decision is about. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/config.rs#RecordTypeConfig` (lines 432-566) ff7dbbe0876e -> f5e9550b598f. Reason: RecordTypeConfig changed only in a doc comment (the MILE-118 dated note on the adopter premise); no field or behaviour changed, so the decision still holds **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
