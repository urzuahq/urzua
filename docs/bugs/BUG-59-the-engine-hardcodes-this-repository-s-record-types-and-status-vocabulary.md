---
Stable-Id: 01M2WAHT989S733NTEAKQ3Y3Q8
Status: Open
Found-in: 'Reviewing a comment that named `waiver` inside a core rule, which surfaced that the engine names the type in its behaviour too'
Regression-test: 'not yet written -- a corpus whose types are named anything else must get the same status handling and the same suppression mechanism as one using this repository's names'
Blocked-on: MILE-98
---
# 59 — The engine hardcodes this repository's record types and status vocabulary

## What is wrong

`urzua-core` is the general-purpose engine. It names this repository's record types in its behaviour:

```rust
// rules.rs -- is_terminal_status
"bug"         => &["Fixed", "WontFix"],
"adr" | "rfc" => &["Accepted", "Rejected", "Superseded"],
"spec"        => &["Accepted"],
"milestone"   => &["Done", "WontDo"],
_             => &[],

// waiver.rs
.filter(|r| r.record_type == "waiver")
```

Both the **type names** and the **status values** are this repository's configuration, compiled in.

## What it costs an adopter

A corpus whose types are `decision` and `proposal` falls to `_ => &[]`, so every status-dependent rule
is permanently inert -- `narrative-field.stale` and `pointer.target-status` examine records and can
never report. `ADR-53` decided every rule is a declared policy; this is the vocabulary those policies
read, and it is not declarable.

Worse for suppression: **the waiver mechanism only works if you name your type `waiver`.** `ADR-11`
decided a waiver is a first-class record rather than a config ignore-list, and the mechanism silently
requires one magic name. An adopter calling theirs `exception` gets no suppression and no error.

`_ => &[]` is the archetype this project cites for *a rule that silently stops applying*. It has been
quoted in eight records this week as a cautionary example while remaining live in the function that
gave it the name.

## Also: `waiver.rs` is a record type as a module

Suppressing a finding is a general capability. `waiver` is what **this corpus** calls the record that
does it. A general-purpose engine should carry the mechanism -- *a record of a declared type can
suppress findings matching a scope* -- and let the configuration name it.

## Fix

Both vocabularies move into the type declaration:

```yaml
record_types:
  adr:
    terminal_statuses: ["Accepted", "Rejected", "Superseded"]
  exception:
    suppresses_findings: true
```

This is `RFC-33`'s point restated one level down: the engine ships the mechanism, the repository
declares the vocabulary. Deferred behind `MILE-98` with the rest of that family rather than hardcoding
a second list beside the first.

## References

- ADR-11 -- a waiver is a first-class record, which this makes conditional on a name.
- ADR-53 -- governance is declared; a status vocabulary is governance.
- RFC-33, MILE-98 -- where the declaration belongs.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** noticed while removing a `waiver` reference from a comment in a core rule -- the comment was a symptom, and the engine names the type in its behaviour. `is_terminal_status` has been quoted in eight records this week as the archetype of a rule that silently stops applying, and it is still live. | **substantive** |
