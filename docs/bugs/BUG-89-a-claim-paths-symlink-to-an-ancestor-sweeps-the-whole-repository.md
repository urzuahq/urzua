---
Stable-Id: 01M2YR3HK3BB9EWBCBB8H17GVW
Status: Fixed
Found-in: "Round 10 of the 0.4.0 review, reproduced against a scratch repository"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::a_symlink_cycle_inside_a_claim_path_terminates_and_reads_each_claim_once"
---
# 89 — A claim_paths symlink to an ancestor sweeps the whole repository

## What was wrong

`BUG-85` let the claim walk follow a directory symlink whose target stays inside the repository,
bounded by a visited set. Containment was tested against the *repository*, and `BUG-69`'s actual
concern was the *declared prefix*.

A link to an ancestor satisfies repository containment and sweeps the whole tree. Reproduced with
`claim_paths: ["claims"]` and `ln -s ../ claims/up`, over a repository holding an undeclared file
containing `This changeset closes ADR-1.`:

```text
claims/up/secretdocs/random.md:1 -- claims to close ADR-1, but ADR-1 has Status Accepted
```

Blocking, on a file the configuration never declared as a claim. The finding's path also runs
*through* the symlink, so scope filtering and waiver `Scope` matching key on a location the file does
not have.

A link to a sibling directory is legitimate -- it is the author saying claims also live there, which
is exactly what a symlinked root already does (`BUG-85`). A link to an ancestor is not, and is now
refused by name.

## Why nothing caught it

`BUG-85` was written to reconcile two earlier fixes that disagreed, and was tested against the case
they disagreed about: a link to a sibling. The ancestor case is what `BUG-69` originally reported,
and reintroducing it was not checked for.

Each of the three fixes was verified against its own scenario. None was verified against the
scenarios of the other two.

## References

- `BUG-69`, `BUG-80`, `BUG-85` -- the three previous positions of this walk.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
