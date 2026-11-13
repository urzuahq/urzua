---
Stable-Id: 01M2YN6ASM6ZQT4VW090K39Q4P
Status: Fixed
Found-in: "Round 8 of the 0.4.0 review"
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#two_records_claiming_one_identifier_are_reported@sha256:119a57ee4c15564fd8be66c37b4bebd15ecff035bffc1092b24f8b7e2cfe0af2
---
# 79 — Two records sharing an identifier collapse in the reference index and the verdict depends on filename order

## What was wrong

`build_normalized_index` collects records into a `HashMap` keyed by normalized identifier. Two
records that normalize to the same key silently collapse; the map keeps whichever came last in
corpus order.

The loser is invisible to every rule that resolves a reference -- `pointer.resolution`,
`pointer.target-status`, `narrative-field.stale`, `graph` -- and nothing reports the collision.

Reproduced: `docs/bugs/BUG-1-x.md` is `Open`, `docs/bugs/BUG-1-y.md` is `Fixed`, and
`changes/0001-f.md` says `Fixes BUG-1.` with `claim.status-agreement: error`. Output: `status: ok`,
`files_examined: 2`, **exit 0** -- while a record called `BUG-1` is still `Open` and the claim is
false. Renaming so the open record sorts last makes the same corpus exit 1.

**The verdict depends on filename ordering.**

Pre-existing rather than introduced by this release, and a live instance of the pattern `ADR-55`
names: the tool knows both records exist and reports on one.

## Why nothing caught it

A duplicate identifier is a corpus error nobody expected to author, and this repository has never
had one -- `urzua new` assigns numbers, so the state is unreachable through the tool. It is reachable
by hand-authoring, which `MILE-103` records is still happening, and by a merge that lands two records
claiming the same number.

Nothing compares the index's length against the number of records that produced an identifier.

## References

- `ADR-55`.
- `MILE-103`, on hand-authoring still being reached for.
- `BUG-58`, on identifier parsing.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
