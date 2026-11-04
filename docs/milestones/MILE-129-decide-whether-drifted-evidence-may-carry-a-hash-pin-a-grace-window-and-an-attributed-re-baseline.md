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

## Done means

The combination is settled: `ADR-67` (with its amendment of 2026-11-04) is Accepted by the maintainer, the opt-in `@sha256:` evidence pins, `drift_grace_days`, `claim_max_age_days` and the attributed re-baseline it decides are built (`evidence_pins_integration.rs`, re-baselining from `MILE-181`), and the options are exercised in urzua's own `.urzua/config.yaml` and the pins on its own records; acceptance is the maintainer's, so `Status` and `Realized-by` move only after it.

## References

- `ADR-32` -- drift via git history, not a stored hash.
- `RFC-2` -- review-before-apply refresh; grace window and attribution.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `ADR-32` (no stored hash, unconditional drift) and `RFC-2` (hash, grace window, attributed refresh) conflict and the tension needs resolving before drift can be promoted past `warn`. | **substantive** |
> | 2026-11-04 | Decision recorded: all three, each opt-in and off by default: content-hash pins on evidence, a configurable grace window in days, and a warning when a claim exceeds a configured age. `Status` stays `Planned`. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | Built: opt-in `@sha256:` evidence pins, `drift_grace_days` and `claim_max_age_days` on `embodiment.consistency` (`SPEC-2`, `SPEC-3`, `SPEC-8`); the attributed re-baseline is left out and the pin is re-set by hand. `Status` stays `Planned`. **Why:** the 2026-11-04 decision names three opt-in pieces and none of them is a re-baseline command. | **substantive** |
> | 2026-11-04 | Fixed a defect in the built grace window (`BUG-179`): inside `drift_grace_days` a record now agrees with either its tier or a stated `Drift detected`, and `urzua fix` no longer flips a stated `Drift detected` while the drift is only inside the window. `Status` stays `Planned`. **Why:** the window computed the tier value, so it contradicted records that honestly state `Drift detected`. | **substantive** |
> | 2026-11-04 | Evidence so far, no `Realized-by` yet. **Why:** pins, the grace window and the claim age are built as opt-in options (`embodiment.rs`, `evidence_pins_integration.rs`, merged in #174), drafted in `ADR-67`, which is still Proposed; the attributed re-baseline is not decided or built. A decision milestone is `Done` once the decision is accepted, which the maintainer owns. `Status` stays Planned. | **substantive** |
> | 2026-11-04 | Checked against its Done condition after re-baselining shipped (#238, `MILE-181`): the three decided opt-ins are built and match `ADR-67`, and so is the attributed re-baseline the earlier row left out. `Status` stays `Planned`, with no `Realized-by`. **Why:** a decision milestone is `Done` once the decision is accepted, and `ADR-67` is still `Proposed`; acceptance is the maintainer's. What remains: accepting `ADR-67` (with its amendment of 2026-11-04), then setting `Realized-by` and `Done` here. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
