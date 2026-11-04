---
Stable-Id: 01M1ZK010SZ6Q9A8W8RKSR1J7V
Status: Done
Phase: '0'
Track: header-format
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/header.rs, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-a/expectations/check-removed-bold-list-config-is-rejected.yaml
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
data ADR-33 itself names as the actual gate, which did not exist when this milestone was filed (see
the 2026-11-02 revision-log row).

## Decision (2026-11-04)

The maintainer decided on full cleanup at switchover. The `bold-list` and `blockquote` header shapes
are dropped, and the switchover (`MILE-102`) rewrites the adopter's headers to the canonical
`yaml-frontmatter` shape rather than urzua parsing the deprecated shapes through adoption. This
supersedes the earlier reading in this record that parsing support must stay for an un-migrated
external corpus: the one real corpus in view is converted once, in the switchover.

The dependency is a header rewriter. It is a one-off script written and run inside the switchover,
the same way the filename rename is, not a shipped command; `MILE-23` and `MILE-65` (a reusable
header-shape migration) are not on the Phase 0 path. Removal of the parser branches follows the
rewrite: the adopter corpus must read as `yaml-frontmatter` and pass `check` before the
`Blockquote`/`BoldList` branches, `HeaderLayout` and `header.layout-consistency` are deleted. The
phase moves from 2 back to Phase 0, since the switchover is the Phase 0 exit.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial milestone. **Why:** ADR-33 named full removal as an explicit non-decision, deferred until real adoption data exists, but never tracked it as an actual milestone -- filed here alongside MILE-3's completion. | **structural** |
> | 2026-11-02 | Noted that this milestone's precondition, "once `yaml-frontmatter` is the only shape any real corpus still needs", is now contradicted: a real adopter corpus of several hundred records uses bold-list headers and bare zero-padded numeric filenames, so the precondition is not met; whether to keep, defer, or close this milestone is a maintainer decision, and `Status` and `Phase` are unchanged. **Why:** the adoption data `ADR-33` named as the gate has arrived and points the other way. | **substantive** |
> | 2026-11-04 | Recorded the maintainer decision of full cleanup at switchover: the bold-list and blockquote header shapes are dropped, and the switchover rewrites the adopter's headers to the canonical shape through a one-off header rewriter. Phase changed from 2 to 0 and `Blocked-on` now names that rewriter; `Status` unchanged. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the switchover is the Phase 0 exit, and it removes the precondition (an adopter still needing the deprecated shapes) that kept this deferred. | **substantive** |
> | 2026-11-04 | Removed the `Blockquote` and `BoldList` branches of `header::parse_with_shape`, the `HeaderLayout` enum and `header.layout-consistency`, plus `header.deprecated-shape` (which can no longer fire) and the `header_layout` option; `header_shape` now defaults to `yaml-frontmatter`. The unsupported forms fail loudly rather than silently: a config declaring `header_shape` or `fields.from` `blockquote`/`bold-list` fails to load naming the form, and a record still written in one is reported unreadable by `header.required-fields` with the form named. `Status` unchanged. **Why:** maintainer instruction of 2026-11-04 to carry out the full-cleanup decision above; this repository's own corpus and tests were already `yaml-frontmatter`, and the fixtures that exercised the removed forms were migrated or rewritten to assert the error. Specs `SPEC-2` 0.44, `SPEC-3` 0.32, `SPEC-5` 0.7, `SPEC-10` 0.6, `SPEC-12` 0.9. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #205: the blockquote and bold-list branches, `HeaderLayout` and `header.layout-consistency` are gone; a removed form fails loudly. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | `Blocked-on` corrected from the adopter-side header rewriter to none; `Status` stays Done. **Why:** the closure audit of #234 found a Done record still naming a blocker. This milestone's Done-when, removing the parser branches, `HeaderLayout` and `header.layout-consistency`, is met in this repository. The one-off header rewrite of the adopter's corpus is not this milestone's delivery: it moved to `MILE-102` (Phase 1), whose switchover list names it. | **substantive** |
