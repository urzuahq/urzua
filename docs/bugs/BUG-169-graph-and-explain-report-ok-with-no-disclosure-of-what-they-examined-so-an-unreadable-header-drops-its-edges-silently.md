---
Stable-Id: 01M6D2KHY3CT24XMZZZJ5Q7YY9
Status: Open
Found-in: "A /code-review v0.3.0...main pass, round 29"
Regression-test: "planned: graph and explain tests where a header is unreadable or the corpus is empty, asserting a notice or a not-run status"
---
# 169 — graph and explain report ok with no disclosure of what they examined, so an unreadable header drops its edges silently

## What was wrong

Once the repository, config and records load, `urzua graph` and `urzua explain` emit `status: ok` with
exit 0 whatever the records contributed, and carry no population. (A failure to find the repository,
load the config or discover files does return `not-run`.) Both reports already have a `notices` field,
but `graph` fills it only for identity collisions and `explain` never does, so nothing discloses an
unreadable header or an empty corpus.

Repro (yaml-frontmatter `adr` type, `Derives-from` declared a pointer field): ADR-2 with
`Derives-from: ADR-1` gives one edge. Change the line to `Derives-from: ADR-1: bad: yaml` (an unparseable
header) and `urzua graph` returns `{"status":"ok","edges":[]}`, exit 0, while `urzua check` on the same
corpus reports `unreadable: 1`. Likewise `urzua explain src/nonexistent.rs` returns
`governing_records: []` with `ok`, indistinguishable from "examined, nothing governs it"; `explain
./src/a.rs` misses a record citing `src/a.rs` (exact match is documented in SPEC-13, the silence about it
is not). On a corpus with zero records both are also `ok`.

## What the docs say

SPEC-1: a check that finds nothing must be distinguishable from one that ran on nothing. `BUG-125` fixed
that collapse for the rule engine (an unreadable header read as absent); these two commands sit outside
the rule engine and never got a population.

## Shared shape

Same family as `BUG-166`: a success status that does not depend on the inputs having reached anything.
There it was a selector that matched nothing; here it is a corpus, or a header, that contributed nothing.

## Decision needed

- Add a `population` to `GraphReport` and `ExplainReport` and use their existing `notices` for
  unreadable records and zero records, or return `not-run` when no record was readable?
- Whether `explain` of a path no record cites should disclose that it is an exact-match miss.
- Whether an unreadable header makes `graph` partial (its own status) or a notice only.

## References

- `BUG-125`, `BUG-166`, `SPEC-13`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** found by a full-release code review and reproduced with the release binary on a disposable fixture. Left `Status: Open` -- the fix needs a decision, not a quick edit. | **substantive** |
