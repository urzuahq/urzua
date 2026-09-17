---
Stable-Id: 01M2P88H1C0KADMN3TRJ0BQSYZ
Status: Fixed
Found-in: 'MILE-51 -- `urzua init` refused this repo''s own corpus, and `urzua new` in an adopted Nygard corpus wrote a record numbered 1 alongside an existing 0001'
Regression-test: 'not yet written -- two planted cases, observed failing: `init` proposing a type for a `TYPE-N-slug.md` corpus, and `next_display_number` returning 10 rather than 1 for a directory of `0001-`..`0009-` records'
---
# 37 — The two halves of adoption recognise mutually exclusive filename shapes

## What was wrong

`init` decides what counts as a record; `new` decides what number comes next. They disagree, and
neither can read what the other accepts.

| | Accepts | Rejects |
|---|---|---|
| `is_record_shaped` (`init.rs:65-74`) | `0001-slug.md` | `ADR-1-slug.md` |
| `next_display_number` (`new_record.rs:21-36`) | `ADR-1-slug.md` | `0001-slug.md` |

`is_record_shaped` requires the stem's first `-`-delimited token to be exactly four ASCII digits.
`ADR-36` removed zero-padding and put the type prefix in the filename, so **every record this project
writes is unreadable to its own adopt mode**:

```
$ urzua init --dry-run          # against a copy of this repo's own docs/
{"status": "not-run", "error": "no record-shaped files found under docs/ -- nothing to adopt"}
```

The mirror image is worse, because it writes. In a corpus adopted from `npryce/adr-tools` — nine
records, `0001-record-architecture-decisions.md` through `0009-help-scripts.md` —
`next_display_number` finds no parseable number, returns `1`, and `urzua new` creates:

```
doc/adr/ADR-1-a-new-decision.md
```

A second record numbered 1, in a filename convention the corpus does not use, sitting beside
`0001-record-architecture-decisions.md`. Verified live.

## Why nothing caught it

`BUG-9` removed legacy bare-number filename support from `next_display_number`, reasoning that *"no
adopter's own corpus would ever independently produce a bare-number filename, and this repo's own
pre-`ADR-36` history no longer exists in the corpus either."* The first half of that is false for the
Nygard convention, which is the most widely used ADR filename scheme there is; the second half made
the case untestable from inside this repo.

`is_record_shaped`'s four-digit requirement predates `ADR-36` and was never revisited when the
filename scheme changed — `init` has been unable to adopt this repo since, and nobody noticed because
`.urzua/config.toml` already exists here, so `init` refuses at `init.rs:135` before reaching the scan.

`MILE-51` exists to exercise a foreign corpus and had never been run.

## Why this is one defect and not two

Both halves encode a filename convention in Rust rather than reading a declared one. `ADR-36`'s scheme
is this repo's; a corpus it adopts has its own. `RFC-29` proposes declaring the numbering convention
per type for `filename.title-consistency`; the same declaration is what both of these need. Fixing
either alone leaves adoption able to read a corpus it cannot then write to, or write to one it cannot
read.

## References

- MILE-51 -- the validation run; the full verdict and its sibling gaps.
- BUG-9, ADR-36 -- the filename scheme and the legacy-support removal whose stated justification this
  contradicts.
- RFC-29 -- declaring the numbering convention per type; likely the shared fix.
- BUG-36 -- adoption's other hardcoded assumption, found in the same run.
- MILE-89 -- `urzua new` colliding with a number claimed elsewhere. A different cause (an unmerged PR)
  with the same symptom; worth deciding together.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Initial record, `Status: Open`. **Why:** found by running adoption end to end against a foreign corpus and then against this repo's own -- both fail, in opposite directions, and the writing half silently produces a duplicate number rather than refusing. `BUG-9` argued the external-adopter case was hypothetical; it is now measured. | **structural** |
> | 2026-09-17 | `Status: Open` → `Fixed`. **Why:** `parse_record_filename` in `urzua-core` is now the single recogniser, and both halves defer to it -- `init`'s adopt scan and `urzua new`'s numbering carried one each, which is what let them accept disjoint sets. It reads `0001-slug.md` and `ADR-1-slug.md`, so adopt mode can finally read the records this project itself writes. `BUG-9`'s exclusion of legacy filenames is reversed: it argued the case 'was never a real external-adopter case', and `MILE-51` is that case -- every filename in an adopted Nygard corpus is `NNNN-`, so the exclusion found nothing to count and `new` returned 1. Verified: in a `0001`-`0003` corpus, `new` now writes number 4.

**Not fixed here, and worth naming:** `new` still writes `ADR-4-slug.md` into a corpus whose own convention is `0004-slug.md`. Recognising both shapes is not the same as *writing* the one a corpus uses, and choosing that per type is `identity.pattern` -- `RFC-33`'s declared document model, not this bug. | **substantive** |
