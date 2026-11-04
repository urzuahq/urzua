---
Stable-Id: 01M6K68092WYW68GYMDXNE7SG8
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/citation.rs, code:rust/crates/urzua-core/src/config.rs, test:rust/crates/urzua-cli/tests/refs_integration.rs
---
# 184 — citation.issue-exists also checks declared header fields

## What

`citation.issue-exists` reads the header fields a repository names in `issue_refs.fields` (for example `Found-in`) for issue and pull request citations, as well as the declared reference sections. The option is off unless declared, and a field name no record type declares is refused at load.

## Why

The rule read only `reference_sections`, so a wrong PR number in a header field such as a BUG's `Found-in` went unchecked, though that is where a wrong number matters most (`MILE-143`). Body prose outside the declared sections is deliberately not made checkable: a bare `#N` there is as often an example or a counter as a citation.

## Done means

A number or repository link in a declared header field that is not in the committed snapshot is a finding naming the field and its line; with no `fields` declared the rule behaves as before; an undeclared or blank field name is a load error; this repository declares `fields: ["Found-in"]`, the only header field here that carries PR numbers, and `urzua check` is clean.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed and delivered: `issue_refs.fields` (`SPEC-2`, `SPEC-3`), enabled for `Found-in` here; the snapshot was refreshed first and no dangling number was found. `Realized-by` and `Status: Done` set by the delivering change. **Why:** a wrong number in a header field passed unchecked; `MILE-143` was already Done, so this is a new milestone. | **structural** |
