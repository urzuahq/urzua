---
Stable-Id: 01M317SJDB5EK301M3JKSCSNYH
Status: Accepted
Date: 2026-09-21
Accepted-on: 2026-09-21
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: ADR-53
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/rules/header.rs@sha256:8daab0cdc431844d3709d53363612118131a256706059621d64448e5682f5511, code:rust/crates/urzua-core/src/header.rs@sha256:138444217009c90bb532bed4f31605269e6a53ccde85651e142fc1da5ea1e654, test:rust/crates/urzua-core/src/property.rs@sha256:0eedac31a1d5cf4a5cd19515c4bd762f0f10b1dc3152253e1a3c382d0df4e943
---
# 57 — field names are compared exactly

## Context

`header.field-set-consistency` decides whether a header field's key names a field the record's type
declared. That comparison was case-insensitive: both sides were lower-cased before being compared, so
`Status` and `status` counted as the same field.

`BUG-97` recorded that this is wrong for any vocabulary that is not ASCII. The lower-casing was
ASCII-only, so `CAFÉ` became `cafÉ` while a declared `café` stayed itself: a field the config **did**
declare is reported undeclared, and no spelling the adopter can write will make it match.

The property suite (`MILE-101`) then found a harder case than the record predicted: the German sharp
s, `ß`/`SS`. Full Unicode lower-casing does not fix that one, because lower-casing and matching
case-insensitively are different operations — the latter is called *case folding* and needs its own
Unicode table.

## Options considered

| option | fixes `café`/`CAFÉ` | fixes `ß`/`SS` | cost |
|---|---|---|---|
| keep ASCII lower-casing | no | no | the defect stands |
| full Unicode lower-casing | yes | no | none, and still wrong |
| the `caseless` crate | yes | yes | a dependency, and the `purity.rs` allowlist edited |
| generate our own Unicode table | yes | yes | ~12 KB of data we own and regenerate |
| **compare exactly** | **n/a** | **n/a** | **behaviour change for a corpus matching loosely today** |

`caseless` was evaluated seriously: built against the pinned `1.87.0`, run against all four cases,
and its issue tracker read — including a hang report against the exact function proposed, which does
not reproduce on the released version.

It was rejected on a different ground. **Matching loosely fails in both directions, and the second
direction is worse.** `Maße` and `Masse` are different German words; every correct implementation
makes them one field, because treating `ß` as `ss` is exactly what fixes `straße`/`STRASSE`. In this
rule a false match means accepting a field name nobody declared — a *missing* finding, which `ADR-55`
names as the defect class this engine exists to remove. The ASCII version produced noisy wrong
findings; a correct version produces silent wrong silences.

Measured before deciding: this corpus matched **1788 field names exactly, with zero case-only
differences**. The loose comparison had never once done any work.

## Decision

In the context of a comparison that must decide whether two spellings name one field, facing loose
comparisons that are wrong in one direction or the other, we decided **field names are compared
exactly.**

A different spelling is a different field. `ADR-53` makes the field vocabulary the adopter's
declaration, and exact comparison is the only reading under which that declaration means what it
says: matching loosely is the engine guessing which two spellings are *really* one name, which is a
judgement about the adopter's vocabulary that the engine is not entitled to make.

A case-only difference is still reported, and the finding **names the declared spelling**: `field
'status' is not declared for record type 'adr' -- the type declares 'Status', differing only in case`.
The hint uses a lowercase comparison, which is safe precisely because it is a hint: a reader already
has a finding in front of them, and an approximate suggestion cannot cause a wrong verdict.

`BUG-97` closes without a fix. There is no loose comparison left to get wrong.

**This applies to every comparison of a field *name*, not to one rule.** `Header::get`, which every
rule uses to read a field's value, matched case-insensitively — so `header.field-set-consistency`
reported `status` undeclared while `header.required-fields` read the same field as satisfying
`Status`, and the engine gave two answers about one field. `duplicate_keys` and `discovery`'s
`Realized-by` lookup had the same split. All are exact.

It does **not** apply to field *values*. `field_state.rs` matches a value against `PLACEHOLDER_TOKENS`
case-insensitively, and that list is the engine's own, not the adopter's: there is no declaration to
overreach against, and someone typing `TBD` left a placeholder by any reading. `BUG-50`'s revision log
already records that this list should become declared config under `MILE-98` — **when it does, this
decision applies to it**, because it will then be the adopter's declaration like any other.

## Reversibility

Cheap. The comparison is one function, `rules::field_is_declared`, and reinstating a loose
comparison is editing its body. The property suite states the current rule as an executable claim and would fail loudly, so
a reversal cannot be silent.

## Consequences

- An adopted corpus that writes `status` where its config declares `Status` now gets a finding where
  it previously did not. That is the intended behaviour change and the only one.
- The `İ`/`i` question disappears rather than being declined. So does normalization (NFC versus NFD
  `é`) — two strings that differ in bytes are two names, which is the same answer exactly matching
  gives everywhere else.
- No dependency, no `purity.rs` allowlist change, no Unicode data to track and no version to drift.
- Two rule tests were relying on the loose comparison: both declared lowercase names against records writing
  capitalised ones, and passed only because `check.rs` lowercased declarations while building the
  allowed set. Both were corrected to the spellings a real config contains.
- A record writing `status` where its type declares `Status` now draws two findings rather than one
  half-answer: the required field is missing, and the written one is undeclared with its declared
  spelling named. Measured no-op on this corpus — 68 findings and zero errors before and after.
- **The comparison is still spread across call sites.** Nothing stops the next one choosing its own
  equality; a field name is a `String`, and four different comparisons were live when this was
  written. Making it a type whose equality is defined once is the structural answer and is deliberately
  not attempted here.
- `SPEC-2`'s rule description does not mention case sensitivity either way and needs no change; this
  ADR is the first statement of the rule.

## Amendment (2026-11-04): reserved keys are read case-insensitively

*Status of this amendment: accepted 2026-11-04 by the maintainer (drafted 2026-11-04 from the drift review the maintainer approved). The Decision text above is unchanged.*

The Decision says the rule "applies to every comparison of a field *name*". `Header::get_reserved`
(`rust/crates/urzua-core/src/header.rs`, `#102`) is a deliberate exception: for the keys the engine
itself invents (`Stable-Id`, `Rule`, `Scope`, `Expires`) it folds ASCII case, so a record spelling
`stable-id` is read as `Stable-Id` and `migrate ids` does not assign it a second id, and a waiver
spelling `rule` still applies. The reasoning is the decision's own: exact comparison protects the
adopter's vocabulary from the engine's guesses, and a reserved key is not adopter vocabulary, because
there is exactly one field of that name. `get_reserved` is used by `migrate ids`, `export` and the
waiver loader. `Header::get`, and every comparison of an adopter-declared field name, remain exact.

## References

- `ADR-53` — every rule is the adopter's declared policy, which is why the vocabulary is theirs to
  state.
- `ADR-55` — a false match is a missing finding, which is what ruled out a correct implementation.
- `BUG-97` — the defect this closes.
- `MILE-101` — the property suite that found the harder case and now states this rule.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-21 | Widened from one rule to every field-*name* comparison, and the boundary against field *values* stated. **Why:** the decision was implemented in `field_is_declared` alone while `Header::get` still matched case-insensitively, so two rules disagreed about one field on the same record -- a decision the engine half-implemented, which is what `BUG-98` records about `ADR-50`. | **substantive** |
> | 2026-09-21 | Initial decision, `Status: Accepted`. **Why:** `BUG-97` was filed expecting a one-line swap to a better case-insensitive comparison. The property suite found that no such comparison is correct in both directions, and measuring the corpus showed this one had never matched anything exact comparison would not have. Deleting it rather than improving it closes the bug and removes the surface it lived on. | **substantive** |
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | `Realized-by` locators pinned by content hash (`@sha256:`) after re-verifying each cited file against the Decision. **Why:** Re-verified against current code; maintainer decision 2026-11-04. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit ce6048953. | **structural** |
> | 2026-11-04 | Amendment added: `Header::get_reserved` reads the engine's reserved keys case-insensitively (`#102`), an exception to "every comparison of a field name". **Why:** drift review of 2026-11-04, approved by beauwilliams; proposed, pending maintainer acceptance. The Decision is unchanged. | **substantive** |
> | 2026-11-04 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/rules/header.rs` 17e08ea792a9 -> 7c679be9601d, `rust/crates/urzua-core/src/header.rs` 7eeae176eeeb -> 5c7df87d90f4, `rust/crates/urzua-core/src/property.rs` 78acfe16ded9 -> dd31b4f429bc. Reason: Re-pinned with the amendment above: Header::get and declared-field comparison remain exact; the reserved-key exception is recorded. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-04 | Amendment of 2026-11-04 accepted. **Why:** Maintainer decision, 2026-11-04; the Decision text is unchanged. | **substantive** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/rules/header.rs` 7c679be9601d -> 52b3c1143b21, `rust/crates/urzua-core/src/property.rs` dd31b4f429bc -> fc7ebda282a5. Reason: Re-verified against the Decision and its accepted amendment of 2026-11-04. Since the pin, rules/header.rs gained suggested_action and action_source on its findings (ADR-73) and property.rs gained fixture fields and the extra rule arguments of the property harness; Header::get and the declared-field comparison remain exact, and the reserved-key exception is the one the amendment records. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/header.rs` 5c7df87d90f4 -> 3aab1e8d5082, `rust/crates/urzua-core/src/property.rs` fc7ebda282a5 -> b48cfbe7f2d6. Reason: BUG-74 renamed Waiver to Suppression (and load_waivers to load_suppressions) in this file's internals; the behaviour this record cites is unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-07 | Corrects the 2026-11-05 row that re-pinned `rust/crates/urzua-core/src/property.rs` fc7ebda282a5 -> b48cfbe7f2d6. The reason it gave, that `BUG-74` renamed Waiver to Suppression in this file's internals, was copied from the `header.rs` row and is false for this file. The actual change is three added `role: None,` lines in the `RecordTypeConfig` constructions of the file's test fixtures, for the new `role` field of the record-type config; no rename, and no change outside the `tests` module. `Header::get` and the declared-field comparison this record cites are untouched, so the claim still holds. **Why:** an independent check of the re-pin reasons found this one inaccurate; corrected here without rewriting the earlier row. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/header.rs` 52b3c1143b21 -> 2ee979c62600, `rust/crates/urzua-core/src/header.rs` 3aab1e8d5082 -> 6bce44f1b7cf, `rust/crates/urzua-core/src/property.rs` b48cfbe7f2d6 -> 0eedac31a1d5. Reason: Read the diff of each pinned span since the old pin and the claim still holds. rules/header.rs: header.field-set-consistency gained per-type extension prefixes (MILE-141), a case-only near miss suggests a rename and never a removal, and a type that declares no required fields is not judged here, field names are still compared exactly; urzua-core/src/header.rs: header parsing now strips a leading byte-order mark, keeps non-string keys, keeps the text a number or boolean was written as, prefers the top-level key over a nested key of the same name and starts a comment only at a # after a space or tab, field names are still compared exactly; urzua-core/src/property.rs: a property test now treats a type that does not require a revision log as leaving a record without one unexamined (BUG-50), the exact-field-name properties are unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/header.rs` 2ee979c62600 -> cd81c077715c. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: header.rs: split header_required_fields into record_scoped_header_findings. Behaviour unchanged, outputs byte-identical (check, check --base and fix --rebaseline JSON compared before and after). **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/header.rs` 6bce44f1b7cf -> 138444217009. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: header.rs: split parse_yaml_frontmatter into header_without_region and frontmatter_fields; the opening-line, closing-line and YAML-error checks stay in parse_yaml_frontmatter in the same order with the same parse_error text. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline and reverify JSON compared before and after, with and without --as-of). **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/header.rs` cd81c077715c -> 8daab0cdc431. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: header.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
