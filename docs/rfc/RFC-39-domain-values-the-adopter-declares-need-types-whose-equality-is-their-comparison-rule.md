---
Stable-Id: 01M31BXW2WXKPY60FN5164WEQ1
Status: Draft
Date: 2026-09-21
Author: beauwilliams
Supersedes / Superseded-by: —
Implements: SPEC-3
---
# 39 — Domain values the adopter declares need types whose equality is their comparison rule

## Summary

A field name, a record identifier and a status are all `String` in this engine, so every comparison
site chooses its own equality. Four different choices were live simultaneously, two rules disagreed
about one field on one record, and a normalization function is applied by hand at thirteen call sites
with nothing enforcing the fourteenth.

This proposes that a value drawn from the adopter's declared vocabulary is a **type whose `Eq` and
`Hash` encode its comparison rule**, so the rule is stated once and a call site cannot choose
differently without saying so in code a reviewer can see.

## Motivation

### The comparison was wrong four ways at once

`ADR-57` decided field names compare exactly. Implementing it took three passes, because the
comparison is not in one place:

| site | what it did |
|---|---|
| `rules::field_is_declared` | `to_ascii_lowercase` — fixed first |
| `config.pointer-field-not-known` | `to_ascii_lowercase` — found by review |
| `config.pointer-narrative-overlap` | `to_ascii_lowercase` — found by review |
| `Header::get` | `eq_ignore_ascii_case` — found after that |
| `Header::duplicate_keys` | `to_ascii_lowercase` — same pass |
| `discovery`'s `Realized-by` lookup | `eq_ignore_ascii_case` — same pass |

Between the first fix and the last, `header.field-set-consistency` reported `status` undeclared on a
record while `header.required-fields` read the same field as satisfying `Status`. **The engine gave
two answers about one field**, and the only reason it was caught is that a reviewer read past the
diff.

Each fix was correct and none was structural. A seventh site written tomorrow can pick a seventh
equality, and nothing will say so.

### `normalize_id` is the same shape, already load-bearing

```rust
pub(crate) fn normalize_id(id: &str) -> String   // ADR-0034 -> ADR-34
```

Applied by hand at **thirteen call sites** across `rules.rs` and `graph.rs`. It is correct at all
thirteen today. A fourteenth that forgets it compares `ADR-0034` against `ADR-34` and finds no match —
which is `BUG-2`'s defect, reachable again at any time, in a normalizer whose discipline is
convention.

This is the more dangerous half of the problem and the reason it is worth an RFC rather than a
milestone: field names had no normalization to lose, but `normalize_id` is relied on and any change
to how it is applied has to preserve thirteen working call sites.

### The engine has types for its own vocabulary and none for the adopter's

Fourteen enums exist where a string would have done — `FieldState`, `HeaderShape`, `HeaderLayout`,
`PopulationUnit`, `RuleStatus`, `ScopeSource`. `Population` goes further: private fields, one
constructor, a `compile_fail` doc test proving the invariant cannot be bypassed. **That type has never
been wrong.**

There are **zero newtypes** in `urzua-core`. `grep "^pub struct [A-Z]\w*("` returns nothing.

So the pattern is applied to *closed* sets — where the engine can enumerate every variant — and never
to *open* sets, where the values come from the adopter. Field names, record identifiers and statuses
as written are all open sets, and all three are bare `String`.

### And the split is visible in two dead enums

`urzua_core::Status` (`Proposed | Accepted | Rejected | Superseded`) and `urzua_core::Embodiment` are
declared in `lib.rs` and **never constructed anywhere in the workspace**. Meanwhile `rules.rs` reads
statuses as `&str` from `header.get("Status")` and compares them against config-declared strings.

`ADR-53` is why. It made every rule the adopter's declared policy, which moved statuses out of the
engine's closed set and into the adopter's open one — and the enum, which could only ever hold the
four values *this* project uses, stopped being usable. Nothing replaced it, so the comparison fell
back to `String`. The dead enum is the fossil of the moment the vocabulary changed hands.

## Proposal

### The rule

**A value drawn from the adopter's declared vocabulary is a type. Its `Eq`/`Hash` implementation is
its comparison rule, and that is the only place the rule appears.**

Not "no normalization" — each domain keeps whatever comparison it already needs, moved from the call
sites into the type:

| type | comparison | source |
|---|---|---|
| `FieldName` | exact | `ADR-57` |
| `RecordId` | numeric-normalized (`ADR-0034` == `ADR-34`) | `BUG-2`, `ADR-36` |
| `StatusValue` | trimmed, exact | declared terminal/closed status lists |

### What this buys that discipline does not

- A call site cannot compare two `FieldName`s loosely without `.as_str()` on both, which is visible in
  review and greppable in a way `eq_ignore_ascii_case` scattered across three modules is not.
- `normalize_id` stops being a thing to remember: constructing a `RecordId` normalizes, and there is
  no un-normalized form to accidentally compare.
- The comparison rule has one place to carry its own doc comment and its own test, instead of the
  rule living in an ADR and the behaviour living in six functions.

### Scope

**In:** `FieldName`, `RecordId`, `StatusValue`. Config deserializes into them; `HeaderField.key`
becomes one; `Header::get` takes one.

**Out:** field *values* generally. `field_state.rs` matches `PLACEHOLDER_TOKENS` case-insensitively
and that list is the engine's own, not the adopter's — `ADR-57` already draws this boundary. It moves
in scope if `MILE-98` makes those tokens declared config.

**Out:** the two dead enums. Whether `Status`/`Embodiment` should be deleted or rebuilt as
adopter-facing types is a separate question this RFC surfaces rather than answers.

### Migration

`FieldName` first — it has no existing normalization, so there is nothing to preserve and `ADR-57`
already specifies the comparison. It is the cheap proof that the pattern works.

`RecordId` second and carefully: thirteen working call sites, and the risk is a conversion that
normalizes in a place the old code did not, or fails to where it did. Every one of `normalize_id`'s
current callers wants a test pinning its present behaviour **before** the type is introduced.

`StatusValue` last, and only if the first two pay off.

## Open questions

- **Does `FieldName` deserialize directly via `serde`**, or does config hold `String` and convert at
  load? Direct is tidier and makes a malformed name a config parse error; conversion at load keeps
  config errors in one place and may give better messages.
- **What is `RecordId`'s `Display`** — the normalized form or what the author wrote? Findings currently
  quote the author's spelling, and normalizing in messages would change output people read.
- **Is `StatusValue` worth it at all?** Statuses are compared against config-declared lists rather
  than looked up, so the failure mode is weaker. It may be enough to leave them `String` and record
  why, which is a real answer rather than a deferral.
- **Do the dead `Status`/`Embodiment` enums get deleted in the same pass**, or does removing them want
  its own record given they are cited in prose across the corpus?
- **Does this want a `newtype` macro** or three hand-written types? Three is more code and no
  dependency; a macro is less code and one more thing to understand. At three types, hand-written
  probably wins.

## Non-goals

- **Not a rewrite.** Nothing about rule behaviour, the report contract, or the CLI changes. This moves
  where a comparison is written, not what it decides.
- **Does not reopen `ADR-57`.** Exact comparison for field names is decided; this makes it structural.
- **Does not ship in `0.4.0`.** That release already breaks the report contract and the field-name
  comparison. A type-system change on top would make the release untestable as one unit, and round 12
  has not happened.

## References

- `ADR-57` — field names compare exactly; three follow-up fixes in one day are this RFC's evidence.
- `ADR-53` — moved the vocabulary from the engine to the adopter, which is what stranded the enums.
- `BUG-2`, `ADR-36` — why `RecordId` normalizes rather than compares exactly.
- `BUG-40` — the same lesson in a different place: `eligible` is the candidate list's length precisely
  so it cannot drift from the rule's own skips. A count derived structurally has never been wrong; a
  count maintained beside a filter was wrong four times.
- `Population` (`report.rs`) — the in-repo precedent for an invariant upheld by construction.
- `MILE-98` — would move `PLACEHOLDER_TOKENS` into declared config and this RFC's scope with it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-21 | Filed. **Why:** `ADR-57` took three passes to implement because the comparison it decided lives at six call sites, and between the first pass and the last two rules disagreed about one field on one record. `normalize_id` is the same shape with thirteen hand-applied calls and is load-bearing today. Filed while the evidence was in front of us rather than after the release, and deliberately not implemented in `0.4.0`. | **substantive** |
