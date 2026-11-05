---
Stable-Id: 01M6MP8T0T4AAJDC15G7PBQ49Q
Status: Planned
Phase: '0'
Track: governance-process
Implements: ADR-79
Blocked-on: —
---
# 190 — Fold audit into check: check --rules, rules_selected in the report, and audit as a deprecated alias of check

## What

`urzua check --rules <id>[,<id>...]` restricts a run to the named declared rules and the report gains a top-level `rules_selected`; an excluded rule is omitted from `rules_executed`, and a requested rule the config does not declare is an invocation error with exit 2. `urzua audit` becomes, now, a deprecated alias of `check` over the configured rules with no rule set of its own, emitting a `Notice` that names the replacement, and its report equals `check`'s. `BUG-99` closes with a test.

## Why

`ADR-79` (amended while Proposed) folds `audit` into `check`: `audit` owns a hardcoded rule set, which `ADR-53` made wrong, and that ownership is the root of `BUG-42`, `BUG-84` and `BUG-99`. `ADR-79` says the removal comes the release after; this milestone builds the fold and the deprecation, not the removal.

## Done means

- `check --rules a,b` runs only the named declared rules, omits the rest from `rules_executed` (never as `not-enabled`), names the filter in `rules_selected`, and computes `status` and the exit code over the selected set; an undeclared id is a could-not-run report (exit 2) naming it; `rules_selected` is absent or null on an unfiltered run as `SPEC-2` states.
- `urzua audit` runs through `check`'s engine over the configured rules with no selection of its own, its report equals `check`'s, and it emits a `Notice` naming `urzua check` and never moves the exit code.
- A test runs `audit` on the config `urzua init` writes for a prefixless corpus and shows it matches `check` (the `BUG-99` regression), and `BUG-99` is closed by it.
- `SPEC-1`, `SPEC-2`, `SPEC-8` and `SPEC-11` describe the new flag, the report field and the alias; this repository's own `urzua audit` and `urzua check --rules` are exercised against its corpus.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-79` Accepted 2026-11-05 folds `audit` into `check`; the milestone builds the part that ships now (selection, the alias and the deprecation Notice), with removal the release after. | **substantive** |
