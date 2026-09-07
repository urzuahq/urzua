# 53 — Decide whether a hosted service can reconstruct corpus history retroactively

> Status: Planned
> Stable-Id: 01M1YN88CCNH4KVBE7PKGY30CJ
> Phase: 1
> Track: scale-and-analytics

## What

Decide whether a hosted component could reconstruct a corpus's governance history retroactively (replaying `check` over historical commits) rather than only collecting forward from install, and if so, resolve the open questions that gate it: replay cost, which tool version to replay with, determinism (RFC-5's own staleness model has time-dependent terms), and squashed-history coarsening.

## Why

MILE-24 names "drift trends" generically; this is the specific, harder question underneath it -- whether a trend can start with real history on day one instead of a flat line, and what a hosted service would have to get right to make that trustworthy rather than a demo. A non-goal today (RFC-14 already declines a hosted corpus-holding service), but the concrete case any future ADR on that topic would have to weigh.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
