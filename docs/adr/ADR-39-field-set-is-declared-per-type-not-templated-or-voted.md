---
Status: Accepted
Stable-Id: 01M1Z3HTHA9VHQX8WCXXX9TQBA
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/config.rs#RecordTypeConfig::required_fields@sha256:fd6d896ef55879037e81e47021ef21367337edc0b94492f9d0eb98c809386aa1, code:rust/crates/urzua-core/src/config.rs#RecordTypeConfig::known_fields@sha256:4c4719ddeff5537ae842caa5b6911e864ab3e0e2d89e46f875bc4f22f95058b7, code:rust/crates/urzua-core/src/rules/header.rs@sha256:8daab0cdc431844d3709d53363612118131a256706059621d64448e5682f5511, test:rust/crates/urzua-core/src/rules/header.rs@sha256:8daab0cdc431844d3709d53363612118131a256706059621d64448e5682f5511
Date: 2026-09-07
Accepted-on: 2026-09-07
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: RFC-10
---
# 39 — Field set is declared per type not templated or voted

## Context

Neither `header.required-fields` nor `header.layout-consistency` (ADR-38) catches a field present
on some records of a type but not others: SPEC-1 carries `Embodiment`/`Author`/`Derives-from` that
SPEC-2 through SPEC-6 don't, and nothing flagged it. `required_fields` only checks for a *missing*
required field, never for a field's mere *presence* being unexpected; `layout-consistency` checks
how existing fields are arranged, never which ones exist.

A naive fix -- treat `required_fields` as the complete, closed set for a type -- was checked
against the real corpus first and rejected immediately: ADRs and RFCs already carry fields
(`Stable-Id`, `Realized-by`, `Derives-from`, `Supersedes / Superseded-by`) that are legitimately
common but never marked required. Closing the set to `required_fields` alone would flag the entire
existing corpus as a false positive the moment the rule shipped.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| `required_fields` is the closed set, nothing else allowed | No new config | False-positives on every ADR/RFC's existing legitimate optional fields (`Stable-Id`, `Realized-by`, etc.) |
| Infer a type's "normal" fields from majority vote among existing records | No new config, works immediately | Ratifies whatever's already drifted in rather than catching it -- rejected for the same reason as `header.required-fields` and ADR-38's `header_layout` |
| Declare `known_fields` per type in `.urzua/config.toml`, additive to `required_fields` | Works without breaking existing optional fields; explicit, diffable; a type with none declared is simply unchecked | One more optional config key to document |

## Decision

In the context of a field's mere presence-or-absence across a type's records having no check at
all, facing a choice between inferring a type's allowed fields and declaring them, we decided:
**`known_fields` is an optional per-type config field, additive to `required_fields`, checked by a
new `header.field-set-consistency` rule.** A field not in `required_fields` ∪ `known_fields` for its
type is a `Warning` finding. A type with no declared `known_fields` is skipped entirely -- additive,
never a forced migration, and deliberately left undeclared for `spec` until MILE-74 decides its
canonical field set, rather than guessed at here.

This repo's own config now declares `known_fields` for every type with more than one settled shape:
`adr` (`Embodiment`, `Realized-by`, `Stable-Id`, `Derives-from`, `Supersedes / Superseded-by`),
`rfc` (`Supersedes / Superseded-by`, `Amends`), `milestone` (`Stable-Id`, `Implements`), `bug`
(`Realized-by`, `Stable-Id`), `waiver` (`Stable-Id`, `Expires`) -- `spec` intentionally excluded.

## Reversibility

Fully additive and cheap to reverse: `known_fields` is an optional config key and
`field-set-consistency` is one more rule in the existing list. Removing either leaves every other
rule and the existing schema untouched.

## Consequences

- A new rule, `header.field-set-consistency`, joins `check`'s rule set at `Warning` severity,
  consistent with how new rules have been introduced against a corpus not yet held to them.
- `RecordTypeConfig` gains `known_fields: Option<Vec<String>>`.
- MILE-81's question is answered: field-set drift gets a rule, declared per type, not inferred.
  Verified against a planted violation in an isolated scratch corpus, not only unit tests.
- `spec` stays unchecked by this rule until MILE-74 resolves its canonical field set -- SPEC-1's
  `Embodiment`/`Author`/`Derives-from` remain visible drift, tracked, not silently accepted.

## Amendment (2026-11-04): `spec` now declares `known_fields`

*Status of this amendment: accepted 2026-11-04 by the maintainer (drafted 2026-11-04 from the drift review the maintainer approved). The Decision text above is unchanged.*

The Decision and its Consequences leave `known_fields` undeclared for `spec` until `MILE-74` settles
the spec field set. `.urzua/config.yaml` now declares `known_fields` for `spec`, so
`header.field-set-consistency` checks specs like the other types. The mechanism (a per-type
`known_fields` list, additive to `required_fields`, opt-in per type) is unchanged.

## References

- RFC-10 — the closed-header model this decision extends to a new axis (which fields exist, not
  how they're arranged).
- ADR-38 — the header-layout-consistency precedent this decision follows exactly, on the field-set
  axis instead of the layout axis.
- MILE-74 — the milestone that must resolve before `spec` can get a `known_fields` declaration.
- MILE-81 — the milestone this ADR resolves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | `Realized-by` locators pinned by content hash (`@sha256:`) after re-verifying each cited file against the Decision. **Why:** Re-verified against current code; maintainer decision 2026-11-04. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 9f42b6db5. | **structural** |
> | 2026-11-04 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/config.rs` f8fe5e070ddb -> 251b70ec14f1, `rust/crates/urzua-core/src/rules/header.rs` 17e08ea792a9 -> 7c679be9601d. Reason: Re-verified: header.field-set-consistency exists and known_fields is still per-type and additive to required_fields. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-04 | Amendment added: `spec` declares `known_fields` in `.urzua/config.yaml`, where the Decision leaves it undeclared pending `MILE-74`. **Why:** drift review of 2026-11-04, approved by beauwilliams; proposed, pending maintainer acceptance. The Decision is unchanged. | **substantive** |
> | 2026-11-04 | Amendment of 2026-11-04 accepted. **Why:** Maintainer decision, 2026-11-04; the Decision text is unchanged. | **substantive** |
> | 2026-11-05 | Narrowed to the item the decision concerns; re-pin approved by the maintainer's 2026-11-04 drift decision (`ADR-74`, `MILE-185`): `config.rs` to `RecordTypeConfig` (lines 424-556). **Why:** an unrelated edit to a volatile file must not stale this pin again; the spans were chosen by the author of this change and not reviewed line by line by the maintainer. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/rules/header.rs` 7c679be9601d -> 52b3c1143b21. Reason: Drift review of 2026-11-04, recommendation (a). The only change to header.rs since the pin is that each finding now carries `suggested_action` and an `action_source` (ADR-73, #275); no rule condition changed, and `header.field-set-consistency` still warns on a field outside the type's required_fields and known_fields. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/config.rs#RecordTypeConfig` (lines 432-566) ff7dbbe0876e -> f5e9550b598f. Reason: RecordTypeConfig changed only in a doc comment (the MILE-118 dated note on the adopter premise); no field or behaviour changed, so the decision still holds **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/config.rs#RecordTypeConfig` (lines 443-583) f5e9550b598f -> cf828abc3ffa. Reason: BUG-74 renamed Waiver to Suppression (and load_waivers to load_suppressions) in this file's internals; the behaviour this record cites is unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (BUG-177, attested by this PR's review): re-pinned `rust/crates/urzua-core/src/config.rs#RecordTypeConfig` (lines 443-592) cf828abc3ffa -> 1e191cf68941. Reason: BUG-177 added the optional `effective_requires` field to `RecordTypeConfig`; the behaviour this record cites is unchanged **Why:** the pinned evidence changed and the PR review is the human re-verification that the claim still holds. | **structural** |
> | 2026-11-05 | Narrowed again, to the fields this decision is about (re-pinned by beauwilliams; the maintainer confirmed the narrowing on 2026-11-05): `config.rs#RecordTypeConfig` (lines 443-592, 1e191cf68941) is replaced by `config.rs#RecordTypeConfig::required_fields`, `config.rs#RecordTypeConfig::known_fields`, each a struct field addressed by the rust resolver's `struct` container and `indent` field member (`ADR-74`, `MILE-185`): `required_fields` (lines 489-490, fd6d896ef558), `known_fields` (lines 513-521, 4c4719ddeff5). Reason: `required_fields` is a `Vec<String>` and `known_fields` an `Option<Vec<String>>` with the doc comment stating that an omitted list means the field set is not checked since the whole-struct pin. **Why:** a pin on the whole struct went stale on every new per-type option (`BUG-177` was the latest) although the claim here concerns only these two fields; each span was read against the declaration it names. | **structural** |
> | 2026-11-05 | Cited the `ADR-74` amendment of 2026-11-05 (accepted) as the recorded decision for the resolver grammar the field pins above use: a list-valued `member` and a container that matches a struct, addressing a field by the `indent` end kind. No pin changed. **Why:** the review of #319 asked for the grammar extension to be a recorded decision; the narrowing itself was confirmed by the maintainer on 2026-11-05. | **structural** |
> | 2026-11-05 | Corrects the row of 2026-11-05 that narrowed the `config.rs#RecordTypeConfig` pin to `required_fields` and `known_fields` (`#319`). The line numbers that row quotes (`required_fields` 489-490, `known_fields` 513-521) are those of `config.rs` at `#295`; at the merge commit of `#319` the spans are `required_fields` 650-651 and `known_fields` 674-682. The narrowing was pure: the text of both fields is byte-identical since `#295` (the content hashes quoted by that row are unchanged), so no pin was stale and none is changed here. **Why:** an independent check of the re-pin reasons found the earlier row incomplete or inaccurate; corrected here without rewriting it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/header.rs` 52b3c1143b21 -> 2ee979c62600. Reason: Read the diff of each pinned span since the old pin and the claim still holds. rules/header.rs: header.field-set-consistency gained per-type extension prefixes (MILE-141), a case-only near miss suggests a rename and never a removal, and a type that declares no required fields is not judged here, the declared-field-set check itself is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/header.rs` 2ee979c62600 -> cd81c077715c. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: header.rs: split header_required_fields into record_scoped_header_findings. Behaviour unchanged, outputs byte-identical (check, check --base and fix --rebaseline JSON compared before and after). **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/header.rs` cd81c077715c -> 8daab0cdc431. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: header.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
