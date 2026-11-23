---
Stable-Id: 01M7PGQA4BNM2BCTVDTZC49N62
Status: Proposed
Date: 2026-11-18
Author: beauwilliams
Deciders: beauwilliams
Embodiment: Not started
Supersedes / Superseded-by: null
---
# 103 — The board leaves the engine: it is the first ecosystem client, a binary in this repository that reads only published JSON

## Context

Every command prints one JSON object on stdout, for an agent, and nothing else (`ADR-23`); each is a `Report` through `emit` (`ADR-46`). The maintainer asked on 2026-11-18, in `MILE-344`, for a built-in board of the records, in these words:

> "I think a built-in Erzua board would be useful, but the board would be pretty useless to most people. Like, it just displays shit. It's not organized in any nice way. So they always have the board that just exports like a standardized output unfiltered. Because remember that they could have any kind of workflow or anything. So yeah, I think we'll have board and then obviously they can have the export option and write their own scripts around it."

("Erzua" is the dictation's spelling of urzua.) `urzua board` was built as the one command whose product is a page for a person, with stdout staying JSON. That left a second kind of product in the command set and the binary's only HTTP listener (`--serve`), for a view that `RFC-27` §3 and its 2026-11-08 amendment class as ecosystem: a generated presentation of the records, computed from the records and the configuration alone.

On 2026-11-23, in `RFC-50` (polyrepo support), the maintainer changed direction: "maybe somehow we should pull the board out of urzua? Like basically the 'client' for the ecosystem?" A cross-repository org view is also ecosystem, and has to live somewhere that is not the engine; the board is the natural first place. The maintainer then answered four questions, quoted below.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Keep `urzua board` in the engine (the earlier decision of this record) | One binary, no build step; already built | A second product kind and a listener in the engine; a presentation inside the engine against `RFC-27` §3; the org view would have to go in the engine too |
| B. A second Rust binary in this repository that reads only published JSON, shipped in the same release archives; `urzua board` removed | One toolchain, one release, one place for tests; the client is a real consumer of the published JSON, so the contract is exercised by the first client | A second binary to build and package; the export has to carry everything the board reads today before the client exists |
| C. A client in another language or repository | Free choice of UI tools | A second toolchain and release, and the contract loses its in-repository test |
| D. No board; every repository writes its own from the export | Nothing to maintain | The maintainer decided against it on 2026-11-18; every adopter repeats the same wrong turns |

## Decision

In the context of a board that is a presentation, ecosystem under `RFC-27`, and an org view that `RFC-50` needs somewhere outside the engine, we decided that `urzua board` leaves the engine and the board becomes the first ecosystem client: a Rust binary in its own crate in this repository, shipped in the same release archives, that reads only published `urzua export` and `urzua check` JSON, to achieve an engine whose commands all print JSON and a client that exercises the published contract, accepting a second binary to package and a breaking removal.

- **Language and place.** The maintainer chose "Rust second binary (Recommended)". The client depends on no engine crate; its only input is JSON, a file or stdin.
- **Removal.** The maintainer chose "Break it, we are only users": the command is removed when the client ships, with no alias and no deprecation period.
- **Sequence.** The maintainer chose "Export first, then client (Recommended)". The export first gains its format version (`RFC-50` §6, `MILE-359`) and the config-declared data the board reads today: each type's statuses and their roles, and its sections (`rust/crates/urzua-cli/src/commands/board_declared.rs`). The export's half is `MILE-361`; the client and the removal are `MILE-362`.
- **What carries over.** The client keeps what the earlier decision required of the page: it names no record type, status or workflow word, makes no network request, escapes everything from a record, and computes nothing a consumer could not read from the export. How it is served and drawn (a file, a local listener, the application shell, the `--prompt` text) is the client's own design and is not decided here.
- **The org view.** The client is where the cross-repository view is built later, from the committed sibling snapshots (`RFC-50` §4). That view is not decided here.

Decided by the builder on 2026-11-23, pending maintainer review, from the maintainer's answers of that date. The words in quotation marks are the maintainer's; everything else in this record is the builder's. This record stays `Proposed`: accepting it is the maintainer's.

## Reversibility

Moderate. The removal is a break by decision, so a script that calls `urzua board` stops working and nothing here eases that. Restoring the command would be a new decision. The export's additions stand on their own and serve any client.

## Consequences

- Stdout is JSON on every command with no exception, and the engine has no listener; `ADR-23` holds without the second product kind.
- The client is the first consumer of the published export, so the contract is exercised by a real one; a change to the export's shape needs a version change.
- The release carries two binaries.
- Until the client ships, `urzua board` stays as built (`MILE-344`, `MILE-349`). The earlier amendment of 2026-11-18, which described that command, is dropped from this record and stays in git history.
- `docs/reference/board.md` and the README demo describe the client once it exists (`MILE-362`).

## References

`RFC-27` (§3 and the amendment of 2026-11-08), `RFC-50`, `MILE-344`, `MILE-349`, `MILE-359`, `MILE-361`, `MILE-362`, `ADR-23`, `ADR-46`, `docs/reference/board.md`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-18 | Initial record, `Status: Proposed`. **Why:** `MILE-344` records the maintainer's request for a built-in board. | **structural** |
> | 2026-11-18 | The Consequences say what was built: a served request does not run `urzua check` (it links `/check`, one run at a time, cached against the digest), the page and the export behind it are cached against the digest, connections are threaded up to a cap, and `--allowed-host` is available. The Decision is unchanged. **Why:** the Consequences said each request ran `urzua check`, which the served page does not do. | **substantive** |
> | 2026-11-18 | Amendment added: the page is drawn as an application shell in the urzua brand, and `urzua board --prompt` prints a prompt for building a view of one's own. The Decision is unchanged. **Why:** the maintainer asked on 2026-11-18 for a board that is not opinionated, looks like a proper web page and offers a prompt for an agent to build a view (`MILE-349`). | **substantive** |
> | 2026-11-23 | Reworked in place while `Proposed`: the decision is now that `urzua board` leaves the engine and the board is the first ecosystem client, a Rust binary in this repository reading only published JSON, removed from the engine with no alias, the export gaining its version and declared data first. The title, Options, Decision, Reversibility, Consequences and References changed; the 2026-11-18 amendment is dropped, as its text described the command now being removed (it stays in git history). **Why:** the maintainer's answers of 2026-11-23: "maybe somehow we should pull the board out of urzua? Like basically the 'client' for the ecosystem?"; "Rust second binary (Recommended)"; "Break it, we are only users"; "Export first, then client (Recommended)" (`RFC-50`). `Status` stays `Proposed`. | **substantive** |
