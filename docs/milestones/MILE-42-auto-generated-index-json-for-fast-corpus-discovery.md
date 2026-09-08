# 42 — Auto-generated INDEX.json for fast corpus discovery

> Status: Planned
> Stable-Id: 01M1YN82MMNP7V6P4WB6FK7XKB
> Phase: 1
> Track: discoverability
> Blocked-on: —

## What

A generated `INDEX.json` (or similar) summarizing the corpus -- record type, status, title -- so an agent can query a compact index instead of reading every record.

## Why

`explain`/`graph` already answer "which records govern this file" and "what's the relationship graph," but nothing answers "what exists in this corpus at all" without reading every file. An agent with a large corpus and a token budget needs the cheap, coarse answer before it needs the precise one.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
