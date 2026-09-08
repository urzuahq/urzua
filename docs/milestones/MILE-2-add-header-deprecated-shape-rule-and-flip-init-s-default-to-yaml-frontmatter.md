---
Status: Done
Stable-Id: 01M1Y5JD3YB2532B114C3ZC87P
Phase: '0'
Track: header-format
Implements: ADR-33
Blocked-on: —
---
# 2 — Add header.deprecated-shape rule and flip init's default to yaml-frontmatter

## What

A non-blocking check-rule warning when a record type's configured shape isn't yaml-frontmatter, and init proposing yaml-frontmatter for any newly-adopted type.

## Why

ADR-33 decided the deprecation; this is the mechanism that actually makes it visible and steers new adoptions, without breaking anyone still on blockquote/bold-list.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Cleared `Blocked-on` (was `BUG-3`, `Status: Fixed` for a while, caught by `blocked-on.stale` -- MILE-83 -- immediately after `Blocked-on` moved from prose into a checked header field). **Why:** the blocker had silently resolved and nobody had revisited this milestone since; found live during backlog triage, confirmed by the new rule firing on this exact case before this fix. | **substantive** |
> | 2026-09-08 | `Status: Done`. Built `header.deprecated-shape` (config-level, one warning per non-yaml-frontmatter type) and `init`'s adopt-mode default (`header_shape = "yaml-frontmatter"` on every proposed type, regardless of the corpus's detected shape). Verified: the planted-violation tests in `rules.rs`, and `init_then_check_flags_a_pre_existing_blockquote_record` observing the real, ADR-33-accepted consequence of that default against a still-blockquote corpus. | **substantive** |
