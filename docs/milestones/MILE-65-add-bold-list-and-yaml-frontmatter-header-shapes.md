---
Status: Done
Stable-Id: 01M1YNNE70GCPJRXKCKCFTJ8Q9
Phase: '0'
Track: header-format
Implements: ADR-16, ADR-17
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/header.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
---
# 65 — Add bold-list and yaml-frontmatter header shapes

## What

Two more `header_shape` values beyond the default `blockquote`: `bold-list` (a bold markdown list with no blockquote) and `yaml-frontmatter` (real, `serde`-deserialized YAML). Declared per record type in config, never sniffed from content.

## Why

Forcing every adopted corpus to rewrite its header convention to match Urzua's own default would violate the adopt-without-a-rewrite principle `init` exists to serve -- these are the two other real shapes this project actually encountered.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-177`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
