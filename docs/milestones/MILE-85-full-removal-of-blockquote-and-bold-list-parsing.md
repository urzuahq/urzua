---
Stable-Id: 01M1ZK010SZ6Q9A8W8RKSR1J7V
Status: Planned
Phase: '2'
Track: header-format
Blocked-on: real adoption data on how much the deprecated shapes are still in use elsewhere
---
# 85 — Full removal of blockquote and bold-list parsing

## What

Remove `header::parse_with_shape`'s `Blockquote`/`BoldList` branches, the `HeaderLayout` enum, and
`header.layout-consistency` entirely, once `yaml-frontmatter` is the only shape any real corpus --
this one or an adopter's -- still needs `check` to read.

## Why

ADR-33 named this as a real, deferred decision the moment it deprecated `blockquote`/`bold-list`
("a future major-version step, decided separately, once real adoption data exists on how much the
deprecated shapes are still actually in use") but never tracked it as an actual milestone. Filed now,
alongside MILE-3's completion, so the deferred half of that decision has a record of its own instead
of only living in ADR-33's prose.

This repo's own corpus has zero real records left in the deprecated shapes after MILE-3 -- but
parsing support must stay for two reasons that don't disappear on that fact alone: an external
adopter's un-migrated corpus needs to stay readable, and `check` must still run against a first-time
evaluator's existing, unmodified docs before they decide whether to adopt anything at all (ADR-33's
own stated reason for deprecating rather than removing). Neither of those depends on this repo's own
state, so this milestone can't move to `Done` on this repo's evidence alone -- it needs the adoption
data ADR-33 itself names as the actual gate, which doesn't exist yet.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial milestone. **Why:** ADR-33 named full removal as an explicit non-decision, deferred until real adoption data exists, but never tracked it as an actual milestone -- filed here alongside MILE-3's completion. | **structural** |
