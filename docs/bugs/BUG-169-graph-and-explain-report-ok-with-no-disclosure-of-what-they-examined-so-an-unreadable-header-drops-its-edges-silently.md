---
Stable-Id: 01M6D2KHY3CT24XMZZZJ5Q7YY9
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 29"
Realized-by: test:rust/crates/urzua-core/src/graph.rs#the_read_population_counts_an_unreadable_header_and_names_it_observed_failing@sha256:bb5853ba35f86f6cd80f35c8c85830ddcf280c0943a39a78325f1a263756e57b, test:rust/crates/urzua-cli/tests/check_integration.rs#graph_and_explain_report_a_population_and_name_an_unreadable_record_observed_failing@sha256:8d78819250a4b85ba75632a954072ff3d96a3e088b8411123b19ed5bc6726930, test:rust/crates/urzua-cli/tests/check_integration.rs#graph_and_explain_on_an_empty_corpus_report_a_zero_population_with_no_notice@sha256:63eaab6932bb16d5c8b6d5f5993ed382cb325614e0af7f8807e85bde7c887027
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

## Decision (2026-11-05)

The maintainer decided: `urzua graph` and `urzua explain` report the same population block `check` does (eligible, examined, unreadable), plus a warning notice listing the records left out. Status stays `ok`. Fixed in `graph.rs` (`read_population`), serialized as `population` on `GraphReport` and `ExplainReport`; a `header_shape: none` type is `out_of_scope`, as for the rule engine. Not built, because the decision does not ask for it: a disclosure that `explain` of a path no record cites is an exact-match miss (`SPEC-13` documents the exact match). `SPEC-13` states the block.

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
> | 2026-11-04 | Triage: still reproduces on main; `Status` stays `Open`. With `Derives-from: ADR-1` changed to `Derives-from: ADR-1: bad: yaml`, `urzua graph` returns `{"status":"ok","edges":[]}`, exit 0, and `urzua explain src/nonexistent.rs` returns `ok` with `governing_records: []`, neither with a population or a notice. **Why:** the decision (a population plus notices, or `not-run`, or a partial status) is the one the record lists. | **substantive** |
> | 2026-11-05 | Fixed. **Why:** maintainer decision 2026-11-05: both commands report the population block plus a warning notice naming the unreadable records; status stays ok. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 3 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#graph_and_explain_report_a_population_and_name_an_unreadable_record_observed_failing` (lines 846-893) c2c182de4bce -> 1fa8afd36075, `rust/crates/urzua-cli/tests/check_integration.rs#graph_and_explain_on_an_empty_corpus_report_a_zero_population_with_no_notice` (lines 895-916) f101595e064c -> 715b31bde988. Reason: Read the diff of each pinned span since the old pin and the claim still holds. check_integration.rs#graph_and_explain_report_a_population_and_name_an_unreadable_record_observed_failing: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged; check_integration.rs#graph_and_explain_on_an_empty_corpus_report_a_zero_population_with_no_notice: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#graph_and_explain_report_a_population_and_name_an_unreadable_record_observed_failing` (lines 846-893) 1fa8afd36075 -> 8d78819250a4, `rust/crates/urzua-cli/tests/check_integration.rs#graph_and_explain_on_an_empty_corpus_report_a_zero_population_with_no_notice` (lines 895-916) 715b31bde988 -> 63eaab6932bb. Reason: the tests in check_integration.rs now name real tracked files; the behaviour this record relies on is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
