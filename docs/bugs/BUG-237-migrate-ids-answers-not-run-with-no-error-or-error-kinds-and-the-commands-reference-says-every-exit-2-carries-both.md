---
Stable-Id: 01M88X42SP45ZSKAQAV0SCRB7R
Status: Open
Found-in: an independent review of pull request 649 (head cf4b377c), comparing the not-run report types with the commands reference
Realized-by:
---
# 237 — migrate ids answers not-run with no error or error_kinds, and the commands reference says every exit 2 carries both

## What was wrong

`urzua migrate ids --apply` reports `"status": "not-run"` and exits 2 when every record missing a
`Stable-Id` has no header it can write into (`MigrateIdsStatus::NotRun` in
`rust/crates/urzua-cli/src/commands/migrate.rs`). `MigrateIdsReport` has no `error` and no
`error_kinds` field, so that exit-2 report carries neither, while every other not-run report states
its cause in `error` and its kind in `error_kinds`.

`docs/reference/commands.md` says that exit 2 means the report has `"status": "not-run"` and an
`error` and `error_kinds`. `SPEC-1` says a run that examined nothing and says `not-run` for that
reason carries no `error_kinds`. The two disagree, and `migrate ids` fits neither: it examined
records, exits 2, and has no `error`.

Which kind this case is (`config`, since the records do not match the declared header shape, or
`environment`), and whether it should exit 2 at all, is a decision, so this bug is filed `Open`.

## Why nothing caught it

The `error_kinds` work (`MILE-365`) added the key to `CouldNotRun`, `WaiveReport` and the
`harness` and `hooks` reports, the types that print `not-run` with a cause. `MigrateIdsReport` has
its own status enum with a `NotRun` variant and no cause field, so a search for `CouldNotRun` did not
find it, and no test runs `migrate ids --apply` on a corpus where every record without an id is
headerless and checks the report's shape.

## References

- `rust/crates/urzua-cli/src/commands/migrate.rs`: `MigrateIdsReport` and the `NotRun` status.
- `docs/reference/commands.md`, the exit-code paragraph.
- `SPEC-1`, the `error_kinds` bullet ("a run that examined nothing ...").
- `MILE-365`: the work that added `error_kinds` to the other not-run reports. Found in review of
  pull request 649.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
