---
Version: '0.13'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/new.rs, code:rust/crates/urzua-core/src/new_record.rs, test:rust/crates/urzua-cli/tests/new_integration.rs
Author: beauwilliams
Subject: '`urzua new` -- generating a record''s initial content with a real stable ID assigned.'
Implements: ADR-27
Parent: SPEC-1
---
# SPEC-12 — `urzua new`

## Purpose

Creates a record from the type's configuration with a stable ID assigned. Never asks the author to
pick a number. Fills in only what the tool can compute; every decision a human makes stays the
template's own placeholder text for a human to pick.

## How a record is built

The header is always synthesized as YAML frontmatter from the type's `required_fields`, the only
header shape `new` writes (the `blockquote` and `bold-list` shapes were removed, `MILE-85`): every
configured field is emitted as a blank frontmatter key, so the config alone drives what an author
has to fill in. Fields `new` can compute are filled: a freshly generated `Stable-Id`, the date and
author fields (`relation_fields.date`/`author`, default `Date`/`Author`; the author resolved the same way `fix --apply` resolves an identity: `gh api user` wins even over an
explicit `--by`, which wins over `git config user.name` -- SPEC-8's identity tiering, reused rather
than redesigned), and the type's status field, which starts at the first value of the type's
declared `statuses.values` (`MILE-151`). That is the vocabulary's entry state rather than a
decision, so moving a record on from it stays a human's; a type that declares no `statuses` leaves
the field blank. Every other field is a decision a human makes and stays blank.

If a checked-in template exists (`.urzua/templates/<type>.md`), its body scaffolding (the H1 and the
sections beneath the header) is appended under the synthesized header; the template's own header, in
whatever shape, is never used as the header, but the field names it lists are read (see below) (`BUG-3`: the configured `header_shape` wins unconditionally over a
leftover template).

A type declaring `header_shape: none` has nowhere to write a `Stable-Id`, so `new` refuses it
(`ADR-50`).

## The declared identity pattern and field set (`MILE-151`)

`new` reads the same declarations `check` does, so a generated record passes the type's own rules
without an edit:

- **Identity.** A type that declares `identity` (`SPEC-3`) is numbered by that pattern, not by the
  `TYPE-NNNN-slug.md` convention: the next number is one past the highest `number` the pattern reads
  from a file in the directory. The file name is `PREFIX-NUMBER-slug.md` with the number zero-padded
  to the narrowest width the pattern accepts (`\d+` takes none, `\d{4}` takes four), and the H1 opens
  `PREFIX-N` with the number padded to that same width (`ADR-0044`), derived from the file name written
  beside it so the two ids agree (`BUG-188`); the title form the declaration implies (`RFC-44`). A regular
  expression cannot be run backwards, so the name is kept only if the pattern reads it back as the
  type's prefix and the number; a pattern no such name fits is a refusal naming the pattern, before
  anything is written. A type with no `identity` is numbered and named exactly as before.
- **Template fields.** A field the template's header names beyond the type's `required_fields` is written
  blank when the type declares it (`known_fields`, or any field when none is declared), so the record starts
  with the fields the template meant it to carry (`BUG-188`). A template field the type does not declare is
  left out, since it would only trip `header.field-set-consistency`.
- **Field set.** `Stable-Id` is always written (ADR-21: every record type gets one). When the type
  declares a closed field set (`known_fields`) that does not list it, the declaration contradicts
  ADR-21 and `header.field-set-consistency` will report the new record; `new` does not hide that by
  omitting the identity. It writes the `Stable-Id` anyway and adds a warning notice naming the
  inconsistency and the fix (add `Stable-Id` to the type's `known_fields`).

## Display-number assignment

Scans the type's directory for the highest existing numeric prefix and takes the next integer —
**never reused**, even if a number was deleted, since a cross-reference elsewhere may still assume
the old numbering held (the same reasoning RFC-13 argues for vacated numbers generally, applied here
as prevention: `new` never manufactures the collision RFC-13 exists to describe after the fact).

**Other refs are consulted too (MILE-89).** After the working-tree scan, `new` also lists the type's
directory on every local branch and remote-tracking ref (`refs/heads/*`, `refs/remotes/*`) and takes
one past the highest number found anywhere. Each number above the working tree's own highest that
another ref already uses is reported on stderr with the ref(s) claiming it. This is git only, with
no network and no `gh`: **a ref that was never fetched into this clone is invisible**, so a branch
or PR known only to the remote can still collide -- run `git fetch` first, and still check open PRs
before opening one. If git cannot be read (git missing, `.git` unusable), `new` numbers from the
working tree exactly as before and says so on stderr; it never silently skips the check. The query is
one `for-each-ref` plus one `ls-tree` per distinct commit: it reads a single directory tree per tip
and knows which ref each name came from, where `git log --all --name-only` walks all history and
cannot attribute.

Filenames emitted going forward carry the type's prefix (`ADR-38-slug.md`, ADR-36); legacy
`NNNN-slug.md` filenames are never renamed and resolve identically forever.

A title that yields an empty filename slug (empty, whitespace, or only non-ASCII characters) is an
error, reported before anything is written and without consuming a number: a slug-less filename is not
one the numbering scan recognises, so the number would be reissued (`BUG-167`).

## Output

Stdout is the JSON report (ADR-23/46): `path`, `display_number`, `stable_id` (always populated), plus an optional
`notices` array (ADR-46) carrying non-fatal observations -- e.g. an explicit `--by` diverging from
the identity `resolve_identity` actually used -- omitted entirely when empty. `new` still does not
echo the generated content back; the caller reads the file at `path` if it needs to.

## Purity boundary

`urzua-core::new_record` is the pure render layer (`render_from_template`, `render_synthetic_yaml`),
following the same pure-render/impure-caller split as `fix.rs`: the CLI resolves identity, reads the
template file, scans the directory for the next number, and writes; `new_record`'s own functions
take content and parameters, returning a string.

## What's deliberately not built

- **Any header shape other than YAML frontmatter.** The `blockquote` and `bold-list` shapes were
  removed (`MILE-85`), so there is no other shape for `new` to write.
- **Interactive prompting** for `Status`/`Deciders`/etc. — `new` is a one-shot, scriptable command;
  a human or agent edits the placeholder text afterward in whatever tool they're already using.

## References

- ADR-27 — the decision this spec details: what `new` fills in versus leaves as placeholder.
- ADR-3/21 — the stable-ID model `new` assigns from on creation.
- ADR-36 — type-prefixed filenames going forward, legacy names unaffected.
- SPEC-1 — `new`'s original one-line contract this spec fills in the mechanism for.
- SPEC-8 — the identity-resolution tiering `new` reuses from `fix --apply`.
- BUG-3 — the template-priority defect this spec's "configured shape wins unconditionally" rule
  fixes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec. **Why:** MILE-77 found `new` documented only as an ADR while comparable-complexity command areas (`check`, `init`) had specs. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-09-11 | §Output's "No other fields" is now false: `notices` (ADR-46) can carry a non-fatal identity-divergence observation. **Why:** this spec's own literal wording would otherwise contradict the shipped output the moment `--by` diverges from an authenticated `gh` login. | **substantive** |
> | 2026-09-11 | Corrected §"The three paths"' identity-resolution order, stale since ADR-31's amendment (`gh api user` wins even over `--by`, which wins over `git config user.name` -- not "`--by` wins outright" as this spec still said). **Why:** caught by review before merge; the same stale order SPEC-8 was already corrected for elsewhere in this same change. | **substantive** |
> | 2026-11-02 | §Display-number assignment now also skips numbers used on other local branches and remote-tracking refs, naming them on stderr, and documents that unfetched refs are invisible (`MILE-89`). Bumped to `0.5`. **Why:** `urzua new` re-assigned numbers already claimed by open PR branches. | **substantive** |
> | 2026-11-02 | Bumped to `0.6`. A title with no slug is refused. **Why:** `new` wrote a slug-less `ADR-N-.md` that its own numbering scan ignored, so the next `new` reissued `N` (`BUG-167`). | **substantive** |
> | 2026-11-04 | Bumped to `0.7`. A type's declared `identity` now decides numbering, the file name's padding and the H1's `PREFIX-N` form; the status field starts at the first declared status value instead of a literal `null`; `Stable-Id` is left out (report `stable_id: null`, warning notice) when a closed `known_fields` does not list it. A pattern `new` cannot name a file for is refused. **Why:** `MILE-151` and `BUG-182` -- the generator and the checker read two grammars, so its output failed `filename.identity-pattern` and `field.quality`. | **substantive** |
> | 2026-11-04 | Bumped to `0.8`. `Stable-Id` is always written and the report's `stable_id` always populated; a closed `known_fields` that omits it yields a warning notice naming the inconsistency instead of an omitted identity. **Why:** `MILE-151` omitted it, contradicting ADR-21 (every record type gets a Stable-Id) and leaving a record with no identity. | **substantive** |
> | 2026-11-04 | Bumped to `0.9`. `new` always synthesizes a YAML frontmatter header and appends a template's body scaffolding when one exists; the "fill the template in" path for a `blockquote` or `bold-list` template is gone. **Why:** `MILE-85` removed those shapes. | **substantive** |
> | 2026-11-04 | Bumped to `0.10`. The H1 id is padded to the file name's width; the header fields a template names that the type declares are kept, blank. **Why:** `BUG-188` -- an unpadded H1 beside a padded file name, and optional template fields dropped. | **substantive** |
> | 2026-11-04 | Bumped to `0.11`. The fields filled with the date and the author are the type's `relation_fields.date` and `relation_fields.author` (default `Date`, `Author`), not literals. **Why:** `MILE-176`. | **substantive** |
> | 2026-11-05 | Bumped to `0.12`. Realized-by names the implementing code and tests, ready for `Shipped` (`MILE-7`). **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/commands/new.rs, code:rust/crates/urzua-core/src/new_record.rs, test:rust/crates/urzua-cli/tests/new_integration.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Bumped to `0.13`. Status changed from Accepted to Shipped. **Why:** Maintainer decision, 2026-11-05: its Realized-by evidence computes Verified and an independent review judged it sound. | **substantive** |
