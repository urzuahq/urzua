---
Status: Planned
Stable-Id: 01M1YN87AP04NCFMXY4WBJK4EK
Phase: '0'
Track: corpus-corrections
Blocked-on: —
---
# 51 — Validate one config against a second real corpus with zero escape hatch

## What

Run `urzua init` + `check` against a second real, independently-authored corpus, with the explicit success criterion that the same config mechanism (record types, required fields, header shapes) expresses both corpora's real rules with no escape hatch or code change.

## Why

This repo's own corpus is the only one `check` has ever run against. "One config, not a fork per repo" is a real, falsifiable claim about the schema, and it's never actually been tested against a corpus this project didn't author -- the cheapest, most informative validation available before building anything else on top of the current schema.

## Result (2026-09-16): the claim fails, with five schema gaps

Run against `npryce/adr-tools` — 9 real Nygard-style records under `doc/adr/`, a corpus this project
did not author and whose convention predates it.

**The criterion is not met, and not nearly.** Its two conjuncts are *"expresses both corpora's real
rules"* and *"no escape hatch or code change."* A Nygard record's real rules are a `# N. Title` H1, a
bare `Date:` line, and `## Status`/`## Context`/`## Decision`/`## Consequences`. The config schema
expresses none of them.

### What was measured

| | Result |
|---|---|
| `urzua init` on the corpus | `not-run` — *"no record-shaped files found under docs/"*; it lives in `doc/adr/` |
| `urzua init` on **this repo's own** `docs/` | `not-run` — the same message; `is_record_shaped` wants four leading digits and `ADR-36` filenames are `ADR-1-…` |
| `check` under a hand-written config | 9 × `header.required-fields` (error) + 1 × `type.no-declared-spec` (warning); `blocking: true` |
| `check` under an `init`-shaped config | identical findings, and **11 of 17 rules examine zero records** rather than 10 — `init` writes no `known_fields`, so `header.field-set-consistency` drops out too |
| `urzua new adr` in the adopted corpus | wrote `doc/adr/ADR-1-a-new-decision.md`, `display_number: 1`, beside an existing `0001-record-architecture-decisions.md` |
| `doctor` | already reports *"record type 'adr' has no required_fields -- field-quality/header rules will never fire for it"* |

Adopting through `init` is **strictly less checked** than the hand-written escape hatch this milestone
forbids. And `doctor` already names the emptiness, while `check` — the CI gate — says nothing.

### The five gaps

| # | Gap | Record |
|---|---|---|
| 1 | No way to declare "this type's metadata is not in a header" | `RFC-31` |
| 2 | No way to declare "this type deliberately has no spec" — no config value silences `type.no-declared-spec` | `RFC-32` |
| 3 | No way to declare a filename/numbering convention — `filename.title-consistency` examines zero here | `RFC-29` |
| 4 | No way to declare section structure — where a Nygard record's metadata actually lives | `MILE-4` |
| 5 | `init` cannot adopt outside `docs/`; adoption's two halves read mutually exclusive filename shapes | `BUG-36`, `BUG-37` |

### Why this milestone stays open, and no fix shipped with this result

Two fixes were drafted and rejected under review, and the rejections are part of the finding:

- **Guarding `header.required-fields` when nothing is required** inverts
  `check_integration.rs:380-416`, whose comment states the introduced behaviour *is* the bug. Since
  `init` always writes `required_fields = []` and `header_shape = "yaml-frontmatter"` (`ADR-33`), the
  guard would delete that diagnostic for every adopted corpus — its only audience.
- **Pointing `init`'s scan at the tracked set** addresses one of four hardcodes; without `init.rs:56`
  it proposes `dir = "docs/adr"` for a corpus at `doc/adr` and `check` examines nothing.

Both would have made the run quiet without making the schema more expressive — which is the failure
mode `AGENTS.md` names directly: *"don't widen a config list, an enum, or an ignore-list just to make a
diff come up clean."* A green run over a corpus the tool understands nothing about is not the claim
this milestone tests.

**The useful output is the five gaps, each now measured rather than predicted.** Closing them is
downstream work this milestone informs; re-running this validation is how it eventually closes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Recorded the first run's result. `Status` stays `Planned` -- the claim fails and five schema gaps are now named with measurements behind each. **Why:** this is a validation milestone whose own Why calls it *"the cheapest, most informative validation available before building anything else on top of the current schema"* -- so the deliverable is the verdict, not a patch. Two drafted fixes were rejected under adversarial review for making the run quiet rather than the schema expressive; that reasoning is recorded above because it is the more durable half. | **substantive** |
