---
Stable-Id: 01M6NKGTQSPMP5TK9CHH3249FT
Rule: status.delivery-agreement
Scope: docs/bugs/BUG-107-comments-narrate-bug-ticket-history-and-use-temporal-language-against-the-comment-convention.md
Reason: "Fixed by rewording temporal comments in `check_integration.rs` and `rules.rs` to state the invariant directly. A comment has no behaviour to regress, and the comment convention is enforced by review, not by a rule the engine runs."
Category: docs-or-workflow-only
---
# WAIVER-0040 — BUG-107 needs no test: its fix rewords comments

## What this covers

`status.delivery-agreement` reports `BUG-107`: its `Status` is `Fixed`, but no `Realized-by` evidence computes to a delivered tier. Fixed by rewording temporal comments in `check_integration.rs` and `rules.rs` to state the invariant directly. A comment has no behaviour to regress, and the comment convention is enforced by review, not by a rule the engine runs.

The waiver has no `Expires`: the cause is a permanent fact about the record, and the `permanent` declaration in `.urzua/config.yaml` allows it under `docs-or-workflow-only`. It is reported by `waiver.matches-nothing` if the record gains evidence and the finding stops.

## References

- `docs/bugs/BUG-107-comments-narrate-bug-ticket-history-and-use-temporal-language-against-the-comment-convention.md` — the record whose status this waives.
- `ADR-84` — the rule; `ADR-86` — permanent waivers under a declared category; `MILE-193`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed with the `ADR-84` build (`status.delivery-agreement` on bugs). **Why:** the record is `Fixed` with a reason no test can cover, stated in `Reason`. | **substantive** |
