---
Stable-Id: 01M6HZ0P342ZYCS29CZ780Y9EV
Status: InProgress
Phase: '0'
Track: schema-governance
Implements: RFC-33
Realized-by: code:rust/crates/urzua-core/src/rules/header.rs#header_field_set_consistency@sha256:c8120dc6ffdad9fde0d1372995277e258f5feb73215aae42df21c5f0305963ff, test:rust/crates/urzua-core/src/rules/header.rs#an_x_prefixed_key_is_exempt_when_the_option_is_declared_observed_failing@sha256:c332d770ee0b2a9895414ca373f0a62e3556722afbfd28f8df28d03031cec1a1, test:rust/crates/urzua-core/src/rules/header.rs#an_unprefixed_unknown_key_stays_a_finding_when_the_option_is_declared@sha256:a2c6f70fb75fefe39e69c6ea75c43c5c412aa1bf9a9df0a7fa1436ebb120af7f, test:rust/crates/urzua-core/src/rules/header.rs#an_x_prefixed_key_is_a_finding_when_the_option_is_absent_or_false@sha256:c7fd9325b8f98e0aee9a2bd521a72bc110b6c0f7cf4e38210c42d56e93c597bd, spec:docs/adr/ADR-44-pointer-and-narrative-fields-are-config-declared-per-type-not-hardcoded.md
---
# 141 — An x- prefix escape for the closed header-key set

## What

Let a type's closed header-key set admit extension keys by a declared prefix, `x-` by convention, so a repository can add a local field without declaring it in `known_fields` and without weakening the closed set for every other key.

## Why

`header.field-set-consistency` treats the union of `required_fields` and `known_fields` as closed, so every local extension has to be added to config before it is used. The adopter's linter closes its key set but accepts any key with an `x-` prefix, which lets a team annotate a record without a config change and still catches a typo in a real key. Without an equivalent, the adopter either declares every extension up front or loses typo detection.

## Done means

- A header key with the declared prefix passes the field-set rule and an unprefixed unknown key still fails it. — evidence: code:rust/crates/urzua-core/src/rules/header.rs#header_field_set_consistency@sha256:c8120dc6ffdad9fde0d1372995277e258f5feb73215aae42df21c5f0305963ff, test:rust/crates/urzua-core/src/rules/header.rs#an_x_prefixed_key_is_exempt_when_the_option_is_declared_observed_failing@sha256:c332d770ee0b2a9895414ca373f0a62e3556722afbfd28f8df28d03031cec1a1, test:rust/crates/urzua-core/src/rules/header.rs#an_unprefixed_unknown_key_stays_a_finding_when_the_option_is_declared@sha256:a2c6f70fb75fefe39e69c6ea75c43c5c412aa1bf9a9df0a7fa1436ebb120af7f
- The escape is absent by default. — evidence: test:rust/crates/urzua-core/src/rules/header.rs#an_x_prefixed_key_is_a_finding_when_the_option_is_absent_or_false@sha256:c7fd9325b8f98e0aee9a2bd521a72bc110b6c0f7cf4e38210c42d56e93c597bd
- The prefix is a per-type config value.
- A prefixed key is not treated as a pointer or narrative field unless declared. — evidence: code:rust/crates/urzua-core/src/rules/header.rs#header_field_set_consistency@sha256:c8120dc6ffdad9fde0d1372995277e258f5feb73215aae42df21c5f0305963ff, spec:docs/adr/ADR-44-pointer-and-narrative-fields-are-config-declared-per-type-not-hardcoded.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter accepts `x-` prefixed header keys as an escape from its closed key set, and urzua's closed set has no such escape. | **substantive** |
> | 2026-11-04 | Built as a rule option, not a per-type config value: `header.field-set-consistency` takes `allow_extension_prefix: true` (off by default), which exempts header keys beginning `x-` in any case from that rule and nothing else. The prefix is fixed at `x-` rather than declared per type. An unprefixed undeclared key still fails, and a prefixed key is never a pointer or narrative field unless declared. **Why:** the rule's own closed set is per type, and the opt-in is the declared-not-voted choice the closed set already makes; a per-type prefix value would have been a second configuration axis with no adopter asking for one. Recorded here so the 'Done means' line's per-type wording is not read as the shipped shape. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #168: `header.field-set-consistency` takes `allow_extension_prefix`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Scope narrowed explicitly; `Status` stays Done. **Why:** the closure audit of #234 found the shipped shape differs from the 'Done means' wording. Delivered: the per-rule option `allow_extension_prefix` on `header.field-set-consistency`, off by default, exempting header keys that begin `x-` (any case); an unprefixed unknown key still fails; a prefixed key is not a pointer or narrative field unless declared. Not delivered: a prefix declared per type in configuration. The 2026-11-04 row above records the deliberate choice (a per-type prefix would be a second configuration axis no adopter asked for), so the per-type prefix is outside this milestone's scope and no follow-up is filed until an adopter needs a prefix other than `x-`. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | `Status` `Done` -> `InProgress`. Status set from its Done-means items (ADR-96, MILE-215): unresolved item 3. **Why:** the item has no evidence clause, and `DoneWithUnresolvedItems` forbids `Done` while an item is unresolved; `fix` never downgrades, so the status is set by hand to the computed value. | **structural** |
> | 2026-11-07 | Reviewed in the reopened-milestone pass; Done means item 3 ("The prefix is a per-type config value") stays unresolved. **Why:** the shipped shape is the rule option `allow_extension_prefix` with the prefix fixed at `x-` (see the 2026-11-04 rows), no ADR or RFC requires a per-type prefix, and building one is a configuration-axis design decision for the maintainer. The item text is unchanged and no evidence is claimed. | **structural** |
