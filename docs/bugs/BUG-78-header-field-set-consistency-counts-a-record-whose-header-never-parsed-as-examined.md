---
Stable-Id: 01M2YN6A58D8YEKHM7YWP8A08C
Status: Fixed
Found-in: "Round 8 of the 0.4.0 review"
Realized-by: test:rust/crates/urzua-core/src/rules/header.rs#header_field_set_consistency_does_not_count_an_unparsed_header@sha256:7fdcc2b41f2a0064367235acfc7eeff4e3e7f4f38927c5391771195ba9b72014
---
# 78 — header.field-set-consistency counts a record whose header never parsed as examined

## What was wrong

`header_field_set_consistency` increments `records_examined` before reading
`record.header.fields`, which is empty when the frontmatter failed to parse. The rule therefore
reports having examined a record whose fields it never saw.

Reproduced: one ADR with deliberately broken YAML frontmatter, `header.field-set-consistency: error`
declared alone. Output: `status: ok`, `records_examined: 1, status: ran`, zero findings, exit 0.

`header_layout_consistency` handles the same case correctly, skipping before the increment.

The cost is larger than one rule's count. `ADR-55` decided that `records_examined` is the signal
distinguishing a rule that looked from one that did not, and `MILE-106` will read it. A rule that
inflates it makes that signal unreliable at the moment it starts being trusted.

## Why nothing caught it

The rule's tests supply well-formed headers, so the unparsed-header branch is never reached. Its
sibling's correct handling was not used as a model because nothing compared the two.

No test asserts that `records_examined` counts only records the rule actually inspected -- the
assertion `ADR-55` now requires.

## References

- `ADR-55` and `MILE-106`, which depend on this count being honest.
- `header_layout_consistency`, which is the correct shape.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
