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
- The adopting repository's input on what made its own board useful, and what a generic one must show to be more than a dump, is recorded in this record before the build starts (the section below).
- The page opens with what waits on a person, what is under way and what is ready, then the commit, branch, whether records have uncommitted changes, and the time the data was read; every record is on the page, with finished and dropped ones collapsed under a count and records with no parent in a group of their own, shown by a test that the number of records on the page equals the number `urzua export` returns.
- Nesting is drawn only where the configuration declares it; the record of this build says which declaration is read as "is part of", and a configuration without it gets types as flat groups.
- A parent shows a count of its children by state beside its own status, and a mark where its status and its children disagree.

## Input from the adopting repository whose board is the reference

Received 2026-11-18, before any build, from the agent that drew the board in this repository's README. Its script is 41 lines over `urzua export --format json`. What it reported, kept here because it decides the shape:

What holds for any repository: one entry per record with id, title, status and one state mark; the states are finished, under way, waiting on (naming the records), ready and dropped, and a record can be under way and waiting at once; children listed inside their parent with an explicit empty case; finished work quiet, ready work prominent; numeric order within a type.

What made its own board wrong, each a thing the built-in one must not repeat: records with no parent, and every type outside its plan, were left off with nothing saying so; a record absent from the order of work read as ready, so finished work was counted as ready; a dropped record showed as ready; a parent's stated status disagreed with its children and nothing showed it; the page carried no commit, branch or time; the page had no colours of its own (they came from the viewer) and was an HTML fragment.

What a board of every record needs to be more than a list: at the top, what waits on a person (a record whose next declared move is reserved by a gate), then what is under way, then what is ready, then when and from what the data was read; work records as the tree the configuration declares, each parent with a count of its children by state beside its own status and a mark where the two disagree; the other types as compact rows grouped by type, not cards; finished and dropped collapsed by default with counts; records with no parent in their own group; search over id and title and a filter by state, in the page; a record's text and what refers to it on tap; headers that could not be read shown, not skipped.

What would make it useless: equal cards in one grid; one column per distinct status across all types; every header field on every entry; a dependency graph as the main view; no sign of how old the data is.

What it could not have read from its configuration, so the build must settle with a declaration and not a guess: that a reference with one declared target type means "is part of" (another typed reference there does not); which statuses mean under way where a type declares no progress set; which fields deserve space on an entry (its suggestion: status, fields with declared values, reference fields, nothing else).

Serving, from a page it does serve: listen on the local address only; no check of the request's host name by default, since a proxy presents its own; relative addresses only; no caching; read-only; a foreground process with a port flag and clean exit codes, so it can run under a service manager and outlive a terminal; a full HTML document with a viewport declaration.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-18 | Initial record, `Status: Planned`. **Why:** the maintainer decided on 2026-11-18 to have a built-in board that shows everything in one standard, unfiltered layout, with `urzua export` for custom views. | **structural** |
> | 2026-11-18 | Added the adopting repository's input and three Done-means items drawn from it (what the page opens with and that no record is left off; nesting only from a declaration; a parent's count of children by state). **Why:** the maintainer asked that the built-in board be done the way that repository's was and that its agent be asked first; the reply arrived the same day. | **substantive** |
