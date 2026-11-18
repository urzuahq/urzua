---
Stable-Id: 01M7PGQA4BNM2BCTVDTZC49N62
Status: Proposed
Date: 2026-11-18
Author: beauwilliams
Deciders: beauwilliams
Embodiment: Not started
Supersedes / Superseded-by: null
---
# 103 — urzua board is the one command whose product is a page for a person while stdout stays JSON

## Context

Every command prints one JSON object on stdout, for an agent, and nothing else (`ADR-23`); each is a `Report` through `emit` (`ADR-46`). The maintainer asked on 2026-11-18, in `MILE-344`, for a built-in board of the records, in these words:

> "I think a built-in Erzua board would be useful, but the board would be pretty useless to most people. Like, it just displays shit. It's not organized in any nice way. So they always have the board that just exports like a standardized output unfiltered. Because remember that they could have any kind of workflow or anything. So yeah, I think we'll have board and then obviously they can have the export option and write their own scripts around it."

("Erzua" is the dictation's spelling of urzua.) An adopting repository had drawn a board of its own plan from `urzua export --format json` with about forty lines of script. What its script had to compute or got wrong is the input recorded in `MILE-344` and the amendment of `MILE-330`: states with no category, a parent's status disagreeing with its children, no read stamp, records left off with nothing saying so. A page is not JSON, so it cannot be a report the way every other command's output is.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. One command, `urzua board`, whose product is a page: written to a file and printed as a JSON answer naming it, or served at a local address with `--serve`; stdout stays JSON | Keeps `ADR-23` whole: stdout is JSON on every command, the page is a file or a response. One binary, no build step, no dependency. The page is a template over the export, so it computes nothing a consumer could not read | A second kind of product in the command set; a server (read-only, loopback, standard library) in a tool that has had none |
| B. `urzua export --format html` | One command to learn | `ADR-23` forbids a second rendering of a command's output on stdout, and `--format agdr` shows a `--format` that prints something else invites more |
| C. No built-in board; every repository writes its own from the export | Nothing to maintain | The maintainer decided against it; every adopter repeats the same wrong turns (records left off, finished work read as ready) |

## Decision

In the context of a maintainer who wants a standard, unfiltered board of any repository's records, facing a command set whose stdout is always JSON, we decided to add one command, `urzua board`, whose product is an HTML page for a person: it writes one self-contained file (default `.urzua/cache/board.html`, or `--out`) and prints a JSON answer naming it, or with `--serve` answers the same page at 127.0.0.1 until interrupted, to achieve a board that shows every record laid out only from the configuration while `export` stays the way to build any other view, accepting a long-lived process and an HTTP listener in a tool that had neither. The page reads only what `urzua export` returns (`MILE-330`), names no record type, status or workflow word, makes no network request, escapes everything from a record, and the server answers `GET` only.

Decided by the builder on 2026-11-18, pending maintainer review. The words quoted in Context are the maintainer's; everything else in this record is the builder's.

## Reversibility

Easy. The command is additive: removing it removes a page and a listener, and no other command reads it. The export's additions it relies on stand on their own. A script that called `urzua board` would break; nothing in the repository's gates does.

## Consequences

- Stdout is still JSON on every command; `urzua board` prints a JSON answer, and `--serve` one JSON line.
- The page is a second consumer of the export's view model, so a change to a state or a wait changes the board and the export together.
- `--serve` is the one place the binary listens. It binds 127.0.0.1 only, answers `GET` only, serves no file from disk by path, and holds no state, so a proxy in front of it is the way to reach it from another device. It ends by the default action of an interrupt, since the standard library installs no handler and the crate forbids unsafe code.
- A request does not run `urzua check`: the served page links `GET /check`, which runs one at a time, shares a run in flight and keeps its result until the records or the configuration change. The page and the export behind it are cached against `GET /stamp`'s digest, so an unchanged repository answers from memory; each connection has its own thread up to a cap, and `--allowed-host` refuses other `Host` names.
- A record's text is escaped plain text; for a very large corpus the text is left out of the page (linked when served).

## References

`MILE-344`, `MILE-330`, `ADR-23`, `ADR-46`, `docs/reference/board.md`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-18 | Initial record, `Status: Proposed`. **Why:** `MILE-344` records the maintainer's request for a built-in board. | **structural** |
> | 2026-11-18 | The Consequences say what was built: a served request does not run `urzua check` (it links `/check`, one run at a time, cached against the digest), the page and the export behind it are cached against the digest, connections are threaded up to a cap, and `--allowed-host` is available. The Decision is unchanged. **Why:** the Consequences said each request ran `urzua check`, which the served page does not do. | **substantive** |
