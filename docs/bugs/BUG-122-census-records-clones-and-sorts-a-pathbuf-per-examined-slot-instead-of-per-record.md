---
Stable-Id: 01M3630KTMZ3JT51978GPKVW5X
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Regression-test: "none -- performance-only, no behavior change; full suite and real-corpus finding count unchanged. No benchmark added; the fix removes a bulk final sort in favor of incremental BTreeSet inserts, which needs no corpus larger than this repo's own to be correct by construction."
---
# 122 — census_records clones and sorts a PathBuf per examined slot instead of per record

## What was wrong

`report::census_records` pushes a cloned `PathBuf` into `examined_records` once per *examined
candidate*, then sorts and dedups the whole vector at the end. For a `Field`-unit rule (the majority
of rules in `rules.rs`), a candidate is a `(record, field)` slot, not a record -- so a record with `F`
declared fields contributes up to `F` clones of the same path, immediately followed by a sort over all
of them to recover what is, at most, the distinct set of examined record paths.

For `N` records and `F` declared fields per type, this is up to `O(N*F)` heap-allocated `PathBuf`
clones and an `O(N*F log(N*F))` sort per rule invocation, repeated across the 8+ `Field`-unit rules on
every `check` run, to recover a set bounded by `N`.

## Why nothing caught it

Measured on this repository's own corpus (309 records), the largest slot count is `field.quality`'s
1023 (`BUG-40`) -- a sort of ~1000 short strings costs microseconds, well under anything a benchmark or
a human would notice. The inefficiency is real but has never been on a scale where it mattered here.

## What this needs before a fix

Not a design decision -- a mechanical change (e.g. dedup with a `HashSet<PathBuf>` as candidates are
examined, only materializing and sorting the final set once) -- but not worth doing without a corpus
large enough to measure the difference, per this project's own precedent (`BUG-113`'s record: "this
runs once per rule per invocation, not a hot-path cost worth trading away"). Filed to track rather
than fixed now, since fixing it today would be optimizing against a benchmark that doesn't exist.

## Fix

`examined_records` is now a `BTreeSet<PathBuf>`, inserted into as candidates are examined rather than
pushed to a `Vec` and sorted/deduped once at the end. The final `sort`+`dedup` pass over up to `N*F`
elements is gone; the set's own tree structure keeps it sorted and deduped incrementally, and
`into_iter().collect()` at the end is the only remaining pass, over the deduped set (bounded by the
record count) rather than the full candidate list. No behavior change.

## References

- `BUG-40` -- established that a `Field`-unit population is routinely far larger than the record
  count, the same shape this bug's cost scales with.
- `BUG-113` -- the precedent for deferring a real-but-currently-negligible-cost fix.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed, not fixed. **Why:** found by a full-release code review; real but measured as negligible on this repository's own corpus, so left open to track rather than fixed under review pressure with no benchmark to verify against. | **substantive** |
> | 2026-09-23 | Fixed. The user asked for this class of filed-but-deferred defect to be closed before 0.4.0 regardless of benchmark pressure. `examined_records` is now a `BTreeSet`, dedupe done incrementally rather than in a final sort pass. No behavior change: full suite and real-corpus finding count unchanged. | **substantive** |
