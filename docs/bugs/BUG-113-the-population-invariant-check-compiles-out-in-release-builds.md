---
Stable-Id: 01M34MYQXW4DDFCM9S6ZNTMG08
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Realized-by: test:rust/crates/urzua-core/src/report.rs#a_population_cannot_examine_more_than_it_was_eligible_for@sha256:c43bd0d3e473d8a3961955e3a64a609c0eca7e4ca9d757091931acd837fbb16f
---
# 113 — the population invariant check compiles out in release builds

## What was wrong

`Population::detailed` guarded `examined <= eligible` with `debug_assert!`, which compiles to nothing
in a release build — the exact build profile `make ci` and an adopter's own CI actually run. If a
future rule bug ever produced `examined > eligible`, a release binary would silently fall through to
`eligible.max(examined)`, raising `eligible` to match `examined` — the function's own comment names
this outcome "the most reassuring wrong answer available." The debug-only assert meant that outcome
was reachable in exactly the build nothing was checking for it.

## Why nothing caught it

`debug_assert!` is the conventional choice for an expensive, hot-path invariant check that isn't worth
paying for in a release binary. This one runs once per rule per `check` invocation — not a cost
worth trading the invariant away for.

## What changed

`debug_assert!` is now `assert!`, active in every build profile.

## References

- `ADR-55` — "a check that reports success without looking" is the defect class a debug-only assert
  in a release binary reintroduces for this specific invariant.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed and fixed in one pass. **Why:** found by a full-release code review; a one-line change, verified by confirming the existing regression test's message and by a full release-profile build. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
