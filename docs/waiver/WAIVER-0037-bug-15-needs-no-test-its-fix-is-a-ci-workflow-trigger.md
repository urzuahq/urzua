---
Stable-Id: 01M6NKGQJ3GRS662PB7WP33300
Rule: status.delivery-agreement
Scope: docs/bugs/BUG-15-ci-yml-s-changeset-gate-evaluates-a-stale-pre-label-event-payload-since-labeled-isn-t-a-trigger-type.md
Reason: "Fixed by declaring `types: [opened, synchronize, reopened, labeled, unlabeled]` on the `pull_request` trigger of `.github/workflows/ci.yml`. The behaviour depends on GitHub delivering a `labeled` event to a running workflow, which no test in this repository can produce, so there is nothing to exercise."
Category: docs-or-workflow-only
---
# WAIVER-0037 — BUG-15 needs no test: its fix is a CI workflow trigger

## What this covers

`status.delivery-agreement` reports `BUG-15`: its `Status` is `Fixed`, but no `Realized-by` evidence computes to a delivered tier. Fixed by declaring `types: [opened, synchronize, reopened, labeled, unlabeled]` on the `pull_request` trigger of `.github/workflows/ci.yml`. The behaviour depends on GitHub delivering a `labeled` event to a running workflow, which no test in this repository can produce, so there is nothing to exercise.

The waiver has no `Expires`: the cause is a permanent fact about the record, and the `permanent` declaration in `.urzua/config.yaml` allows it under `docs-or-workflow-only`. It is reported by `waiver.matches-nothing` if the record gains evidence and the finding stops.

## References

- `docs/bugs/BUG-15-ci-yml-s-changeset-gate-evaluates-a-stale-pre-label-event-payload-since-labeled-isn-t-a-trigger-type.md` — the record whose status this waives.
- `ADR-84` — the rule; `ADR-86` — permanent waivers under a declared category; `MILE-193`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed with the `ADR-84` build (`status.delivery-agreement` on bugs). **Why:** the record is `Fixed` with a reason no test can cover, stated in `Reason`. | **substantive** |
