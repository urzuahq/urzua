---
Stable-Id: 01M2WCN7XW36XM92HNEQJEBE23
Status: Fixed
Found-in: "Round 6 of the 0.4.0 release review"
Realized-by: test:rust/crates/urzua-cli/src/commands/init.rs#a_detected_prefix_is_written_so_the_proposed_rules_can_read_an_identity@sha256:bad1824d5f9aa45b8cba8a1fa643d18dc75b6b02c4c130e9420bfffb6ef9f6fc, test:rust/crates/urzua-cli/src/commands/init.rs#a_corpus_with_no_prefix_is_not_offered_rules_that_cannot_fire@sha256:6ef287fc50cfbda3aa575acd67bf498224f3de1aa280abe5348f1221bec10f1e
---
# 61 — init writes no prefix so the rules it proposes examine zero records on the corpus it adopted

## What was wrong

`render_config_yaml` writes `dir`, `required_fields` and `header_shape` for each type it proposes,
and no `prefix`. Discovery therefore defaults the prefix to the uppercased type name.

But `is_record_shaped` deliberately accepts filenames that carry no prefix at all -- `0001-slug.md` --
because adopting a corpus in that shape is what adopt mode is for. For such a corpus `init` proposes
type `adr`, so the prefix defaults to `ADR`, and every rule that derives a record's identity from its
filename tries `strip_prefix("ADR-")`, gets `None`, and `continue`s **before** incrementing its
examined count.

The result is a configuration that looks healthy and checks nothing: `filename.title-consistency` and
`relation.supersession-reciprocity` report `status: ran, records_examined: 0`, and
`pointer.resolution` builds an empty index. Zero findings over a corpus none of them read.

`parse_record_filename` already returns the prefix it detected and `init` discards it.

## Why nothing caught it

Adopt mode has no test that runs `check` against the configuration `init` produced and asserts the
rules actually addressed the records. The existing tests assert on the configuration's contents, which
is why a config that is well-formed and inert passes them.

`init` already declines to propose rules whose required options it cannot supply. The same reasoning
applies to a rule whose shape assumption it cannot satisfy, and was not extended to it.

## References

- `BUG-58`, which taught `parse_record_filename` to return the prefix this fix needs to write out.
- `MILE-51`, the adopt-a-foreign-corpus validation whose corpus is exactly this shape.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs#a_detected_prefix_is_written_so_the_proposed_rules_can_read_an_identity` (lines 945-961) 72f13d1a7e48 -> bad1824d5f9a, `rust/crates/urzua-cli/src/commands/init.rs#a_corpus_with_no_prefix_is_not_offered_rules_that_cannot_fire` (lines 991-1023) ff03c4b54c4a -> 6ef287fc50cf. Reason: the pinned test span's only change is the added `exclude_files: vec![]` fixture field; the behaviour this record relies on is unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
