---
Stable-Id: 01M7MC01FXP1FHDRNK89RMP6PC
Status: Fixed
Found-in: "review of the pull request for MILE-322 on 2026-11-17: a type declaring statuses.values [Done, Todo] with initial [Todo] got Status: Done from urzua new"
Realized-by: test:rust/crates/urzua-core/src/new_record.rs#the_status_starts_at_the_declared_initial_state@sha256:10e3b0f359d6565d8bce8c6293c0f989f333e119b767b7ee6e020ddf3eb400ee
---
# 199 — urzua new starts a record at the first status value instead of the declared initial status

## What was wrong

`urzua new` set the status field to the first entry of the type's `statuses.values`, ignoring `statuses.initial`. A type declaring `values: [Done, Todo]` with `initial: [Todo]` got a new record at `Status: Done`, a state its own lifecycle says a record cannot start in, and `status.transition-allowed` had nothing to compare against because the record was new.

## Why nothing caught it

`new_record.rs` predates `statuses.initial` (`MILE-144`): when `initial` was added, the scaffold kept reading the first value, and every test fixture listed its initial state first, so the two readings always agreed.

## References

- Regression test: `the_status_starts_at_the_declared_initial_state` in `rust/crates/urzua-core/src/new_record.rs`, with one type whose first value is not its initial state and one declaring no `initial`.
- Fixed on the branch of `MILE-322`'s pull request: `scaffold_record_reporting` takes `statuses.initial` first and falls back to the first value only when no `initial` is declared.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Open`. **Why:** found in review of `MILE-322`'s pull request and fixed on its branch; the status is left for `/done`. | **structural** |
> | 2026-11-17 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 0 file-level, 1 scoped. Evidence: rust/crates/urzua-core/src/new_record.rs#the_status_starts_at_the_declared_initial_state@sha256:10e3b0f359d6 Reason: the regression test asserts new starts at statuses.initial and falls back to the first value only without initial **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-17 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `Realized-by computes to Verified` -- stated `Open` disagreed with the computed value `Fixed`. | **structural** |
