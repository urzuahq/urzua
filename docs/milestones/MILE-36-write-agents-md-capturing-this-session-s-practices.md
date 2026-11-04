---
Status: Done
Stable-Id: 01M1Y9H4N2TZ88D8CTWZP6G5RG
Phase: '0'
Track: roadmap-tracking
Implements: —
Blocked-on: —
Realized-by: code:AGENTS.md
---
# 36 — Write AGENTS.md capturing this session's practices

## What

`AGENTS.md`: durable instructions for any AI agent working in this repo -- use the tool's own
record types to track work, verify claims against real runs rather than memory, never silently
rewrite an Accepted decision, and don't build speculative capability.

## Why

A lot of hard-won practice from this project's own history lived only in conversation transcripts,
with no durable, discoverable home a fresh agent session would actually read. Named directly as
something worth writing rather than letting it keep living nowhere.

## Done means

`AGENTS.md` exists at the repository root and instructs an agent to track work in the tool's own record types, to verify claims against real runs rather than memory, never to silently rewrite an Accepted decision (amend with a dated `## Amendment`), and not to build speculative capability; each is a section an agent reads at session start.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial record: already complete at creation time. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
