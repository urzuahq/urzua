---
Stable-Id: 01M31GYJ2C8HXXB3J7W5AQYR6G
Status: Done
Phase: '1'
Track: schema-governance
Implements: RFC-39
Blocked-on: —
---
# 107 — Make the engine's declared vocabulary and its projections unmistakable at compile time

## What

`RFC-39`'s implementation, in two halves.

**Values.** `FieldName`, `RecordId`, `StatusValue` -- types whose `Eq`/`Hash` *is* their comparison
rule, so the rule is stated once instead of at every call site.

**Projections.** The ten rule signatures taking a bare collection derived from configuration converge
on `&Config`, which fifteen rules already do, or on named projection types. The rule to hold: a bare
collection derived from configuration does not cross a function boundary.

## Why

Not theoretical. Both halves shipped a defect in the `0.4.0` work, on the same day.

**Values.** `ADR-57` decided field names compare exactly. Implementing that one decision took three
passes, because the comparison lived at six call sites across three modules -- and between the first
pass and the last, `header.field-set-consistency` reported a field undeclared while
`header.required-fields` read the same field as satisfied. **The engine gave two answers about one
field**, caught only because a reviewer read past the diff.

`normalize_id` is the same shape and is load-bearing today: thirteen hand-applied call sites, correct
at all thirteen, with nothing enforcing the fourteenth. Missing it reintroduces `BUG-2`.

**Projections.** `field.untrimmed-value` was wired to `known_fields_by_type` where it needed
`declared_fields_by_type`. Both are `&HashMap<String, HashSet<String>>`; the first is absent for a type
declaring no `known_fields`, deliberately, and the second is populated for every type. Identical to the
compiler, opposite in meaning, and the distinction recorded in a comment. The rule silently skipped
every type declaring only `required_fields` -- a rule added to catch values nothing else checks, wired
so it would not check them.

Six further parameters are `&HashMap<String, Vec<String>>`, two of them adjacent arguments of
`pointer_resolution`. Swapping them compiles.

## The evidence that a type would have held

`Population` has private fields, one constructor and a `compile_fail` doctest. **It has never been
wrong.** Over the same release, `records_examined` -- a bare `usize` maintained beside a filter -- was
wrong four times (`BUG-78`, `BUG-81`, `BUG-83`, `BUG-90`), and `eligible`, derived structurally from
the candidate list, has never been wrong once.

The codebase already knows this. Fourteen enums exist where a string would have done. What it has not
done is apply the same reasoning to values the *adopter* declares, which is what `ADR-53` turned into
an open set -- and `urzua_core::Status` and `Embodiment` sit in `lib.rs` declared and never
constructed, the fossil of that handover.

## Sequencing

`FieldName` first: no existing normalization to preserve, and `ADR-57` already specifies its
comparison, so it is the cheap proof the pattern works.

The projections second, by converging on `&Config`. `RFC-39` decides this rather than leaving it open:
the safe pattern is already the majority in `rules.rs`, so it is demonstrated rather than proposed.

`RecordId` last and carefully. Thirteen working call sites, and the risk is a conversion that
normalizes where the old code did not or fails to where it did. Every current caller wants a test
pinning present behaviour **before** the type is introduced.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-21 | Projection approach fixed to `&Config` per `RFC-39`'s decision, rather than left as two options. | **structural** |
> | 2026-09-21 | Filed. **Why:** `RFC-39` was a `Draft` with no work item, and both halves of it caused a shipped defect during the `0.4.0` review on the day it was filed. Filed so the work is scheduled rather than left as a proposal, and so the evidence stays attached to it while it is fresh. | **substantive** |
> | 2026-09-22 | `Status: Planned` → `Done`. **Why:** `ADR-59` decided the scope precisely (`FieldName` and `RecordId` built; the `&Config` convergence done for all ten signatures; `StatusValue` decided against, not deferred) and both were implemented and verified: full test suite, `make ci`, and a real-corpus `check` run reporting the same 70 findings before and after. `RecordTypeConfig`'s own fields stay `Vec<String>` -- narrower than this record's original "Values" section proposed, corrected in `ADR-59` before implementation once counting construction sites found over ninety of them, almost all test fixtures never implicated in a comparison bug. | **substantive** |
