---
Status: Planned
Stable-Id: 01M2Q8V7B6SHPXH6X92KKRQG95
Phase: '2'
Track: schema-governance
Implements: —
Blocked-on: —
---
# 96 — Support the Python PEP corpus end to end, as the exercise that proves a foreign flow

## What

Take a corpus this project does not use, and make `urzua` read and check it without a code change
that is specific to it. Python PEPs are the candidate: ~600 records, actively maintained, and shaped
unlike anything here.

Deliverable is a working configuration plus whatever primitive it turns out to need -- and a written
account of which parts fought back.

## Why this corpus

It differs on the axes the two corpora already examined do not.

| | `adr-tools` | MADR | this repo | PEPs |
|---|---|---|---|---|
| where fields live | bare `Date:` line | YAML frontmatter | YAML frontmatter | **RFC-822 header block** |
| status vocabulary | 4 values | 5 | per type | **7**: Draft, Accepted, Final, Rejected, Withdrawn, Deferred, Superseded |
| relations | — | — | `Derives-from`, `Supersedes` | **`Replaces` / `Superseded-By`, a real reciprocal pair in the wild** |

The status vocabulary and the reciprocal pair are the valuable parts: `enumeration`, `target-status`
and `reciprocal` have only ever been exercised against corpora whose conventions this project wrote.

## Why it is a milestone and not a gate

An earlier framing made a third corpus a prerequisite before `RFC-33`'s document model could be built.
It is not. `ADR-53` decided the principle and the declared model without it, and deliberately left
only the **function vocabulary** open. Running this corpus tests that vocabulary; it does not block the
layer beneath it.

Nor does it justify building a parser in advance. `RFC-34`'s admission rule is that a primitive
arrives when this project needs one, or when a corpus demonstrates the gap -- and **this milestone is
how that gap gets demonstrated**, rather than a reason to assume it now. `rfc-822` was withdrawn from
`RFC-34` on exactly that basis and should be reached here or not at all.

## What would count as done

1. A `.urzua/config.yaml` that reads the PEP corpus, written by hand, with no PEP-specific code.
2. Every rule that fires is one a PEP maintainer would recognise as a real problem -- the `MILE-51`
   standard: an adopter must not receive findings they never asked for.
3. Whatever the config could **not** express is written down. That list is the actual output; a clean
   pass proves less than a specific failure does.

## Related

`MILE-51` ran this exercise against `adr-tools` and the founding claim failed, which is what produced
`RFC-33` and `ADR-53`. This is the same exercise against a corpus that is not an ADR corpus at all.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Filed. **Why:** a third corpus had been framed as a gate before the document model could be built. It is not -- `ADR-53` decided the principle without it and left only the function vocabulary open. Refiled as the exercise it actually is: read a foreign corpus end to end, and let that demonstrate which primitives are needed rather than assuming them, per `RFC-34`'s admission rule. | **substantive** |
