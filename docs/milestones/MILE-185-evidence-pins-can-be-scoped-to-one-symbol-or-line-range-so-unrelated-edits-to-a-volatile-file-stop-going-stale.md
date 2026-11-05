---
Stable-Id: 01M6M5FASC120FA01WG74QWH7A
Status: Planned
Phase: '0'
Track: embodiment-model
Implements: ADR-74
Blocked-on: —
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

A pin written as `code:<path>#<symbol>@sha256:<hex>` (and as `#L<a>-L<b>`), where a symbol is a bare item, a member of a type (`Type::member`, found across all of the type's inherent `impl` blocks and unique across them) or a member of a trait impl (`Type::<Trait>::member`) as `ADR-74` specifies, is accepted by the locator
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
> | 2026-11-04 | `Blocked-on` cleared. **Why:** it named `ADR-67` as Proposed, and the maintainer accepted it on 2026-11-04. The scoped-pin syntax still needs its own deciding ADR, as the What says. | **structural** |
> | 2026-11-04 | Deciding ADR drafted: `ADR-74` (Proposed) sets out the scoped-locator grammar, declared symbol resolvers, the shared slot for `MILE-84`'s commit anchor and the reporting of a scope that stops resolving, with six open questions. `Status` and `Blocked-on` unchanged. **Why:** the What says the syntax needs its own deciding ADR before this milestone is built; the maintainer accepts or amends it. | **substantive** |
> | 2026-11-04 | Done means extended: a symbol may be a member of a type or of a trait impl, and a commit-anchored locator is reported as not yet supported. **Why:** the maintainer chose to include methods in the first build of `ADR-74` and its amendment specifies how a member is found across several `impl` blocks. `Status` unchanged. | **substantive** |
> | 2026-11-04 | `Implements` set to `ADR-74`, the deciding ADR (Accepted). First of three PRs built: the locator grammar and parse, declared resolvers and `resolve_scope`, the hashed bytes, every failure reason reported by `embodiment.locator-exists`, the normative vectors in `SPEC-3`, and the Rust and YAML resolvers in this repository's config. Drift, `fix --rebaseline`, `explain` and the corpus migration follow. `Status` unchanged. **Why:** `ADR-74` sets this milestone's build order. | **structural** |
> | 2026-11-05 | Second of three PRs built: scoped pins drift on their span's hash with no file-based grace window, `fix --rebaseline` re-pins them from the span and lists unresolvable ones as `failed_pins`, and `explain <path>#<scope>` is the authoring route; `SPEC-2`, `SPEC-8` and `SPEC-13` document it. The corpus migration remains. `Status` unchanged. **Why:** `ADR-74` points 6 to 8. | **structural** |
> | 2026-11-05 | Third of three PRs built: the file-level pins on the volatile files `embodiment.rs`, `main.rs`, `check_integration.rs`, `config.rs` and `.urzua/config.yaml` are narrowed to scoped pins for `ADR-18`, `ADR-26`, `ADR-30`, `ADR-35`, `ADR-39`, `ADR-43`, `ADR-44` and `SPEC-1` (recommendation (a) of the drift review of 2026-11-04), and an unrelated edit to each of those five files leaves `urzua check` with no finding on those records, also with `drift_grace_days: 0`. `ADR-32` (`embodiment.rs`) and `ADR-42` (`.urzua/config.yaml`) keep their file-level pins: the review found their decisions diverge from the code (recommendation (b), amend), and narrowing a pin would certify what the review did not. `Status` stays `Planned` until those two are amended and narrowed. **Why:** `Done means` names the pins on these five files; two are held for the maintainer's amend decision. | **structural** |
> | 2026-11-05 | Review follow-up: the narrowing rows no longer claim an attestation by the maintainer; the re-pin reasons on `ADR-39`, `ADR-43`, `ADR-44` and `SPEC-1` now state the actual change since each old pin; `ADR-35`'s row says its test locator was replaced; `ADR-18` also pins `parse_realized_by`; and `ADR-22`, `ADR-24` and `ADR-36` are re-baselined under recommendation (a) with per-record reasons. Rust resolver: `const`, `static`, `type` and `mod` no longer start a brace-balanced span. `Status` unchanged. **Why:** review of the three `MILE-185` PRs. | **structural** |
