---
Stable-Id: 01M6MABN5K11J8512YBZH3M031
Status: Accepted
Accepted-on: 2026-11-04
Date: 2026-11-04
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: ADR-64, ADR-11
---
# 75 — A record declares the ids it names without citing as a reference, and citation.resolution does not resolve them

## Context

`BUG-190`. `citation.resolution` has two states for an id written in prose: it resolves to a record, or it
does not. A record that names an id without meaning to cite it (an example, a number that was minted and
retracted, a hypothetical corpus) has no third state, so what its text means depends on which records
happen to exist.

The incident: `ADR-18` and `RFC-5` write "ADR-72/73/74" as a worked example, three records from a different
corpus that independently claimed the same evidence. The engine reads that shorthand as **one** citation,
`ADR-72`: an id needs its prefix, so `73` and `74` are not citations. While this repository had no `ADR-72`,
the finding was covered by standing waivers. Creating `ADR-72` (allowed status transitions) made the
citation resolve to an unrelated real record. The finding vanished, each waiver matched nothing, and
`waiver.matches-nothing` reported it; the waivers were removed in #256. Nothing was wrong with their intent.

The exposure is general, not specific to this pair: any id a record writes in full is captured when a record
with that number is created (a five-digit ADR number in `ADR-36`, a retracted spec number in four records). Nine standing waivers
(`WAIVER-0022`, `0024` to `0031`) already excuse `citation.resolution` findings in nine records, 34 waived
findings in the whole-repository run, and each is exposed the same way. Most are history, not illustration:
a spec number minted and retracted, a bug number renumbered, two ADR numbers of four and five digits (a
width example), and two ADR numbers in padded and unpadded spellings (a hypothetical corpus). Six distinct
ids cover all nine records. A waiver is the wrong tool: it is time-boxed, so it expires into a finding again, and it can
say "excuse this finding" but not "this citation must never be treated as a reference". `external_ids`
(`MILE-156`) exempts an id repository-wide, which is wrong here: `ADR-72` is a real record and must keep
resolving when another record cites it as a reference.

What is missing is a way for one record to say, about its own prose, that these ids are named without being
cited. Two constraints apply: `ADR-64` (structured in, structured out: no prose parsing where a declared
value will do) and `SPEC-1`'s no-silent-no-op rule (a declaration that matches nothing is reported).

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. A per-record header field naming the ids, its name declared in config (chosen) | Explicit; scoped to one record; no prose parsing; a declared id the record never cites is checkable | Granularity is the id, not the occurrence: a record that names `ADR-72` both as an example and as a real reference cannot say which; adds a header field to records, including frozen ones |
| B. An inline marker in the prose (a declared delimiter pair, or a word such as "e.g.") | Per citation; sits next to the text it excuses | Parses prose to find the marker, against `ADR-64`; markers are easy to forget or mistype and a stray one silently exempts a real reference; every example in a frozen record must be edited in the body |
| C. Keep waivers | Nothing to build | Expire into findings; cannot say "never a reference"; the capture is silent (this incident) |
| D. A central map in config (`record -> ids`) | No record edited | An ignore list keyed by record id, which `ADR-11` rejects for waivers; config churns with the corpus |
| E. Reword the records so they write no resolvable-looking id | No engine change | `ADR-18` is Accepted and frozen and `RFC-5` Accepted; their text is the evidence, and a reworded citation hides the example's point |

## Decision

In the context of a prose citation whose meaning changes when an unrelated record is created, facing a rule
with no way to say "this id is named, not cited", we decided that **a record declares, in a header field,
the ids it names without citing them as a reference; `citation.resolution` neither resolves nor reports a
citation of a declared id in that record, and reports a declaration that matches nothing**, to achieve a
citation whose meaning does not depend on the rest of the corpus, accepting that the unit is the id within
one record, not the single occurrence.

1. **Declaration and value grammar.** `citation.resolution` takes an optional `non_reference_field` naming
   a header field. The option is named for what the declaration says (the id is not a reference), because
   most of the evidence is history, not illustration; the field name is config, never hardcoded (`Non-
   reference-ids` is a candidate). The field holds a **comma-separated list of exact ids** (`PREFIX-N`), as
   `Derives-from: RFC-5, ADR-67` does. There are no ranges. Entries are compared by value as record ids
   are, so `ADR-072` and `ADR-72` are one id, and the prefix is case-sensitive. An entry that is not an
   id of a declared prefix (`ADR-72,`, `adr-72`, an undeclared prefix) is a **finding on the record**, never
   dropped. Id parsing is shared with the exact-id path of `external_ids`, so the two options cannot
   disagree on what an id is. At load, the option must name a field some record type lists in `known_fields`
   (`ADR-8`: the header is a closed structure), and must not name a pointer field, a narrative field or a
   relation-role field, since those rules would resolve or stale-check the ids, the opposite of the intent;
   either failure is a config error (`ADR-64` clause 5).
2. **Effect.** In a record that declares an id, a citation of that id in its prose is counted as eligible
   and unexamined, as an `external_ids` match is, so the population still shows it: neither resolved nor
   reported. In any other record the same id resolves as before. Precedence is `external_ids` first, then the
   declaration. "Cited" means the prose the rule already reads: a declared id appearing only in the header,
   in a fenced block, or in a revision-log row while `skip_revision_log` is on counts as uncited.
3. **A declaration that matches nothing is reported.** A declared id the record never cites is a finding on
   the record, naming the field, the id and the line of the field. Because there are no ranges, every
   entry is checked individually. This check has its own population (unit: field entry), so a run over no
   declarations is distinguishable from a clean one.
4. **An id that now resolves is told, not blocked.** When a declared id resolves to a real record, one
   aggregated Info notice per run names the records and ids (as `section_alias_notices` aggregates), produced
   by a sibling pure function called by `check`, not per citation. A notice never changes the exit code
   (`ADR-46`), which is right: the declaration is the author's statement of intent. It is permanent here
   (`ADR-72` exists), like the other Info notices this repository already prints. A repository-set level is
   not added until a repository asks; a notice can become a finding later, which is additive.
5. **Scope.** The field is read only by `citation.resolution`. It is not evidence and not a relationship.
6. **Exercise.** On acceptance the build declares `ADR-72` in `ADR-18` and in `RFC-5`, with no waiver and
   with those records' revision rows. The shorthand "ADR-72/73/74" is read as one citation, so the other two
   numbers are not citations and are not declared. `BUG-190`'s Done means are corrected to match (a revision
   row on the bug). The regression test names the incident (an id later created as a real record) and is
   observed failing first. "Reports nothing" in `BUG-190` is read as no finding: the Info notice remains.
7. **Frozen records.** Adding the field to an Accepted record is a revision-log row of class `structural`
   with a stated reason, not a `## Amendment`: the field carries no decision, as the changes to `ADR-18`'s
   header in #180 and #249 were recorded. An `## Amendment` is for correcting what a decision says. The
   same holds for any header or locator edit on a frozen record.
8. **Waivers.** A waiver for `citation.resolution` on a record that now declares its ids reports as matching
   nothing, which is the signal to retire it; the change that adds a declaration deletes the waiver in the
   same PR. Waivers are broader than a declaration, so retiring them is stricter. The other seven waived
   records are done as a separate change soon after, one PR step per record with its revision row, and are
   not part of this build; the waivers expire 2027-11-04.

## Reversibility

Hard to reverse once records use it: a header field is written into records, including Accepted ones, and
removing the option later would leave a field no rule reads, or require stripping it. The engine side is
small and additive: off unless a repository sets `non_reference_field`, with no effect on any rule when
unset. Choosing B instead would be reversible in the engine but would write markers into prose that must
then be rewritten.

## Consequences

- A named-not-cited id stops depending on the corpus: creating a record with that number changes nothing
  for the declaring record.
- Nine standing waivers can be retired by declaring their six ids in the records they cover.
- A record that uses the same id both ways (an example and a real reference) cannot split them: both are
  exempt. The escape is a header pointer field, which `pointer.resolution` still checks; it does not work for
  a prose reference, so a real prose reference to a declared id is silently exempt too. No record of the nine
  mixes meanings, so this is accepted.
- Declaring an id exempts all of its citations in the record, so a mistaken declaration hides a genuinely
  dangling reference; the uncited-declaration finding and the notice limit the damage, not remove it.
- The guarantee holds while `citation.resolution` is enabled, as for every rule.
- A new header field on each type that carries it: `known_fields` for each, `SPEC-16` and `SPEC-17` (and
  the specs of the milestone and bug types if the other records adopt it) document it, or
  `header.field-set-consistency` flags it.

## References

- `BUG-190`: the incident and the two shapes.
- `ADR-18`, `RFC-5`: the records that write the example.
- `ADR-64`: structured in, structured out.
- `ADR-11`: a waiver is a record, not an ignore list.
- `ADR-8`: the record header is a closed structure.
- `ADR-46`: a notice never changes the exit code.
- `MILE-156`: `external_ids`, the repository-wide exemption this is the per-record counterpart of.
- `BUG-186`: the same class, a declared input set aside on purpose being misreported.
- `SPEC-2`, `SPEC-3`, `SPEC-16`, `SPEC-17`: where the option and field are documented on acceptance.
- #256: removed the waivers.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Drafted as Proposed, then amended after independent review: only the one real citation is declared, the value grammar is exact ids, and the option covers history as well as illustration. | **substantive** |
> | 2026-11-04 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-04, after an independent review and re-review found no remaining blockers. | **substantive** |
