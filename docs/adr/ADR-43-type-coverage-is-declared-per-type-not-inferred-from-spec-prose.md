---
Status: Accepted
Stable-Id: 01M1ZHBEPZ8S9XNM9WYRCA0JP3
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/config.rs#RecordTypeConfig::spec@sha256:a71c8c3e50dd94f101ff345f73a43ca1269413348749fd09660ac1c9cc6af5b6, code:rust/crates/urzua-core/src/rules/type_scope.rs@sha256:7f98f5799993088360735c671f692eef26e465a1bee087d0d9b8a127e08ce62b, test:rust/crates/urzua-core/src/rules/type_scope.rs@sha256:7f98f5799993088360735c671f692eef26e465a1bee087d0d9b8a127e08ce62b
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
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/config.rs#RecordTypeConfig` (lines 443-583) f5e9550b598f -> cf828abc3ffa. Reason: BUG-74 renamed Waiver to Suppression (and load_waivers to load_suppressions) in this file's internals; the behaviour this record cites is unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (BUG-177, attested by this PR's review): re-pinned `rust/crates/urzua-core/src/config.rs#RecordTypeConfig` (lines 443-592) cf828abc3ffa -> 1e191cf68941. Reason: BUG-177 added the optional `effective_requires` field to `RecordTypeConfig`; the behaviour this record cites is unchanged **Why:** the pinned evidence changed and the PR review is the human re-verification that the claim still holds. | **structural** |
> | 2026-11-05 | Narrowed again, to the fields this decision is about (re-pinned by beauwilliams; the maintainer confirmed the narrowing on 2026-11-05): `config.rs#RecordTypeConfig` (lines 443-592, 1e191cf68941) is replaced by `config.rs#RecordTypeConfig::spec`, each a struct field addressed by the rust resolver's `struct` container and `indent` field member (`ADR-74`, `MILE-185`): `spec` (lines 545-555, a71c8c3e50dd). Reason: the `spec` declaration and its doc comment (omission is surfaced by `type.no-declared-spec`, never an opt-out) are unchanged since the whole-struct pin. **Why:** a pin on the whole struct went stale on every new per-type option (`BUG-177` was the latest) although the claim here concerns only `spec`; each span was read against the declaration it names. | **structural** |
> | 2026-11-05 | Cited the `ADR-74` amendment of 2026-11-05 (accepted) as the recorded decision for the resolver grammar the field pins above use: a list-valued `member` and a container that matches a struct, addressing a field by the `indent` end kind. No pin changed. **Why:** the review of #319 asked for the grammar extension to be a recorded decision; the narrowing itself was confirmed by the maintainer on 2026-11-05. | **structural** |
> | 2026-11-05 | Corrects the row of 2026-11-05 that narrowed the `config.rs#RecordTypeConfig` pin to `spec` (`#319`). The line numbers that row quotes (`spec` 545-555) are those of `config.rs` at `#295`; at the merge commit of `#319` the spans are `spec` 706-716. The narrowing was pure: the text of the field is byte-identical since `#295` (the content hashes quoted by that row are unchanged), so no pin was stale and none is changed here. **Why:** an independent check of the re-pin reasons found the earlier row incomplete or inaccurate; corrected here without rewriting it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/type_scope.rs` dbdcaf431cd2 -> 7f98f5799993. Reason: Read the diff of each pinned span since the old pin and the claim still holds. rules/type_scope.rs: only test fixture Config literals gained the new default fields, no rule code changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
