---
Stable-Id: 01M7FW1C2K2MD1KZD0N37X3E22
Status: Planned
Phase: '2'
Track: embodiment-model
Blocked-on: null
---
# 253 — Evidence categories are declared, and evidence may be external with a hash

## What

Let a configuration declare its own evidence categories, and accept a locator to something outside the repository that carries a hash.

## Why

the audit of 2026-11-15's extensibility experiments found three fixed categories and a requirement that evidence is a tracked file: a compliance control cannot cite an attestation, and a model card cannot cite a dataset.

## Done means

- A decision record states how declared categories map to tiers and how an external locator is verified.
- A configuration that declares a category `attestation` accepts locators of that category.
- An external locator with a hash is reported as unverifiable offline, never as verified, and a command re-checks it on request.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
