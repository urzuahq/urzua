# 35 — `bug`: a configured record type requiring a named regression test

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
> Date: 2026-09-07
> Author: @beauwilliams
> Deciders: @beauwilliams
> Supersedes / Superseded-by: —
> Derives-from: ADR-34 (Accepted)

## Context

`check`'s `paths` argument was found, live, to do nothing: it located the repo root and was then
silently discarded, so `check docs/adr/` and `check docs/` always returned identical results.
Nothing caught this because nothing had ever exercised a genuinely narrower path — every existing
test called `check docs/`. This is exactly the "no check runs its own regex... no test caught it
because no real record ever needed the fallback" defect class this project's own practice already
names, discovered here about its own CLI argument rather than a rule.

The immediate question was whether to track this as a `milestone` (`Track: bug-fix`) or as its own
type. Checked against what a bug genuinely needs that a milestone doesn't: a milestone is
*prospective* (`Status`, `Phase`, `Track` — planned work moving toward done); a bug is
*retrospective* (what was actually wrong, how it was found, and — the one real governance gain — a
named, checkable pointer to the test that proves it's fixed). Forcing a bug into milestone's schema
loses that pointer or bolts it on as an optional field nothing requires.

## Decision

In the context of a real defect needing a home, facing a schema question milestone's own fields
don't answer well, we decided: **`bug` is a separate configured record type — zero `urzua-core`
changes, same footprint as `milestone` and `waiver`.**

```toml
[record_types.bug]
dir = "docs/bugs"
required_fields = ["Status", "Found-in", "Regression-test"]
```

- `Status`: `Open | Fixed | WontFix`.
- `Found-in`: how/where it was discovered — free text, since this is inherently a narrative
  (`"check docs/adr/ vs check docs/ compared by hand"`), not a resolvable reference.
- `Regression-test`: the specific test name/path proving the fix — **required**, not optional. A
  bug record with no regression test is exactly a claim nobody verified, which is the failure mode
  this whole field exists to make impossible to state by accident.
- `Realized-by`: reused, generic — points at the actual fix commit/function, same as any other
  type's evidence field.

## Reversibility

Fully additive: one config entry, one template, one new directory.

## Consequences

- A `bug` record with `Status: Fixed` but a blank/placeholder `Regression-test` is a
  `field.quality` finding already, for free — no new rule needed, since required-field quality
  checking is already generic.
- This ADR's own `Realized-by` doubles as the first real `bug` record's evidence, and
  `docs/bugs/0001-*.md` names this exact defect, required-field-complete, as the first real
  instance of the type.

## References

- ADR-34 — the `milestone` type this is a sibling of, and the reasoning this ADR follows.
- `rust/crates/urzua-cli/src/main.rs` — `scope_to_requested_paths`, the fix.
- `rust/crates/urzua-cli/tests/check_integration.rs` —
  `check_scopes_to_the_requested_path_not_the_whole_corpus`, the regression test.
