---
Stable-Id: 01M6M5FASC120FA01WG74QWH7A
Status: Planned
Phase: '0'
Track: embodiment-model
Implements: ADR-67
Blocked-on: ADR-67 (Proposed; the pin syntax this extends)
---
# 185 — Evidence pins can be scoped to one symbol or line range so unrelated edits to a volatile file stop going stale

## What

A pinned `Realized-by` locator can name one item in a file, so the pin hashes only that item. A candidate
shape is `code:path#fn_name@sha256:<hex>` for a named symbol (hashing that function, type or `impl`
block) and `code:path#L10-L40@sha256:<hex>` for a line range. A file-level `code:path@sha256:<hex>` keeps
working unchanged. Drift, the grace window and `fix --rebaseline` treat a scoped pin like any other pin;
only the bytes hashed differ. The syntax is hard to reverse, so it is first decided in an ADR that
amends `ADR-67` (the maintainer's decision); this milestone is the build after that.

## Why

`ADR-67` pins a locator to the hash of the whole file. That is exact, but the files worth pinning are
the most edited ones, and a decision is almost always about one function in them. Today's evidence:

- The 14 records pinned in #180 (`ADR-18`, `22`, `24`, `26`, `30`, `32`, `35`, `36`, `38`, `39`, `43`,
  `44`, `57`, `SPEC-1`) were all reported stale within about nine hours of being pinned, on one working
  day (the drift review of 2026-11-04).
- Eleven of the 14 needed nothing but a re-pin: the changes to the cited file did not touch what the
  decision describes (fixtures converted to YAML frontmatter, a struct default, a new option on a
  sibling rule, one added line in `audit.rs`). The other three (`ADR-32`, `ADR-57`, `ADR-38`) had a real
  divergence in the cited code, which a scoped pin would still have caught because the changed items
  were the ones the decision is about.
- The churn sits in a handful of volatile files: `config.rs` took 33 commits after #180 and
  `check_integration.rs` 12; `main.rs`, `embodiment.rs` and `.urzua/config.yaml` also moved. A file-level
  pin on a file that busy goes stale again on the next unrelated commit, however often it is re-baselined.
- Each stale pin costs a reviewer a re-verification and a revision row for a change that does not bear
  on the decision, which trains the review into a rubber stamp: the opposite of what a pin is for.

Related records. `RFC-5` defines a locator as a file or glob and has no sub-file form. `ADR-67` chose the
narrowest pin syntax (a suffix on a locator) and lists pins as a separate field as Option C; it does not
consider a narrower hashed unit, so nothing already proposes this. `MILE-84` (Phase 2, Planned) proposes
line- and commit-anchored locators (`path@<short-sha>:<line>`) so a locator survives a refactor and can
cite generated or config files; that is a pointer to a site, not a content hash, and does not stop a pin
going stale. The two should share one locator grammar, so this milestone's ADR must be written with
`MILE-84`, not beside it.

## Done means

A pin written as `code:<path>#<symbol>@sha256:<hex>` (and as `#L<a>-L<b>`) is accepted by the locator
grammar and drifts only when the hash of that item changes: an edit to another function in the same file
is not drift, an edit to the named item is, and a symbol or range that no longer resolves in the file is
reported by `embodiment.locator-exists` rather than passing silently. `urzua fix --rebaseline` lists,
re-pins and attributes a scoped pin exactly as it does a file-level one, and a file-level pin behaves
as before. Symbol resolution is configuration-driven, not a hardcoded language list (`ADR-64`), and a
file type with no resolver reports that the scoped form is unsupported for it. It is covered by tests
over both forms, and exercised in this repository: the pins on `config.rs`, `check_integration.rs`,
`main.rs`, `embodiment.rs` and `.urzua/config.yaml` are narrowed to the items their decisions are about,
and an unrelated edit to each of those files leaves `urzua check` at zero unwaived findings, also with
`drift_grace_days: 0`. `SPEC-2`, `SPEC-3` and `SPEC-8` document the form, and the deciding ADR is
Accepted.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** 14 pins set in #180 were stale within a working day and eleven of them needed only a re-pin, because file-level `@sha256:` pins on volatile files go stale on edits unrelated to the decision; a pin scoped to the cited symbol or line range would have stayed current. Proposed from the drift review of 2026-11-04; the locator syntax is not decided here. | **structural** |
