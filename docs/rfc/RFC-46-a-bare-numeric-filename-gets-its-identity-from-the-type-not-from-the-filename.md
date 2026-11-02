---
Stable-Id: 01M6D5B37EB2FEZEBNP95GPTCA
Status: Rejected
Date: 2026-11-02
Author: beauwilliams
---
# 46 — A bare numeric filename gets its identity from the type, not from the filename

## Summary

A record's identity today is read out of its filename as `<prefix>-<number>-<slug>.md`, so a file named
`0173-slug.md` has no identity, whatever `prefix` its type declares. This RFC proposes the narrowest
config-driven fix: a type may declare that its filenames carry **only a number**, and the type then
supplies the prefix. The identity of `docs/adr/0173-slug.md` becomes `ADR-173` (as a numeric value),
and `ADR-0173` and `ADR-173` resolve to it alike, as `RecordId` (`values.rs`) already makes them equal by numeric value. It is
one declared option on the type, not a second filename policy in Rust, and it is the smallest
interim shape of `MILE-98`'s `identity` declaration rather than a rival to it.

## Motivation

A real adopter corpus of several hundred records names files `NNNN-slug.md` (`docs/adr/0173-slug.md`)
while its prose and code cite `ADR-0173`, always zero-padded, in thousands of places. Reproduced on a
two-record corpus (`0001-a.md`, `0002-b.md`, `prefix: ADR` declared, `Supersedes: ADR-1` /
`Superseded-by: ADR-2`): both `pointer.resolution` targets report "does not resolve to any discovered
record". Declaring a prefix does not help, because `record_id` requires the filename itself to begin
with `<prefix>-`; the prefix is a *filter* on filenames, never a *source* of identity.

This is the same silent shape `BUG-99` found for `audit` on a prefix-less corpus: the first `check`
is wrong, not loud. Every identity-dependent rule (`IDENTITY_DEPENDENT_RULES`: `pointer.resolution`,
`pointer.target-status`, `filename.title-consistency`, supersession reciprocity, `narrative-field.stale`,
`relation.target-status-undeclared`) examines nothing for such a corpus, and `identity.collision`
sees no identifiers at all, so duplicates of one number are never reported either.

`ADR-36`'s amendment (`BUG-9`) removed bare-number acceptance on the argument that no adopter would
independently choose that shape. That premise is falsified by this corpus, and by the `adr-tools`
convention that `MILE-51` and `MILE-98` already use as their example. `BUG-37` is the earlier
instance of the same blind spot: `next_display_number` had to count bare numbers to stop `urzua new`
writing a duplicate. So the numbering path already reads this shape; the identity path does not.

## Proposal

**1. The type declares where the prefix comes from.** An optional per-type setting (working name
`filename_prefix: none`, to be settled by the open questions) states that this type's filenames are
`<number>-<slug>.md` and that the type's `prefix` supplies the identifier prefix. Absent, behavior is
exactly today's: `TYPE-N-slug.md` only. No corpus changes unless it opts in.

**2. Identity is `prefix` (from config) + number (from filename).** For `prefix: ADR` and
`0173-slug.md`, `record_id` yields `ADR-173`. Numeric normalization is the existing `RecordId` comparison (`values.rs`)
(compare by value), so `ADR-0173`, `ADR-173` and `ADR-00173` all resolve to this record in pointer
fields, narrative fields, `graph` and prose-reference extraction. No new matching mechanism; the type
only changes how an id is *produced*, not how it is *compared*.

**3. A declared type only.** The bare form is accepted for a type that declares it, never globally.
`ADR-36` continues to govern every type that does not (the default for a new repo and for this one).
This is therefore a scoped exception to ADR-36's amended rule, not a reversal of it, and would be
recorded as a new ADR deriving from this RFC, not as an edit to ADR-36.

**4. `urzua new` for such a type writes the shape the corpus uses.** Numbering already works
(`next_display_number` counts bare numbers). The emitted filename is `<number>-<slug>.md`, so the
new file does not become the one typed-prefix file in a bare-number directory. Width: see open
questions; the working default is to carry forward the width of the highest existing filename in the
directory, and to use four digits when the directory is empty.

**5. Other rules, by their existing contracts.**

- `identity.collision`: unchanged in logic. It keys on the normalized id, so `0005-a.md` and
  `0005-b.md` collide, and so would `0005-a.md` and `00005-b.md` (same value, different width).
- `filename.title-consistency`: compares the filename number with the H1 number by value (already how
  it works); the H1 for such a type reads `# 173 — Title` or `# ADR-0173 — Title` per the type's
  declared H1 identity (`RFC-44`), not a new rule.
- `migrate ids` backfills `Stable-Id` only and never touches filenames or references; it is
  unaffected, and remains the recommended first step because `Stable-Id` is independent of this
  question.
- `init` (adopt mode, `BUG-61`): detecting a directory of all bare-number filenames is the natural
  point to *propose* the declaration instead of writing an inert prefix. That is a follow-up, not part
  of the minimum.

**6. Relationship to `MILE-98`.** `MILE-98`'s `identity: {from: filename, pattern: ...}` is the
general mechanism and would express this directly (`^(?P<number>\d+)-(?P<slug>.+)$` with the prefix
supplied by the type). This RFC proposes the narrow form first because it is one declared value
on a field that already exists, can be built and verified against a real corpus now, and its
semantics (`prefix` + number, compared by value) are a strict subset of the pattern form. The
declaration should be designed so that, when `MILE-98` lands, it is *expressible as* an identity
pattern and migrates mechanically, never contradicts it.

## Alternatives considered

- **Rename the corpus to `ADR-173-slug.md`** (what `ADR-36` implicitly tells adopters). Rejected: it
  breaks thousands of external links, anchors and git-history paths, and `urzua` should read a corpus
  as it is. The "real bar is replacement": an adopter must be able to delete their linter without
  restructuring their repo first. Remains available through `migrate`, never required.
- **Accept the bare shape globally** (revert `BUG-9`). Rejected: it silently re-opens the cases
  `ADR-36`'s amendment closed, makes identity depend on directory context for every type, and lets a
  stray `1-notes.md` become a record. An opt-in declaration keeps the failure loud elsewhere.
- **Infer it: if every file in a typed directory is bare-number, treat it as bare-number.** Rejected:
  inference is the "decided by whoever happened to be in the directory" shape this project keeps
  removing (`ADR-53`: declared, not voted); a corpus mid-migration would flip meaning with one file.
- **Wait for `MILE-98` and do nothing now.** Not rejected outright: it is the open question below. The
  cost is that an adopter's first `check` is silently wrong until the whole document model ships,
  and `MILE-98` blocks six other milestones.
- **A prefix-alias table (`ADR-0173` -> file) in config.** Rejected: handles the citation form but
  not the missing identity, and does not scale to several hundred records.
- **Resolve `ADR-0173` by scanning for any file whose leading digits match, ignoring the type.**
  Rejected: cross-type collisions (`ADR-5` vs `RFC-5` in one directory) make this ambiguous, and the
  type is exactly the information that disambiguates.

## Open questions

- **Interim option or `MILE-98`'s identity pattern?** Should this ship as its own small declaration
  now, or only as part of `identity.pattern`? If separate, is there a risk of two ways to say the same
  thing that must be reconciled later?
- **What is the declaration's name and shape?** A boolean-like `filename_prefix: none`, an
  `identity: bare-number` enum value, or a minimal pattern string? Which is least likely to need a
  breaking rename when `MILE-98` lands?
- **Is padding width declared or inferred?** For `urzua new`: a declared `padding: 4`, the width of
  the highest existing file, or the most common width? What happens when the directory mixes widths?
- **Does a bare-number type get `urzua new` support at all in the first version?** Reading is the
  blocking defect; writing is convenience. Shipping read-only identity first leaves `new` refusing
  with a clear message instead of guessing a shape.
- **May one repository mix shapes within a type** (some `0173-` and some `ADR-174-`)? The proposal
  says no once declared, but a corpus mid-migration may want both for a while.
- **Which form does a report print?** A finding for `0173-slug.md` says `ADR-173`, `ADR-0173` (the
  form the adopter cites), or the as-written filename number? `ADR-59` (`RecordId` display) bears on
  this and is not yet reconciled.
- **Does `init` propose the declaration automatically**, and on what evidence (every file in a
  directory, or a majority)?
- **Should this be a new ADR that amends `ADR-36`'s scope** (a typed exception) or a section within
  the `MILE-98` ADR?

## Non-goals

- Does not reintroduce the legacy `NNNN-slug.md` shape as a default or for any type that does not
  declare it.
- Does not rename or rewrite an adopter's files or citations.
- Does not design the general identity pattern, field sources or section declarations (`MILE-98`,
  `RFC-33`/`RFC-34`).
- Does not verify that a number in a filename agrees with a record's own header or H1 beyond what
  `filename.title-consistency` already does.

## References

- `MILE-126` — the decision this RFC proposes an answer for.
- `ADR-36` and its amendments — type-prefixed filenames; the bare shape's removal (`BUG-9`).
- `BUG-99` — the same silent first-run wrongness for a prefix-less corpus.
- `BUG-11`, `BUG-2` — id normalization by numeric value, which this design reuses unchanged.
- `BUG-37` — `next_display_number` already counts bare-number filenames.
- `MILE-98`, `RFC-33`, `RFC-34` — the declared document model and identity pattern.
- `RFC-44` — declared record identity (H1 shape) as a field type.
- `ADR-53` — declared, not voted; `ADR-59` — `RecordId` display.

## Decision (2026-11-02)

**Rejected by the maintainer.** A type stays namespaced in its filenames (`TYPE-NNNN-slug.md`, `ADR-36`): a bare number is only meaningful relative to its directory, so it cannot be namespaced across types. A corpus in the bare `NNNN-slug.md` shape is renamed to the typed form instead of the engine gaining a per-type exception. Ids cited as `ADR-NNNN` keep resolving through numeric id comparison, so citations need no change; path references to the renamed files do. A general filename-migration command is future work, not part of this decision.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed as `Draft`. **Why:** a real adopter corpus names files `NNNN-slug.md` and cites `ADR-NNNN`; a declared `prefix` gives such files no identity, so every cross-record pointer fails to resolve (`MILE-126`). | **substantive** |
> | 2026-11-02 | `Draft` -> `Rejected`. **Why:** maintainer decision: keep the type in the filename and rename bare-number corpora rather than add a per-type identity exception. | **substantive** |
