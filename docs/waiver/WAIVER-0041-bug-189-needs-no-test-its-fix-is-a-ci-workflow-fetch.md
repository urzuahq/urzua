---
Stable-Id: 01M6NKGVPM8N8JQX9FJYE3NMQT
Rule: status.delivery-agreement
Scope: docs/bugs/BUG-189-the-changeset-required-job-fails-with-no-merge-base-on-a-pr-branch-containing-a-merge-commit.md
Reason: "Fixed in `.github/workflows/ci.yml` by dropping `--depth=1` from the two base-branch fetches of the `changeset required` job. No test runs the workflow. The record documents a reproduction on a throwaway repository holding a merge commit of main; that reproduction is not part of the suite."
Category: docs-or-workflow-only
---
# WAIVER-0041 — BUG-189 needs no test: its fix is a CI workflow fetch

## What this covers

`status.delivery-agreement` reports `BUG-189`: its `Status` is `Fixed`, but no `Realized-by` evidence computes to a delivered tier. Fixed in `.github/workflows/ci.yml` by dropping `--depth=1` from the two base-branch fetches of the `changeset required` job. No test runs the workflow. The record documents a reproduction on a throwaway repository holding a merge commit of main; that reproduction is not part of the suite.

The waiver has no `Expires`: the cause is a permanent fact about the record, and the `permanent` declaration in `.urzua/config.yaml` allows it under `docs-or-workflow-only`. It is reported by `waiver.matches-nothing` if the record gains evidence and the finding stops.

## References

- `docs/bugs/BUG-189-the-changeset-required-job-fails-with-no-merge-base-on-a-pr-branch-containing-a-merge-commit.md` — the record whose status this waives.
- `ADR-84` — the rule; `ADR-86` — permanent waivers under a declared category; `MILE-193`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed with the `ADR-84` build (`status.delivery-agreement` on bugs). **Why:** the record is `Fixed` with a reason no test can cover, stated in `Reason`. | **substantive** |
