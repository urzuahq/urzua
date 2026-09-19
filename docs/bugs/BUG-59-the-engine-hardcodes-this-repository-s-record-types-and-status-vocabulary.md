---
Stable-Id: 01M2WAHT989S733NTEAKQ3Y3Q8
Status: Fixed
Found-in: 'Reviewing a comment that named `waiver` inside a core rule, which surfaced that the engine names the type in its behaviour too'
Regression-test: "rust/crates/urzua-core/src/rules.rs::narrative_field_stale_judges_a_type_the_engine_has_never_heard_of"
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

Round 6 of the 0.4.0 review measured the reach. `pointer.target-status` escaped the fallback by
reading its vocabulary from `not_in`, but `narrative-field.stale` still calls `is_terminal_status`,
so the arm is live for it: any declared type outside the five compiled-in names gets `false` for
every status, including its terminal ones, while the record is counted as examined and can never
produce a finding.

This corpus does not currently reach it. `narrative_field_stale` returns early when the target has no
`Status`, and the only type here outside the five -- `waiver` -- declares none, so a `Blocked-on`
pointing at a waiver stops before the fallback rather than falling through it. The exposure is an
adopter's: a type named anything else that does carry a `Status`, which is the ordinary case, and
`init` proposes type names straight from directory names. Round 7 built that adopter -- a `dec` type
whose target is `Ratified` -- and measured `records_examined: 1`, zero findings, `status: ok`.

## What was fixed

The status half. `narrative-field.stale` now takes a declared `terminal_statuses`, required like
`pointer.target-status`'s `not_in`, so an undeclared vocabulary fails to load rather than going
inert. It was the only caller, so `is_terminal_status` is deleted rather than corrected -- the
archetype is gone from the codebase, not merely routed around.

The suppression half stays open: `waiver.rs` still requires the type to be named `waiver`. Filed
separately rather than held here, since the status vocabulary no longer blocks on `MILE-98`.

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
