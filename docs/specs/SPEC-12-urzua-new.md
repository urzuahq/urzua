---
Version: '0.8'
Date: 2026-09-07
Status: Accepted
Author: beauwilliams
Subject: '`urzua new` -- generating a record''s initial content with a real stable ID assigned.'
Implements: ADR-27
Parent: SPEC-1
---
# SPEC-12 — `urzua new`

## Purpose

Creates a record from the configured template with a stable ID assigned. Never asks the author to
pick a number. Fills in only what the tool can compute; every decision a human makes stays the
template's own placeholder text for a human to pick.

## The three paths, in priority order

1. **A checked-in template exists (`.urzua/templates/<type>.md`) — fill it in.** The H1's number and
   title, `Date`, `Author` (resolved the same way `fix --apply` resolves an identity: `gh api user`
   wins even over an explicit `--by`, which wins over `git config user.name` — SPEC-8's identity
   tiering, reused rather than redesigned), and a freshly generated `Stable-Id` line inserted right after the
   header's first line. Everything else — `Status: Proposed | Accepted | ...`, `Deciders`, an
   `Embodiment` starting value — stays exactly the enumerated placeholder text already in the
   template, because those are decisions a human makes, not values `new` can compute.
2. **No template, but the type declares `header_shape = "yaml-frontmatter"` — synthesize one** from
   `required_fields`: every configured field emitted as a blank frontmatter key, so the config alone
   drives what an author has to fill in, with no template file needing to exist. Two fields are not
   blank: `Date` and `Author`, which `new` computes, and the type's status field, which starts at the
   first value of the type's declared `statuses.values` (`MILE-151`). That is the vocabulary's entry
   state rather than a decision, so moving a record on from it stays a human's; a type that declares
   no `statuses` leaves the field blank.
3. **Neither holds — refuse.** A record type with no template and a `blockquote`/`bold-list` shape
   has nothing for `new` to safely generate a header from; guessing a shape is explicitly rejected
   (this is `spec`'s own current state — MILE-74 tracks giving it a template).

**The configured `header_shape` wins unconditionally over path 1 existing** (BUG-3): if a type
declares `yaml-frontmatter` but also happens to have a leftover template file, the config's
declared shape still governs, never the template's mere presence.

## The declared identity pattern and field set (`MILE-151`)

`new` reads the same declarations `check` does, so a generated record passes the type's own rules
without an edit:

- **Identity.** A type that declares `identity` (`SPEC-3`) is numbered by that pattern, not by the
  `TYPE-NNNN-slug.md` convention: the next number is one past the highest `number` the pattern reads
  from a file in the directory. The file name is `PREFIX-NUMBER-slug.md` with the number zero-padded
  to the narrowest width the pattern accepts (`\d+` takes none, `\d{4}` takes four), and the H1 opens
  `PREFIX-N` with the unpadded number, the title form the declaration implies (`RFC-44`). A regular
  expression cannot be run backwards, so the name is kept only if the pattern reads it back as the
  type's prefix and the number; a pattern no such name fits is a refusal naming the pattern, before
  anything is written. A type with no `identity` is numbered and named exactly as before.
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

- **A fourth fallback** — synthesizing a `blockquote` header from `required_fields` alone, the way
  the YAML path does, for a type with neither a template nor a YAML declaration. Plausible, not
  ruled out, not needed by any type this repo currently declares.
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
