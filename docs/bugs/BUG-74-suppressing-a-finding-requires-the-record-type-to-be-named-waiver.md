---
Stable-Id: 01M2WGK3J9WW6NCJAYGZPE8FW4
Status: Open
Found-in: "Splitting BUG-59 once its status-vocabulary half was fixed"
Regression-test: "not yet written -- a corpus whose suppression record type is named anything else must still suppress"
Blocked-on: RFC-37
---
# 74 — Suppressing a finding requires the record type to be named waiver

## What was wrong

`ADR-11` decided that suppressing a finding is done with a record rather than a config ignore-list.
The engine implements that by filtering on a literal:

```rust
.filter(|r| r.record_type == "waiver")
```

`waiver` is what **this repository** calls that record. An adopter who calls theirs `exception` gets
no suppression and no error -- the mechanism is simply absent, and nothing says so. The module is
named `waiver.rs` for the same reason: a general capability named after one corpus's word for it.

This is the second half of `BUG-59`. The first half -- a status vocabulary keyed on this repository's
type names -- is fixed: `narrative-field.stale` now reads a declared `terminal_statuses` and
`is_terminal_status` is deleted. Suppression is the same defect and needs the same treatment: the
type that carries a waiver is a declaration, not a name the engine knows.

`ADR-11` is not in question and does not change. It decided a waiver is a first-class record, and its
own Consequences section already required that the type be declared "the same way `adr`/`rfc`/`spec`
are -- **no special-casing in the tool**." The literal above is the special-casing that decision ruled
out, present from the start. The fix honours `ADR-11` rather than revisiting it.

## Where the declaration goes

Blocked on `RFC-37`, not on `MILE-98`: which type carries suppression is neither identity, fields nor
sections, so the document model does not gate it -- the same reasoning that unblocked `BUG-59`'s
status half.

It is blocked all the same, because the cheap fix is wrong. Adding a `suppresses_findings` key makes
a tenth type-level field, and `MILE-55` will want an eleventh for roles. `RFC-37` proposes one
`capabilities` axis instead, under which this is a declared value rather than a new key. Fixing it as
a key first means fixing it twice.

The waiver's *field* names -- `Rule`, `Scope`, `Expires`, also literals in the same function -- are a
separate concern and stay with `MILE-98`, where which fields a record carries is the subject.

## Why nothing caught it

Every test and every run is against this repository's own configuration, where the type *is* named
`waiver`, so the literal is always satisfied. A rule that is correct for exactly one input is
indistinguishable from a correct rule until a second input exists.

`MILE-51`, adopting a foreign corpus, is where a second input comes from, and it has not run against
suppression.

## References

- `BUG-59`, whose status half is fixed and whose suppression half this is.
- `ADR-11`, which decided a waiver is a record.
- `MILE-98`, the declared document model.
- `MILE-51`, the foreign-corpus validation.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
