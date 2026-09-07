# 1 — Fix urzua new's template-priority bug

> Status: Done
> Stable-Id: 01M1Y5JCCP2D9ES98WDB9V5PPF
> Phase: 0
> Track: header-format
> Implements: ADR-27, BUG-3

## What

urzua new picks blockquote vs. YAML frontmatter by whether a template file exists, ignoring the configured header_shape entirely.

## Why

Flipping any type's header_shape to yaml-frontmatter without this fix means urzua new keeps emitting blockquote content that check can't parse under the declared shape -- silently broken output.

## Blocked on

—

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Fixed and shipped: header_shape wins unconditionally over a template's own shape (BUG-3). | **substantive** |
