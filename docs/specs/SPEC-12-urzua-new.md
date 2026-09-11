---
Version: '0.4'
Date: 2026-09-07
Status: Accepted
Author: '@beauwilliams'
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
   drives what an author has to fill in, with no template file needing to exist.
3. **Neither holds — refuse.** A record type with no template and a `blockquote`/`bold-list` shape
   has nothing for `new` to safely generate a header from; guessing a shape is explicitly rejected
   (this is `spec`'s own current state — MILE-74 tracks giving it a template).

**The configured `header_shape` wins unconditionally over path 1 existing** (BUG-3): if a type
declares `yaml-frontmatter` but also happens to have a leftover template file, the config's
declared shape still governs, never the template's mere presence.

## Display-number assignment

Scans the type's directory for the highest existing numeric prefix and takes the next integer —
**never reused**, even if a number was deleted, since a cross-reference elsewhere may still assume
the old numbering held (the same reasoning RFC-13 argues for vacated numbers generally, applied here
as prevention: `new` never manufactures the collision RFC-13 exists to describe after the fact).
Filenames emitted going forward carry the type's prefix (`ADR-38-slug.md`, ADR-36); legacy
`NNNN-slug.md` filenames are never renamed and resolve identically forever.

## Output

Stdout is the JSON report (ADR-23/46): `path`, `display_number`, `stable_id`, plus an optional
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
