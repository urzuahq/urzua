---
Stable-Id: 01M317SJDB5EK301M3JKSCSNYH
Status: Accepted
Date: 2026-09-21
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: ADR-53
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/rules.rs, test:rust/crates/urzua-core/src/property.rs
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
- `SPEC-2`'s rule description does not mention case sensitivity either way and needs no change; this
  ADR is the first statement of the rule.

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
> | 2026-09-21 | Initial decision, `Status: Accepted`. **Why:** `BUG-97` was filed expecting a one-line swap to a better case-insensitive comparison. The property suite found that no such comparison is correct in both directions, and measuring the corpus showed this one had never matched anything exact comparison would not have. Deleting it rather than improving it closes the bug and removes the surface it lived on. | **substantive** |
