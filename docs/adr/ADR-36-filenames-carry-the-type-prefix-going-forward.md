---
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/rules.rs, code:rust/crates/urzua-core/src/new_record.rs, code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-core/src/rules.rs, test:rust/crates/urzua-core/src/new_record.rs
Date: 2026-09-07
Author: '@beauwilliams'
Deciders: '@beauwilliams'
Supersedes / Superseded-by: —
Derives-from: ADR-3, ADR-34
---
# 36 — Filenames carry the type prefix going forward; legacy names stay valid forever

## Context

BUG-2 already removed the actual defect in numbering (any digit-length filename prefix resolves,
matching is by numeric value not exact string) — what remained was a considered style question,
raised directly: should a record's filename carry its type prefix explicitly
(`ADR-36-slug.md`), rather than relying on directory context the way `docs/adr/0036-slug.md`
does today? And should zero-padding and the human-readable slug stay, now that neither is required
for correctness?

## Decision

In the context of a filename convention question with no remaining correctness defect behind it,
we decided on three points together:

- **Type prefix moves into the filename**: `TYPE-NNNN-slug.md` is the default `urzua new` emits
  going forward. This matches how a record is already referenced everywhere else in this corpus —
  `Implements: ADR-36` is the same string as the file's own name, not a translation of it.
- **Zero-padding stays**, despite no longer being required for parsing. A plain `ls`, `git log`, or
  GitHub's file browser all sort lexicographically, not numerically — padding is what keeps that
  sort order matching creation order once past 9 or 99 records of one type.
- **The slug stays.** Genuinely useful for at-a-glance readability in a file listing without
  opening the file, and for any future generated web view. Staleness against the title is already
  a solved problem (`filename.title-consistency`), not a new cost this decision introduces.
- **Legacy `NNNN-slug.md` filenames are never renamed and never stop working** — the same
  non-disruptive precedent ADR-33 already set for header shapes: parsing accepts both forms
  permanently, `record_id`/`filename_number` try the type-prefixed form first, then fall back to
  the legacy form. This repo's own existing 95 records are untouched by this decision.

A second, independently-found instance of BUG-2's exact defect class was fixed in the same
pass: `filename_title_consistency`'s own `filename_number`/`title_number` helpers had their own
separate hardcoded 4-digit checks, missed when `record_id` was fixed. Both now accept any digit
length and compare by numeric value, matching `record_id`/`normalize_id`'s already-established
approach.

## Reversibility

Additive and non-disruptive: existing filenames are untouched, both shapes parse identically
forever. Only what `urzua new` emits for brand-new records changes.

## Consequences

- `docs/adr/`, `docs/rfc/`, etc. will hold a mix of legacy and type-prefixed filenames indefinitely
  — this is expected, not a migration-in-progress state to eventually finish.
- `next_display_number` reads both shapes to find the true max across a mixed directory, so
  numbering never collides regardless of which shape any individual file happens to use.
- A mass rename of this repo's own 95 existing records to the new shape is explicitly not decided
  here — possible later, purely cosmetic, never required.

## Amendment (2026-09-07): zero-padding reversed

The padding half of this decision is reversed. Re-examined directly: fixed-width padding does not
solve the lexicographic-sort problem it was chosen for, it only defers the point where it breaks —
once any one type crosses the padded width (e.g. a 5-digit `ADR-10000` next to four-digit
`ADR-1`..`ADR-9999`), the directory is *still* lexicographically out of order, and now with
mixed-width names, which is a worse break than the one padding was meant to prevent. There is no
width that makes this permanent; every fixed width just picks when the break happens.

Unpadded numbers don't solve lexicographic sort either, but they never claimed to — a plain `ls`
has never sorted `ADR-2` before `ADR-10`, padded or not, past two digits without leading zeros
matching. Padding's actual value was cosmetic consistency at small scale, at the cost of an
eventual, harder break. Not worth it.

Decision, superseding the padding bullet above: **no zero-padding, anywhere** — filenames
(`ADR-36-slug.md`, not the four-digit `ADR-0036-slug.md`) and each record's own H1 heading both show the bare
number. The type-prefix-in-filename, slug-stays, and legacy-names-stay-valid-forever bullets are
unchanged. `urzua new` emits unpadded numbers as of this amendment; existing records are renamed
separately, in one pass across the whole corpus, tracked as its own piece of work rather than
folded silently into this note.

## References

- BUG-2 — the defect this decision's remaining scope is purely stylistic on top of.
- ADR-33 — the non-disruptive-deprecation precedent this decision follows exactly.
- ADR-3 — display-number-is-identity, unchanged by this decision.
- `rust/crates/urzua-core/src/rules.rs` — `record_id`, `filename_number`, `title_number`.
- `rust/crates/urzua-core/src/new_record.rs` — `next_display_number`.
- `rust/crates/urzua-cli/src/main.rs` — `run_new`'s filename construction.
