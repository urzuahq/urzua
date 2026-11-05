---
Stable-Id: 01M6NE3QYKTPYG5TR1WYE9QJ54
Status: Planned
Phase: '0'
Track: schema-governance
Implements: ADR-86
Blocked-on: ADR-86 being Accepted
---
# 193 — A waiver may be declared permanent under a declared reason vocabulary

## What

The suppression type can declare `permanent` (`ADR-86`): per rule, a waiver without `Expires` takes effect only with a category from a declared vocabulary, and a citation where the category requires one. A repository declaring nothing is unchanged.

## Why

`ADR-84` makes a done record without evidence a finding and names a waiver as the exemption route, but a waiver cannot state a permanent exception on a declared cause: 19 `Fixed` defects here lack a test for a reason that will not change. The maintainer asked for the enhancement on 2026-11-05.

## Done means

- A fixture config declaring `permanent` for a rule reports a no-expiry waiver for that rule with a category outside the vocabulary, or without a required citation, as `waiver-ineffective` (naming the missing field), and its finding is not waived; with a valid category and citation the finding is waived. A waiver with `Expires` behaves as before. A fixture declaring no `permanent` behaves exactly as today.
- A no-expiry waiver for a rule that `permanent` does not list is ineffective once the key is declared.
- A permanent waiver whose finding disappears is reported by `waiver.matches-nothing`; a cited record that is superseded or retired is reported by `pointer.resolution`.
- `urzua check` reports the number of permanent waivers by category.
- Config load rejects a `permanent` entry naming an unknown rule, an undeclared field, an empty or duplicated category list, or a `cite_field` that is not a pointer field.
- It is exercised in urzua's own `.urzua/config.yaml` and corpus: `permanent` is declared for `status.delivery-agreement`, and the waiver template gains `Category` and `Justified-by`. `SPEC-2`, `SPEC-3` and `SPEC-10` document it with revision rows, and a changeset (minor) is added.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** maintainer request of 2026-11-05; decided in `ADR-86` (Proposed). Depends on `ADR-86` being Accepted. | **substantive** |
