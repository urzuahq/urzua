# 65 — Add bold-list and yaml-frontmatter header shapes

> Status: Done
> Stable-Id: 01M1YNNE70GCPJRXKCKCFTJ8Q9
> Phase: 0
> Track: header-format
> Implements: ADR-16, ADR-17
> Blocked-on: —

## What

Two more `header_shape` values beyond the default `blockquote`: `bold-list` (a bold markdown list with no blockquote) and `yaml-frontmatter` (real, `serde`-deserialized YAML). Declared per record type in config, never sniffed from content.

## Why

Forcing every adopted corpus to rewrite its header convention to match Urzua's own default would violate the adopt-without-a-rewrite principle `init` exists to serve -- these are the two other real shapes this project actually encountered.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
