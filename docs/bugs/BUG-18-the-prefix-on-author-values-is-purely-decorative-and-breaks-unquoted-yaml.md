---
Stable-Id: 01M26DVVGZAATR67X7Z5P637ZW
Status: Open
Found-in: 'noticed live comparing urzua new''s auto-filled Author value (resolve_identity''s bare login, e.g. `beauwilliams`) against the hand-typed corpus convention (`''@beauwilliams''`, quoted) -- asked why the tool''s own output doesn''t match, and found the `@` serves no function anywhere in the codebase, while forcing the exact YAML-quoting workaround its own presence necessitates'
Regression-test: 'not yet written -- fix scope (drop @ going forward vs. backfill every existing record) not yet decided'
---
# 18 — the '@' prefix on Author values is purely decorative and breaks unquoted YAML

## What was wrong

Every hand-written `Author` value in this corpus is quoted (`'@beauwilliams'`), while
`resolve_identity()` (used by both `urzua new` and `fix --apply`) returns the bare login with no
`@` at all. Checked whether the `@` serves any mechanical purpose before assuming either side was
"right": grepped every crate for code that reads, strips, or validates an `@`-prefixed identity --
nothing. The prefix is purely decorative, copied forward by hand from record to record.

Worse, it's actively YAML-hostile. Confirmed directly (not just from the spec): a plain scalar
starting with `@` is invalid, unquoted YAML --

```
$ python3 -c "import yaml; yaml.safe_load('Author: @beauwilliams')"
ERROR: found character '@' that cannot start any token
```

`@` is a reserved YAML indicator character, the same class of problem `BUG-12` found for a
backtick-starting `Subject` value. Every hand-typed `'@name'` Author value only needs quoting
*because* the `@` is there -- remove it, and the value is a perfectly ordinary plain scalar,
matching what the tool's own auto-fill already produces unquoted, correctly, today.

## Why nothing caught it

Nothing about a correctly-quoted `'@beauwilliams'` value ever failed to parse -- the quoting was
always applied by hand alongside the `@`, so the two together never actually broke anything in
practice. The gap only surfaces by asking "does this prefix do anything," which nobody had reason to
ask until comparing the tool's own auto-filled output against the hand-typed convention side by side
and finding they disagreed.

## References

- `rust/crates/urzua-io/src/lib.rs` -- `resolve_identity`, whose bare-login output is already
  correct and needs no change.
- BUG-12 -- the same defect class (a reserved YAML indicator character forcing avoidable quoting),
  found for `Subject` instead of `Author`.
- Every `Author: '@...'` line across this corpus -- the convention this bug questions.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-10 | Initial bug record, `Status: Open`. Not yet fixed -- whether to drop `@` going forward only (existing records untouched, both forms remain valid once quoted) or backfill every existing record to match is not yet decided. | **structural** |
