---
Stable-Id: 01M7W8EZ84B0CNEPGQY72HFJQZ
Status: Open
Found-in: review of pull request #616 (section.decided-body-changed), 2026-11-20: a prefix-lines record's rewrite went unexamined; a check of the decider gate on a prefix-lines fixture reproduced the same read.
Realized-by:
---
# 213 — A record's merge-base copy is read with header_shape alone, so a prefix-lines type's base header reads as unreadable

## What was wrong

A record type that declares `fields: {from: prefix-lines}` has no `---` front matter, but three places that read the merge base's copy of a record parse it with `header_shape` alone, which defaults to YAML front matter. The base header then has no region and reads as unreadable, so the check that needed it silently does less:

- `rust/crates/urzua-core/src/rules/decided_body.rs`, `judge` (`section.decided-body-changed`): the base status was unreadable, so a rewrite of an Accepted prefix-lines record was counted unexamined and not reported. Fixed in the pull request that found it, by reading the copy through `record_in_text`.
- `rust/crates/urzua-cli/src/diff_scope.rs`, `committed_statuses` (the decider gate, `diff.decider-required`): reproduced on a prefix-lines `adr` type with a gate on `Accepted`. A move from Proposed to Accepted is reported as "the merge base's copy could not be read, so it is treated as a move", not as a move from Proposed.
- `rust/crates/urzua-core/src/rules/revision_log.rs`, `judge_version_bump` (`revision-log.version-bump-required`): reads the base copy with `parse_with_shape_and_prefix` and compares the declared version field, so on a prefix-lines type the base value reads as absent. Read from the code; not reproduced.

Other sites that read a base or older copy of a record were not audited.

## Why nothing caught it

Each site rebuilt a `Record` from text itself instead of through one function that reads a record as the loader does (`Record::parse_with_model` with the type's declared model). `record_in_text` in `claim_expiry.rs` is that function but is private to the rules module. No test used a prefix-lines type for a base-copy read.

## References

`record_in_text` (`rust/crates/urzua-core/src/rules/claim_expiry.rs`) and `resolved_model` (`rust/crates/urzua-cli/src/discovery.rs`) read a record as the loader does; the shared fix is to route every base-copy read through one such function and add a prefix-lines case for each site.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
