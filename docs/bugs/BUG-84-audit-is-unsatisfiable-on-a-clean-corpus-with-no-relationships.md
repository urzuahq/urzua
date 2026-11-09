---
Stable-Id: 01M2YPPRWS5FHNE404A3GN7NCK
Status: Fixed
Found-in: "Round 9 of the 0.4.0 review, reproduced against a scratch repository"
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#audit_is_ok_on_a_clean_corpus_with_no_relationships@sha256:fa68f05ca625f0a380712a7e8e326ca60e0ab5116946e405e25843c5f15686e1
---
# 84 — audit is unsatisfiable on a clean corpus with no relationships

## What was wrong

`BUG-77` made a run report `not-run` unless some rule examined a record. Applied to `audit`, whose
two rules are `pointer.resolution` and `relation.supersession-reciprocity`, that makes a corpus with
no pointers and no supersessions unsatisfiable.

Reproduced with the configuration `init` generates, over two valid records:

```json
{"status": "not-run", "files_examined": 2,
 "rules_executed": [{"pointer.resolution", 0, ran}, {"relation.supersession-reciprocity", 0, ran}]}
```

Exit 2. Both rules ran, correctly, and had nothing to judge -- which is a clean result, not an
unestablished one. No user action short of inventing relationships between records makes it exit 0,
and `init` writes this config, so every fresh adopter's first `audit` fails CI on a clean corpus.

The guard now requires that a record-scoped rule **ran**, not that it examined a non-zero count.

## What stays unsolved

A rule that ran and examined nothing is *sometimes* the `BUG-61` failure -- a rule that could not
address any record because the corpus does not have the shape it assumes. Telling that apart from
"there was nothing to examine" needs each rule to declare its eligible population, so the signal
becomes `eligible > 0 && examined == 0` rather than `examined == 0`.

That is `MILE-106`'s subject and its stated hard part. This fix returns the strict check to that
milestone rather than shipping a proxy that was wrong in both directions: too permissive in `BUG-83`,
too strict here.

## Why nothing caught it

Every test of the guard supplies a corpus where something is wrong, because the guard was written to
catch a run establishing nothing. A clean corpus that legitimately gives a rule nothing to do was not
among them.

`records_examined > 0` was chosen because `ADR-55` makes that number load-bearing. It is the right
number for "did this rule look" and the wrong one for "did this run establish anything", and the two
were not separated.

## References

- `BUG-77`, whose fix introduced this, and `BUG-83`, the same guard failing the other way.
- `MILE-106`, which owns the discriminator.
- `ADR-55`, and `BUG-61` for the failure the strict form was meant to catch.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#audit_is_ok_on_a_clean_corpus_with_no_relationships` (lines 2294-2312) 6ea12ee652a6 -> fa68f05ca625. Reason: Read the diff of each pinned span since the old pin and the claim still holds. check_integration.rs#audit_is_ok_on_a_clean_corpus_with_no_relationships: the output assertions now read the JSON field with json_field instead of substring-matching the printed text, with the same expected values. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
