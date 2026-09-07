# 72 — Add a top-level next_action to every command's output

> Status: Planned
> Stable-Id: 01M1YS1N3TFEQC307R6JQV66NZ
> Phase: 1
> Track: schema-governance
> Implements: ADR-23

## What

A single top-level `next_action` field on every command's JSON output, alongside `status` and
`findings`: a plain string (or `null`) naming the single most useful next command to run, given
everything in this response. Distinct from MILE-44's `suggested_action`, which is scoped to one
finding at a time: `next_action` looks at the response as a whole and recommends one thing, e.g.
`"run urzua fix to auto-repair 3 of these findings"` after a `check` that found mixed
mechanically-fixable and human-judgment findings, or `null` when there's nothing better to do than
read the findings themselves.

## Why

Urzua's own stated design is that an agent acts on output directly instead of interpreting a
sentence first (README). `status` and `findings[]` already remove the need to interpret what
happened. Nothing yet removes the smaller but real step of deciding what to do about it: an agent
still has to reason from a findings list to a plan. A single recommended next command is the same
category of value `suggested_action` already provides per-finding, applied once to the whole
response instead of forcing the caller to synthesize one from several.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
