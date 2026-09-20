---
Stable-Id: 01M2WG8327QXG0VRRR57FRYHT5
Status: Fixed
Found-in: "Round 7 of the 0.4.0 review"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::a_staged_deletion_is_not_reported_as_owned_by_no_type"
---
# 71 — type.record-outside-declared-dir reports an unreadable or deleted record as owned by no type

## What was wrong

`BUG-62` added `type.record-outside-declared-dir` so a record-shaped file below a declared `dir` but
not directly in it is reported rather than silently dropped. Ownership is inferred from the record
list, and `load_records` skips a file it cannot read (`Err(_) => continue`).

Discovery unions `ls-files` with the staged diff, so a staged deletion is still a candidate while no
longer being on disk. After `git rm docs/adr/ADR-2-y.md`:

```text
docs/adr/ADR-2-y.md | sits below a declared record type's dir but not directly in it,
                      so no type owns it and no rule examines it
```

The message is false in both halves. The file is directly in the declared directory -- so the type
owns it by location, which is what `RFC-35` defines ownership to mean -- and it no longer exists, so
there is nothing there to own. The same false positive arises for any unreadable or
non-UTF-8 `.md` directly inside a declared directory.

The rule should decide ownership from the path -- whether its parent is a declared `dir` -- which is
the condition `RFC-35` actually defines. Unreadable is a different problem and deserves its own
finding, not this one's message.

## Why nothing caught it

`claimed` means *records that were successfully read*. It was used to mean *owned by a type*. Those
agree for every file that parses, which is every file in the fixture and every file in this
repository's corpus.

The rule's test plants an unowned file in a subdirectory and asserts it is reported. Nothing plants
an owned file the loader cannot read.

## References

- `BUG-62`, whose fix introduced this, and `RFC-35`, which defines what a `dir` owns.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
