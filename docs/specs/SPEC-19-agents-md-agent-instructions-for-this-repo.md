---
Stable-Id: 01M20SH9CXA18HFPABZJQY795D
Status: Accepted
Date: 2026-09-08
Version: '0.1'
Author: '@beauwilliams'
Implements: MILE-36
Parent: SPEC-1
---
# SPEC-19 — `AGENTS.md`

## Purpose

`AGENTS.md` is this repo's durable, discoverable home for practices an AI agent needs to work in it
correctly — written directly by [MILE-36](../milestones/MILE-36-write-agents-md-capturing-this-session-s-practices.md)
with no preceding RFC or ADR, the same "no decision point existed" situation as `doctor` (SPEC-15):
it was named as worth writing, not debated into existence. This spec exists to give that already-
built artifact the same governed home every other feature area has (ADR-41's "coherent feature area"
principle), going from the milestone straight to a spec — skipping the ADR stage is legitimate when,
as here, there was no real decision to record (RFC-4's own "common path, not the only path" already
established this).

Distinct from a `spec`-the-record-type instance in one respect: `AGENTS.md` itself is not a governed
record (`urzua check` doesn't validate it — it has no `.urzua/config.toml` entry, no header, no
`Status`). This spec is the governance layer *about* that file: what belongs in it, how it's
maintained, and a place for `Why` to accumulate as it changes, since `AGENTS.md`'s own prose doesn't
carry a revision log the way a record does.

## What belongs in `AGENTS.md`

Its own opening line draws the boundary already: `CONTRIBUTING.md` is the human-facing equivalent;
`AGENTS.md` states what that file doesn't need to. Concretely, as of this spec's writing, three
sections:

- **Use the tool on itself** — track multi-step work as a `milestone`/`bug` via `urzua new`, not
  scratch notes; verify a cross-reference actually resolves via `urzua graph` before treating linked
  work as done.
- **Verify before trusting** — run the actual command against real input rather than reading code
  and assuming it behaves as documented; a new rule needs an observed-failing planted-violation test
  before it needs anything else.
- (A third section, present in the file itself, on when *not* to build: no speculative capability
  ahead of a real, current need.)

## How it's maintained

Edited in place, like a spec — not append-only the way an ADR's Decision text is frozen. Unlike a
`spec`-type record, there's no mechanical enforcement of this (no `Version`/revision-log fields on
`AGENTS.md` itself to check); this spec's own revision log is where the "why this changed" history
actually lives, since the file being governed can't carry it.

## What's deliberately not built

- **Making `AGENTS.md` itself a governed record type** — it would need a home directory, a header,
  and a `Status` that mean something (a single-instance type is a real pattern already used
  elsewhere? not currently, in this corpus) — not attempted here; this spec's own revision log is a
  sufficient history for a file this size, at this frequency of change, without inventing a new
  record-type instance for a single file.
- **A `check` rule verifying `AGENTS.md`'s content matches what this spec describes** — would require
  parsing prose for section presence, the same permanent content-scope ceiling SPEC-1 already names
  for structural-vs-content checking generally.

## References

- MILE-36 — wrote `AGENTS.md`; this spec's `Implements` pointer.
- ADR-41 — the "coherent feature area, decided editorially" principle this backfill follows, the
  same one that produced SPEC-6/9/10/16/17/18.
- RFC-4 — "RFC → ADR → Spec: the common path, not the only path" — the precedent for skipping
  the ADR stage when no real decision existed to record.
- SPEC-15 — `doctor`, the other feature area in this corpus with no ADR behind it, for the same
  reason.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial spec. **Why:** `AGENTS.md` was written directly by MILE-36 with no spec of its own — found live reviewing the dashboard, the same "implemented without any spec" gap ADR-41's review already found for `milestone`/`bug`/`waiver`/`adr`/`rfc`/`spec`, just for a file rather than a record type. | **structural** |
