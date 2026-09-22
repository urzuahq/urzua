---
Stable-Id: 01M31BXW2WXKPY60FN5164WEQ1
Status: Accepted
Date: 2026-09-21
Author: beauwilliams
Supersedes / Superseded-by: —
Implements: SPEC-3
Realized-by: spec:docs/milestones/MILE-107-make-the-engine-s-declared-vocabulary-and-its-projections-unmistakable-at-compile-time.md
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

### The same conflation one level up: projections of the configuration

Filed as a proposal about declared *values*. A defect shipped and caught the same day shows the
*structure* has it too.

`check.rs` projects `Config` into bare collections and hands them to rules. Ten rule signatures take
one; fifteen take `&Config` and project internally. The fifteen cannot be called wrongly. The ten can,
because the projections are indistinguishable to the compiler:

| line | parameter | type |
|---|---|---|
| `rules.rs:279` | `allowed_by_type` | `&HashMap<String, HashSet<String>>` |
| `rules.rs:1597` | `declared_by_type` | `&HashMap<String, HashSet<String>>` |

Identical. `field.untrimmed-value` was wired to the first when it needed the second, which compiled
cleanly and made the rule **silently skip every type declaring only `required_fields`** -- a rule
added to catch values nothing else checks, wired so it would not check them.

Six further parameters are `&HashMap<String, Vec<String>>`, and two of them are **adjacent arguments
of the same function**:

```rust
pub fn pointer_resolution(
    records: &[Record],
    pointer_fields_by_type: &HashMap<String, Vec<String>>,
    narrative_fields_by_type: &HashMap<String, Vec<String>>,
) -> (RuleExecution, Vec<Finding>)
```

Swapping them compiles. Nothing at the call site would say so.

**And the absence semantics differ while the types do not.** `known_fields_by_type` has no entry for a
type that declares no `known_fields`, deliberately, because `header.field-set-consistency` skips such
a type. `declared_fields_by_type` is populated for every type. Same Rust type, opposite meaning for a
missing key, and the difference recorded only in a comment -- which is what was misread.

This is the same failure as the value half: a type that cannot express a distinction forces every
consumer to guess it, and a guess in a projection is invisible where a guess in a comparison at least
produces a wrong answer someone might notice.

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

### The structural half: a projection is a type, or there is no projection

Two options, and the second is already the majority practice in this file.

**Every projection is a named type.** `KnownFields`, `DeclaredFields`, `PointerFields`,
`NarrativeFields` -- each wrapping its map and carrying its own absence semantics in doc and in
construction, so passing one where another is wanted does not compile.

**Or rules take `&Config` and project internally.** Fifteen already do. There is then nothing to pass
wrongly, at the cost of each rule re-deriving what it needs and of `urzua-core` rules depending on the
config type rather than on plain data.

The second is cheaper, already proven here, and removes the class outright rather than naming it. The
first preserves the current shape and is more work for the same guarantee.

**Decided: converge on `&Config`.** A rule taking a bare collection derived from configuration is the
exception and has to argue for itself. Wrapper types are not pursued -- they would add four types to
preserve a shape whose only merit is that it is the current one, and fifteen rules already demonstrate
the alternative works.

Either way the rule is: **a bare collection derived from configuration does not cross a function
boundary.**

### Scope

**In:** `FieldName`, `RecordId`, `StatusValue`. Config deserializes into them; `HeaderField.key`
becomes one; `Header::get` takes one.

**In:** the ten rule signatures taking a projected map, converged on `&Config` or on named projection
types.

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
> | 2026-09-21 | Projection direction decided: converge on `&Config` rather than introduce wrapper types. **Why:** fifteen rules already take it and none of them can be called wrongly, so the alternative is demonstrated rather than proposed. Wrappers would add four types to preserve a shape whose only argument is incumbency. | **substantive** |
> | 2026-09-21 | Widened to cover projections of the configuration, not only declared values. **Why:** a Major defect shipped and was caught the same day from the same root -- `field.untrimmed-value` was wired to `known_fields_by_type` where it needed `declared_fields_by_type`, two parameters of identical type with opposite absence semantics, so the rule silently skipped every type declaring only `required_fields`. Six further parameters share a type, two of them adjacent arguments of one function that would compile if swapped. Fifteen rules already take `&Config` and cannot be called wrongly, so the safe pattern is present and applied to a minority. | **substantive** |
> | 2026-09-21 | Filed. **Why:** `ADR-57` took three passes to implement because the comparison it decided lives at six call sites, and between the first pass and the last two rules disagreed about one field on one record. `normalize_id` is the same shape with thirteen hand-applied calls and is load-bearing today. Filed while the evidence was in front of us rather than after the release, and deliberately not implemented in `0.4.0`. | **substantive** |
> | 2026-09-22 | `Status: Draft` → `Accepted`; decided by `ADR-59`. **Why:** `RFC-40`/`ADR-58` landed in between and independently confirmed the same defect family (`BUG-100`, `BUG-101`), and answered this RFC's open question about `StatusValue` with evidence rather than leaving it open a second time -- `ADR-59` decides against building it. `FieldName` and `RecordId` are built; the `&Config` convergence proceeds as this RFC already decided. Implemented in `0.4.0`, reversing the prior entry's deferral, because the evidence for it kept arriving rather than going stale. | **substantive** |
