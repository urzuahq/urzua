---
Stable-Id: 01M2QC2094ERQXS4CR9J5SEX7D
Status: Open
Found-in: 'Asking why five specs could be `Draft` while shipped without CI failing -- the rule that should have spoken had silently examined almost none of them'
Regression-test: 'not yet written -- a corpus where one enabled rule has nothing in scope must be distinguishable in the report from one where the rule is broken, and the per-rule count must be comparable across rules'
---
# 40 — `records_examined` cannot distinguish nothing-to-check from checked-nothing, and is not one unit

## What is wrong

Measured on this repository, 2026-09-17, across 242 files:

| rule | `records_examined` | of 242 |
|---|---|---|
| `header.layout-consistency` | **0** | 0.0% |
| `claim.status-agreement` | 4 | 1.7% |
| `type.no-declared-spec` | 6 | 2.5% |
| `embodiment.consistency` | 33 | 13.6% |
| `header.required-fields` | 242 | 100% |
| `pointer.resolution` | 261 | 108% |
| `field.quality` | **817** | **338%** |

### 1. Zero is unreadable

`header.layout-consistency` examined nothing and reports `status: "ran"`. That is *correct* -- it
applies only to blockquote-shaped records and this corpus has none -- but it is indistinguishable
from a rule whose matcher is broken.

`MILE-80` added `not-enabled` so "off" and "ran clean" stay distinguishable. It did not add a signal
for **on, ran, and had nothing in scope**, which is the third state and the one that hides a defect.

`MILE-51` measured exactly this against a foreign corpus -- *"10 of 17 rules examined zero records"* --
and treated it as the headline finding there. The same condition in this corpus reads as success.

### 2. The number is not one unit

- `header.required-fields`: **242** -- one per record
- `pointer.resolution`: **261** -- one per pointer-field *instance*
- `field.quality`: **817** -- one per record x required-field *pair*

Three different denominators under one name. The column cannot be compared between rules, summed, or
read as coverage. 338% is not a typo, and nothing in the report says so.

## Why it matters

This is the failure direction this project keeps finding: not a wrong answer, an answer that never
arrives. A rule silently in scope of nothing is the mechanism behind every instance found today --
`is_terminal_status`'s `_ => &[]`, `extract_references` returning `[]` on prose (`BUG-39`), and
`embodiment.consistency` skipping 18 of 19 specs because `Embodiment` is permitted rather than
required (`BUG-26`).

The engine's own founding claim is that a check which cannot run should say so. It currently says the
opposite.

## Fix

Two parts, and the first is worth more than the second.

1. **Distinguish in-scope-zero from examined-and-clean.** A rule that ran with nothing in scope is a
   third status, not a count of zero. `doctor` already reports one instance of this in prose
   (*"record type 'adr' has no required_fields -- field-quality/header rules will never fire for it"*);
   `check`, the CI gate, says nothing.
2. **Make the count mean one thing,** or name the unit per rule. Comparable numbers, or honest labels;
   not a single column holding three denominators.

## Related

`BUG-26` is one symptom of (1): 18 of 19 specs carry no `Embodiment`, so the rule that would have
caught five shipped `Draft` specs examined 33 records and reported success.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Filed. **Why:** asking why CI never failed on five `Draft`-but-shipped specs. The answer was not a missing rule -- it was that the rule which should have spoken had quietly examined almost none of the records in question, and the report presented that as coverage. Measured across all nineteen rules before filing, which is how the unit problem surfaced. | **substantive** |
