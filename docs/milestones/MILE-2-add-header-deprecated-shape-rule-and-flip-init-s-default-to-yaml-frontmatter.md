# 2 — Add header.deprecated-shape rule and flip init's default to yaml-frontmatter

> Status: Planned
> Stable-Id: 01M1Y5JD3YB2532B114C3ZC87P
> Phase: 0
> Track: header-format
> Implements: ADR-33

## What

A non-blocking check-rule warning when a record type's configured shape isn't yaml-frontmatter, and init proposing yaml-frontmatter for any newly-adopted type.

## Why

ADR-33 decided the deprecation; this is the mechanism that actually makes it visible and steers new adoptions, without breaking anyone still on blockquote/bold-list.

## Blocked on

Milestone: Fix urzua new's template-priority bug

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
