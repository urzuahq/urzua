---
Stable-Id: 01M2YPPS9KYHX2BTKWE6PGAJQY
Status: Fixed
Found-in: "Round 9 of the 0.4.0 review, reproduced against a scratch repository"
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#a_symlink_cycle_inside_a_claim_path_terminates_and_reads_each_claim_once@sha256:ea2178098d2da4779e8fc9820e50aac8c422e0d939cb7f2203be21ccb4e38c5b
---
# 85 — A directory symlink is rejected inside claim_paths but accepted as its root

## What was wrong

`BUG-69` stopped the claim walk following a directory symlink, because a link to an ancestor gave an
unbounded walk that left the declared prefix. `BUG-80` then resolved the *root* symlink and required
only that it stay inside the repository.

The two ended up disagreeing about the same symlink. A directory link nested inside `claim_paths`
aborted the whole run with exit 2, while the identical link used as the `claim_paths` root was
accepted:

```json
{"status": "not-run",
 "error": "claim_paths: .../claims/real/linked is a symlink to a directory; the walk does not follow it"}
```

A repository organising its claims behind an intra-repo directory symlink lost `check` entirely.

Both paths now apply the same test -- resolve the target, require it under the repository root -- and
a visited set bounds the walk, which is what actually prevents the cycle `BUG-69` reported. Rejecting
links outright was stricter than the reason given for it.

## Why nothing caught it

`BUG-69` and `BUG-80` were fixed in consecutive rounds, the second correcting the first, and each was
tested against its own case: a link to an ancestor, and a link used as the root. Nothing tested a
link that is neither -- nested and legitimate -- which is where the two rules met and contradicted
each other.

The abort was also the wrong instrument: the stated hazard was an unbounded walk, and a visited set
addresses that directly without refusing a construct the root already accepts.

## References

- `BUG-69` and `BUG-80`, the two fixes this reconciles.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
