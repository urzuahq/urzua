---
Stable-Id: 01M36BA15XBAZKPVGZZE5SKTME
Status: Done
Phase: '1'
Track: schema-governance
Implements: RFC-45
Blocked-on: —
---
# 111 — Disclose every non-absent exclusion; no rule depends on another to do it

## What

`RFC-45`/`ADR-63`'s implementation, in two parts:

1. **`Population` gains a disclosed `unreadable` count**, alongside `eligible`/`examined`/
   `out_of_scope`, computed structurally by `census`/`census_records` from `Outcome::Unreadable` —
   never hand-maintained per rule. Closes `BUG-125` for every rule using the shared machinery, in one
   change.
2. **A new, dedicated, opt-in rule, `relation.target-status-undeclared`**: for every resolved
   pointer/narrative reference, does the target's type declare `Status` at all (`ADR-60`'s gate). One
   rule, one place this is diagnosed, matching `header.field-case-mismatch`'s own precedent
   (`RFC-40`/`ADR-58`) rather than `pointer_target_status`/`narrative_field_stale` each inventing
   their own check.

Part 1's original design (a new `Outcome::Undeclared` variant threaded into `Population`) did not
survive implementation — see `ADR-63`'s amendment — and was corrected to the dedicated-rule shape
above before shipping, not silently rewritten.

## Why

Three consecutive review rounds flagged the same false positive: a rule narrowing its population
because `ADR-60`'s gate excluded a record reads as a silent regression. Investigating whether the same
silent-folding applied to `Unreadable` surfaced `BUG-125`, a live, reproduced instance of `ADR-55`'s
core defect class: a config enabling only `field.quality` reported a clean run over a record with a
completely unparseable header, because that rule's silence depended on `header.required-fields` being
separately enabled — a pairing `ADR-53` never guarantees. The user named the principle directly: no
rule may depend on another rule being enabled to cover a gap in its own disclosure.

## Done means

1. ✅ `Population::unreadable()` discloses parse failures on their own; verified with a
   planted-violation test (`field_quality_discloses_unreadable_on_its_own_observed_failing`) against
   `BUG-125`'s exact scratch reproduction.
2. ✅ `relation.target-status-undeclared` reports a resolved reference whose target's type never
   declares `Status`; verified with planted-violation tests for both the flagging case and the silent
   case.
3. ✅ `SPEC-2` and this repository's own `.urzua/config.yaml` updated in the same change; a real-corpus
   `check` diff confirms no unintended finding-count change (this repository has no unreadable headers
   or undeclared-target references today).
4. ✅ The `AGENTS.md` "A narrower rule population is usually policy, not a regression" section removed
   in the same change — a structural fix supersedes the prose stopgap rather than sitting beside it.

## References

- `RFC-45`, `ADR-63` — the design this milestone implements, including the same-day amendment.
- `BUG-125` — the live defect this milestone closes.
- `ADR-53`, `ADR-55`, `ADR-60` — the principles this milestone makes structural.
- `RFC-40`/`ADR-58` — the `header.field-case-mismatch` precedent Part 1's final shape follows.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed, `Status: Planned`. **Why:** `RFC-45`'s central question needed deciding before implementation; filed the same round it was resolved so the work is scheduled immediately rather than left open. | **substantive** |
> | 2026-09-23 | Rescoped alongside `RFC-45`'s broadening: `BUG-125` (a live, reproduced defect, not the original narrow three-rule ambiguity alone) is now this milestone's primary driver. `Blocked-on` cleared since the design question was decided in the same session it was raised. | **substantive** |
> | 2026-09-23 | `Status: Planned` → `Done`. **Why:** both parts implemented and verified: full test suite, clippy, `make ci`, and a real-corpus `check` diff reporting the same 70 findings before and after. Part 1 shipped as a dedicated rule rather than the originally-decided `Outcome` variant, corrected mid-implementation per `ADR-63`'s amendment. | **substantive** |
