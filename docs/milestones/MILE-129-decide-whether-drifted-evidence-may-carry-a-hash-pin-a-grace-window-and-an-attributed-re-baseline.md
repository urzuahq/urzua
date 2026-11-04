---
Stable-Id: 01M6CXZYQQ4ZT1R16B9JVYZ90W
Status: Planned
Phase: '0'
Track: embodiment-model
---
# 129 — Decide whether drifted evidence may carry a hash pin, a grace window, and an attributed re-baseline

## What

Decide whether drifted evidence may carry a hash pin, a grace window before drift blocks, an attributed
re-baseline (logged with who and why), and a claim age (how long a stated `Embodiment` has gone
unchanged). Each is independently decidable; this milestone is where the combination is settled or
split.

## Why

`ADR-32` decided drift detection reads git history on the `Realized-by` line and stores no hash, so
drift is unconditional and there is nowhere to record "re-verified, still holds". `RFC-2` (Accepted)
proposed the opposite on every point: recorded and current hashes, a `graceExpiresAt`, and an
attributed refresh via `--by`; its own text says content-hash drift detection "is not built". The two
records pull in opposite directions and neither has been reconciled. A corpus of several hundred records
is large enough that unconditional drift would block constantly with no sanctioned way to re-baseline, so the
tension has to be resolved before `embodiment.consistency` can be promoted past `warn` on it.

## Decision (2026-11-04)

All three, each opt-in and off by default: content-hash pins on evidence, a configurable grace window in days, and a warning when a claim exceeds a configured age.

## References

- `ADR-32` -- drift via git history, not a stored hash.
- `RFC-2` -- review-before-apply refresh; grace window and attribution.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `ADR-32` (no stored hash, unconditional drift) and `RFC-2` (hash, grace window, attributed refresh) conflict and the tension needs resolving before drift can be promoted past `warn`. | **substantive** |
> | 2026-11-04 | Decision recorded: all three, each opt-in and off by default: content-hash pins on evidence, a configurable grace window in days, and a warning when a claim exceeds a configured age. `Status` stays `Planned`. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
