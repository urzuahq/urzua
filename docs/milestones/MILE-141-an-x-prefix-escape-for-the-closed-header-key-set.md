---
Stable-Id: 01M6HZ0P342ZYCS29CZ780Y9EV
Status: Planned
Phase: '0'
Track: schema-governance
Implements: RFC-33
---
# 141 — An x- prefix escape for the closed header-key set

## What

Let a type's closed header-key set admit extension keys by a declared prefix, `x-` by convention, so a repository can add a local field without declaring it in `known_fields` and without weakening the closed set for every other key.

## Why

`header.field-set-consistency` treats the union of `required_fields` and `known_fields` as closed, so every local extension has to be added to config before it is used. The adopter's linter closes its key set but accepts any key with an `x-` prefix, which lets a team annotate a record without a config change and still catches a typo in a real key. Without an equivalent, the adopter either declares every extension up front or loses typo detection.

## Done means

A header key with the declared prefix passes the field-set rule and an unprefixed unknown key still fails it; the prefix is a per-type config value, absent by default; a prefixed key is not treated as a pointer or narrative field unless declared.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter accepts `x-` prefixed header keys as an escape from its closed key set, and urzua's closed set has no such escape. | **substantive** |
> | 2026-11-04 | Built as a rule option, not a per-type config value: `header.field-set-consistency` takes `allow_extension_prefix: true` (off by default), which exempts header keys beginning `x-` in any case from that rule and nothing else. The prefix is fixed at `x-` rather than declared per type. An unprefixed undeclared key still fails, and a prefixed key is never a pointer or narrative field unless declared. **Why:** the rule's own closed set is per type, and the opt-in is the declared-not-voted choice the closed set already makes; a per-type prefix value would have been a second configuration axis with no adopter asking for one. Recorded here so the 'Done means' line's per-type wording is not read as the shipped shape. | **substantive** |
