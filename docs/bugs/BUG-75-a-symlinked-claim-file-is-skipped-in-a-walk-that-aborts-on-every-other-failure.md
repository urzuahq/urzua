---
Stable-Id: 01M2YN68KSMPKWF5RX6XGK3RQE
Status: Fixed
Found-in: "Round 8 of the 0.4.0 review, reproduced against a scratch repository"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::a_symlinked_claim_file_is_read_not_skipped"
---
# 75 — A symlinked claim file is skipped in a walk that aborts on every other failure

## What was wrong

`BUG-69` replaced `metadata` with `symlink_metadata` so the claim walk could not follow a link out of
its declared prefix. `meta.is_file()` on a symlink's *own* metadata is always false, so a symlinked
`.md` inside `claim_paths` matches neither arm and is dropped.

Silently. The same function's comment, four lines above, reads:

> Nothing here is skipped on error. A claim the rule could not read is a claim it did not check, and
> continuing past it reports agreement over an incomplete corpus.

Every other failure in the walk aborts the run. This one does not, because it is not an error -- the
entry simply matches no branch.

Reproduced: `store/0001-f.md` says `Fixes BUG-1.`, `changes/0001-f.md` symlinks to it,
`claim_paths: ["changes"]` at `error`, and `BUG-1` is `Open`. Output: `status: ok`,
`claim.status-agreement` `records_examined: 0, status: ran`, **exit 0**. The contradicting claim is
never opened. The code this replaced read it and exited 1.

## Why nothing caught it

The fix was verified against the hazard it was written for -- a directory symlink producing an
unbounded walk -- and the test plants exactly that. A symlinked *file* is the other half of the same
change and was not considered.

The regression is narrower than the rule it broke: the walk still finds every real file, so nothing
in the report looks wrong. `records_examined: 0` was the signal, and it is only a signal to someone
reading for it.

## References

- `BUG-69`, whose fix introduced this.
- `ADR-55`, on a check reporting success without looking.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
