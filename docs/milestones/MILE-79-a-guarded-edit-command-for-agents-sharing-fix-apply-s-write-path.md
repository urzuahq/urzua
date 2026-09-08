# 79 — A guarded edit command for agents, sharing fix apply's write path

> Status: Planned
> Stable-Id: 01M1Z2D54784252A0PQMKM8XV6
> Phase: 2
> Track: write-path
> Implements: ADR-14, ADR-15, ADR-19
> Blocked-on: —

## What

Design and build the shared write primitive both `fix --apply` (ADR-19, deferred pending exactly
this) and a new `urzua edit` command need: targeted mutation of a specific header field or section
without disturbing the rest of a record, plus a revision-log append (ADR-14, now Why-bearing for
substantive entries). `fix --apply` uses it under RFC-8/ADR-15's narrow eligibility test (the tool
writing a value it computed itself). `urzua edit` uses it under a different, looser policy: an
agent or human supplies the new content, and the tool's job is guarding the mechanical parts of the
edit -- refusing to let a full-record rewrite silently clobber `Stable-Id` or an already-resolved
`Author`/`Deciders`, auto-appending a correctly dated/classed revision-log row instead of a
hand-typed one, and re-running `check` immediately after to catch a broken edit before it's
committed. One write primitive, two policies -- not two separately-built write paths.

## Why

Proposed live, right after ADR-38 was found to have exactly the failure this would prevent: `urzua
new` correctly auto-resolved `Author` via `resolve_identity()`, and a subsequent hand-edit (mine,
this session, rewriting the whole header block to update its content) silently overwrote it back to
the old placeholder text, undetected until a much later corpus-wide check. ADR-19 already named the
two missing pieces `fix --apply` needs -- field-level mutation and a revision-log write-path -- and
this is the same gap. Building it once, shared, avoids the alternative of `fix --apply` and `edit`
each growing their own ad hoc mutation code that drifts from each other.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
