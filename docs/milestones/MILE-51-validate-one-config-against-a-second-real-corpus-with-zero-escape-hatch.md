---
Status: Done
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

**The five gaps share one cause, named after the fact in `RFC-33`.** Each was filed here as a missing
config *key*; that framing was too small. The engine ships governance opinions an adopter cannot
decline — `type.no-declared-spec` fires on the existence of a record type, reading zero records — and
the gaps are symptoms. `RFC-33` proposes the general form and is a Phase 0 requirement.

**The useful output is the five gaps, each now measured rather than predicted.** Closing them is
downstream work this milestone informs; re-running this validation is how it eventually closes.

## Re-run, 2026-09-17, against `npryce/adr-tools` at HEAD

`BUG-36` and `BUG-37` -- the two defects that made the original run fail before it reached a single
record -- were fixed the same day. Re-run to find out what the verdict is once adoption mechanically
works.

| | original run | re-run |
|---|---|---|
| `urzua init` | refused: `docs/` hardcoded, corpus is at `doc/adr/` | **ok** -- adopts `doc/adr`, 9 records |
| `check` files examined | **0** | **9** |
| findings | **9 blocking errors** | 10 warnings, `blocking: false` |
| `urzua new` | returned 1, beside an existing `0001-` | correct next number |
| rules examining zero records | 10 of 17 | **14 of 20** |

**The blockers are gone. The verdict is unchanged.** Adoption now runs end to end and does not hand an
adopter errors they never asked for, which is this milestone's own standard. But the two findings they
*do* receive are the original verdict's gaps 1 and 2, word for word:

```
[warning] header.required-fields: no header-shaped region found -- required fields [] cannot be checked
[warning] type.no-declared-spec:  record type 'adr' has no declared spec
```

Nothing was required, and it still fails anyway -- because the engine cannot be told that this type's
metadata is not in a header at all. A Nygard record carries a bare `Date:` line between the H1 and the
first `##`. `init` proposes `header_shape: yaml-frontmatter` regardless (`ADR-33`, deliberate), so the
engine asks nine records for a shape their convention does not have.

That is the declared document model, `RFC-33`'s layer 1, and it is the remaining blocker for this
milestone's founding claim. Gap 2 is `ADR-51`, which rejected `spec = "none"` the day it was proposed
and left the rule with no way to be silenced by configuration.

**14 of 20 rules examined zero records**, a worse proportion than the original 10 of 17 -- partly
because three rules shipped today (`pointer.target-status`, `field.pending`,
`claim.status-agreement`) legitimately have nothing to check here. `BUG-40` is what makes that number
unreadable: every one of the fourteen reports `status: "ran"`, indistinguishable from a rule that
examined records and found them clean.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Recorded the first run's result. `Status` stays `Planned` -- the claim fails and five schema gaps are now named with measurements behind each. **Why:** this is a validation milestone whose own Why calls it *"the cheapest, most informative validation available before building anything else on top of the current schema"* -- so the deliverable is the verdict, not a patch. Two drafted fixes were rejected under adversarial review for making the run quiet rather than the schema expressive; that reasoning is recorded above because it is the more durable half. | **substantive** |
> | 2026-09-17 | `Status: Planned` → `Done`. **Why:** this milestone's deliverable is a verdict, not a fix, and the verdict exists: run against `npryce/adr-tools`, the founding claim **failed** -- `init` could not run, a hand-written config produced 9 blocking errors, and 10 of 17 rules examined zero records. That result produced `RFC-33` and `ADR-53`. Leaving it `Planned` claimed the validation had not happened when it is the single most consequential thing this project has run. | **substantive** |
> | 2026-09-17 | Re-run against the real corpus after `BUG-36`/`BUG-37` landed, result recorded above. **Why:** the original verdict failed at step one -- `init` could not run -- so it could not say whether anything *past* step one worked. It now does: adoption is end to end and non-blocking, and the founding claim still fails, on the same two gaps, for the same reason. Status stays `Done`: this milestone's deliverable is a verdict, and a second verdict does not reopen it. | **substantive** |
