---
Stable-Id: 01M6K68092WYW68GYMDXNE7SG8
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/refs_integration.rs#a_declared_header_field_is_checked_end_to_end_and_only_when_declared@sha256:ca288cd98d9026d70c768cf8ea537ac934369039c667bdc01e546135092754e2, code:rust/crates/urzua-core/src/rules/citation.rs, test:rust/crates/urzua-core/src/config.rs#issue_refs_fields_must_be_declared_header_fields_and_never_blank@sha256:3c1d9033c06d0894d26493fddc21ead263fc5f9fd3141161aae4dbb825327fb7, code:.urzua/config.yaml
---
# 184 — citation.issue-exists also checks declared header fields

## What

`citation.issue-exists` reads the header fields a repository names in `issue_refs.fields` (for example `Found-in`) for issue and pull request citations, as well as the declared reference sections. The option is off unless declared, and a field name no record type declares is refused at load.

## Why

The rule read only `reference_sections`, so a wrong PR number in a header field such as a BUG's `Found-in` went unchecked, though that is where a wrong number matters most (`MILE-143`). Body prose outside the declared sections is deliberately not made checkable: a bare `#N` there is as often an example or a counter as a citation.

## Done means

- A number or repository link in a declared header field that is not in the committed snapshot is a finding naming the field and its line. — evidence: test:rust/crates/urzua-cli/tests/refs_integration.rs#a_declared_header_field_is_checked_end_to_end_and_only_when_declared@sha256:ca288cd98d9026d70c768cf8ea537ac934369039c667bdc01e546135092754e2, code:rust/crates/urzua-core/src/rules/citation.rs
- With no `fields` declared the rule behaves as before. — evidence: test:rust/crates/urzua-cli/tests/refs_integration.rs#a_declared_header_field_is_checked_end_to_end_and_only_when_declared@sha256:ca288cd98d9026d70c768cf8ea537ac934369039c667bdc01e546135092754e2
- An undeclared or blank field name is a load error. — evidence: test:rust/crates/urzua-core/src/config.rs#issue_refs_fields_must_be_declared_header_fields_and_never_blank@sha256:3c1d9033c06d0894d26493fddc21ead263fc5f9fd3141161aae4dbb825327fb7, test:rust/crates/urzua-cli/tests/refs_integration.rs#a_declared_header_field_is_checked_end_to_end_and_only_when_declared@sha256:ca288cd98d9026d70c768cf8ea537ac934369039c667bdc01e546135092754e2
- This repository declares `fields: ["Found-in"]`, the only header field here that carries PR numbers, and `urzua check` is clean. — evidence: code:.urzua/config.yaml

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed and delivered: `issue_refs.fields` (`SPEC-2`, `SPEC-3`), enabled for `Found-in` here; the snapshot was refreshed first and no dangling number was found. `Realized-by` and `Status: Done` set by the delivering change. **Why:** a wrong number in a header field passed unchecked; `MILE-143` was already Done, so this is a new milestone. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
