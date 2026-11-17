---
Stable-Id: 01M7KKNVWA6SVZ0V13JH4R1C3K
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 326 — A declared requirement reports without a second switch

## What

Have the rules that report `statuses.require` and `statuses.forbid` entries be on by default wherever a type declares such an entry, the way the rules for declared field values and declared target types are, and have `urzua doctor` name a declared entry that no enabled rule can report.

## Why

In release 0.4.7 `field.value-not-declared` and `pointer.target-type` switch on by themselves once a configuration declares the key they read. `status.required-field-missing` and `status.forbidden-field-present` do not: a `require` entry, including one keyed on another field's value, reports nothing until the rule is named under `rules:`, and a freshly initialised configuration does not name it. The adopting agent wrote: "Without your note I would have declared `require` with `when_field` and seen nothing." A declaration that silently does nothing is the failure the tool exists to prevent.

## Done means

- `status.required-field-missing` is on by default at error where any type declares a `require` entry, and `status.forbidden-field-present` where any type declares a `forbid` entry, shown by a test with neither rule named under `rules:`.
- A configuration that declares neither entry reports exactly as before, and a configuration that names either rule keeps the level it gives.
- `urzua doctor` reports, at warn, a declared entry whose rule the configuration turns off, shown by a test.
- The release notes state the change of default, since a configuration with entries and without the rules will start to report.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Reported on 2026-11-17 by an agent in an adopting repository after moving to release 0.4.7, relayed at the maintainer's request. | **structural** |
