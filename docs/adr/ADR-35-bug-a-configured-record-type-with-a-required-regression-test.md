---
Status: Accepted
Embodiment: Verified
Realized-by: code:.urzua/config.yaml@sha256:8bc61e85bdcce1d754a31cf5b9d304cd0df564d848a7fb50cc263e82b7fe5d46, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:b668d62f2f4a538e1b48383362b11314795407225697dd62f3f560ada9e74348
Date: 2026-09-07
Accepted-on: 2026-09-07
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: ADR-34
---
# 35 — `bug`: a configured record type requiring a named regression test

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

## Amendment (2026-11-04): the `bug` type declares five statuses

*Status of this amendment: accepted 2026-11-04 by the maintainer (drafted 2026-11-04 from the drift review the maintainer approved). The Decision text above is unchanged.*

The Decision gives `Status` as `Open | Fixed | WontFix`. The `bug` type in `.urzua/config.yaml` now
declares five: `Open`, `Fixed`, `WontFix`, `Not a bug` and `Rejected`. The required fields
(`Status`, `Found-in`, `Regression-test`) and the rest of the Decision are unchanged. The declared
list in the config is the authority; this text does not enumerate it again.

## References

- ADR-34 — the `milestone` type this is a sibling of, and the reasoning this ADR follows.
- `rust/crates/urzua-cli/src/main.rs` — `scope_to_requested_paths`, the fix.
- `rust/crates/urzua-cli/tests/check_integration.rs` —
  `check_scopes_to_the_requested_path_not_the_whole_corpus`, the regression test.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | `Realized-by` locators pinned by content hash (`@sha256:`) after re-verifying each cited file against the Decision. **Why:** Re-verified against current code; maintainer decision 2026-11-04. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 784fa758e. | **structural** |
> | 2026-11-04 | Re-baselined (attested by beauwilliams): re-pinned `.urzua/config.yaml` e185c4da7b1f -> 8bc61e85bdcc, `rust/crates/urzua-cli/tests/check_integration.rs` 288a08b371a8 -> b668d62f2f4a. Reason: Re-verified: the bug type still requires Status, Found-in and Regression-test; config.yaml growth is unrelated rule options and types. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-04 | Amendment added: the `bug` type declares five statuses, not the three the Decision lists. **Why:** drift review of 2026-11-04, approved by beauwilliams; proposed, pending maintainer acceptance. The Decision is unchanged. | **substantive** |
> | 2026-11-04 | Amendment of 2026-11-04 accepted. **Why:** Maintainer decision, 2026-11-04; the Decision text is unchanged. | **substantive** |
