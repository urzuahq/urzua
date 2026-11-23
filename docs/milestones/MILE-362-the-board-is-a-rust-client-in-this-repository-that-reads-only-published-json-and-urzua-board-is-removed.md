---
Stable-Id: 01M83NHABRKR1X1ZZ25RA3CZYM
Status: Planned
Phase: '1'
Track: integrations
Implements: ADR-103
Blocked-on: MILE-361
---
# 362 — The board is a Rust client in this repository that reads only published JSON, and urzua board is removed

## What

The board is a separate Rust binary in this repository, in its own crate, that reads only published `urzua export` and `urzua check` JSON, and ships in the same release archives. `urzua board` is removed from the engine with no alias and no deprecation period. The board's tests move with it.

## Why

The maintainer's direction of 2026-11-23: "maybe somehow we should pull the board out of urzua? Like basically the 'client' for the ecosystem?" Their answers the same day: "Rust second binary (Recommended)" for the client's language, and "Break it, we are only users" for the old command, so nothing keeps `urzua board` alive and no one is asked to migrate. The board is ecosystem under `RFC-27` §3 and its 2026-11-08 amendment, and `RFC-50` makes it the first ecosystem client, the place a cross-repository org view is built later. It waits on the export gaining its version and declared data (`MILE-361`) so that the client needs nothing but published JSON.

## Done means

- A crate in `rust/` builds a binary that takes its input only from `urzua export --format json` and `urzua check` JSON (a file or stdin), and a test fails if the crate depends on `urzua-core` or `urzua-cli`.
- The release workflow's archives contain the client's binary beside `urzua`, shown by the packaging step's file list in a release dry run.
- `urzua board` no longer exists: the subcommand, its `--serve`, `--prompt` and `--out` flags and the code behind them are removed from the engine with no alias, shown by `urzua board` exiting with an unknown-command error in a test.
- The board's integration tests now run against the client, and the acceptance expectation that names the old command's flags is removed or rewritten; no test is deleted without the same case existing for the client.
- The client renders this repository's own corpus and a fixture corpus of a different shape from their exports alone, shown by a test on each.
- A changeset names the removal of `urzua board` as a breaking change; `docs/reference/board.md`, `docs/reference/commands.md` and the README demo describe the client instead.
- `ADR-103` is `Accepted` by the maintainer before this milestone moves to `Done`; the status is the maintainer's to set, and this milestone does not set it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-23 | Filed as planned work: nothing built, no evidence yet. Filed from the maintainer's answers of 2026-11-23: "Rust second binary (Recommended)" and "Break it, we are only users". | **structural** |
