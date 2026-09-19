---
Stable-Id: 01M2WCN8AQY9N8P4TWZER0GC9D
Status: Open
Found-in: "Round 6 of the 0.4.0 release review"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::a_record_below_a_declared_dir_but_not_in_it_is_reported_not_dropped"
---
# 62 — A record in a subdirectory of a declared type dir is owned by no type and nothing says so

## What was wrong

`RFC-35` decided that a type's `dir` means that directory and not its subtree, and discovery now
matches on the parent directory rather than a path prefix. That is the decided behaviour and it is
correct.

What is missing is the diagnostic. A record-shaped, tracked file one level below a declared type dir
is now owned by no type, so it is dropped from the record set entirely. `check` reports a smaller
`files_examined`, `status: ok`, `blocking: false`, and says nothing at all about it.

For a configuration written against 0.3.0 this is a silent change of coverage on upgrade: records that
were governed yesterday are ungoverned today, and the report reads exactly as it did before.

A file that looks like a record, sits under a declared type directory, and belongs to no type is
something the tool knows and is not saying.

## Why nothing caught it

`RFC-35` was decided as a discovery question -- which files a `dir` selects -- and the change was
verified by asserting the new selection. Nothing asked what happens to the files the old selection
included and the new one does not.

The general shape: a narrowing was tested for what it now includes, never for what it silently
dropped.

## References

- `RFC-35`, which decided the semantics this diagnostic defends.
- `MILE-98`, the declared document model.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
