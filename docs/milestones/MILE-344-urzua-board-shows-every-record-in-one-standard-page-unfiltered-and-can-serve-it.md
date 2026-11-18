---
Stable-Id: 01M7PE9S5QSJ1GHQD5CR21DD1P
Status: Planned
Phase: '1'
Track: integrations
Blocked-on:
---
# 344 — urzua board shows every record in one standard page, unfiltered, and can serve it

## What

A command `urzua board` writes one self-contained HTML page showing every record the repository holds, laid out only from what its configuration declares, and with `--serve` answers it at a local address until stopped. `urzua export` stays the way to build any other view.

## Why

The maintainer asked for this on 2026-11-18: "I think a built-in Erzua board would be useful, but the board would be pretty useless to most people. Like, it just displays shit. It's not organized in any nice way. So they always have the board that just exports like a standardized output unfiltered. Because remember that they could have any kind of workflow or anything. So yeah, I think we'll have board and then obviously they can have the export option and write their own scripts around it." ("Erzua" is the dictation's spelling of urzua.) An adopting repository had drawn a board of its own plan from `urzua export --format json` with about forty lines of script, and its picture is the demo in this repository's README; the maintainer asked that the built-in one be done the way that one was and that its author be asked for input first. This is the one command whose output is a page for a person. Every other command prints JSON for an agent, and stays so.

## Done means

- `urzua board` writes one HTML file with no network requests and no build step, to a path it prints in its JSON answer; `urzua board --serve` answers the same page on a local address and port it prints, re-reading the records on each request, until interrupted.
- The page shows every record of every declared type, unfiltered: its id, title, status, and its references to other records as links within the page; nothing is hidden by default.
- The layout is derived only from the configuration: records grouped by declared type; each status shown by the role the configuration gives it (the type's success, failure and in-progress lists), never by a status name; relation pairs and reference fields drawn as the links they declare; the order of work shown where the configuration declares the pairs `urzua roadmap` reads. No record type, status name or workflow word appears in the code, shown by the same test that holds this for the engine.
- Two repositories with different types and statuses produce correct boards from the same binary, shown by a test on two fixture configurations and on this repository's own records.
- The page is readable at phone width and in light and dark, and stays usable with this repository's record count.
- What the board needs from the records comes through the same data `urzua export --format json` carries (`MILE-330`); the board computes nothing an exporter's consumer could not.
- The adopting repository's input on what made its own board useful, and what a generic one must show to be more than a dump, is recorded in this record before the build starts.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-18 | Initial record, `Status: Planned`. **Why:** the maintainer decided on 2026-11-18 to have a built-in board that shows everything in one standard, unfiltered layout, with `urzua export` for custom views. | **structural** |
