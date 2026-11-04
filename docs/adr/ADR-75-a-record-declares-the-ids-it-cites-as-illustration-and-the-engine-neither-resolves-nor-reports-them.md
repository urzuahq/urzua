---
Stable-Id: 01M6MABN5K11J8512YBZH3M031
Status: Proposed
Date: 2026-11-04
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: ADR-64, ADR-11
---
# 75 — A record declares the ids it cites as illustration, and the engine neither resolves nor reports them

## Context

`BUG-190`. `citation.resolution` has two states for an id cited in prose: it resolves to a record, or it
does not. A citation of an id that is an example, not a pointer, has no third state, so its meaning depends
on which records happen to exist.

The incident: `ADR-18` and `RFC-5` cite "ADR-72/73/74" as a worked example, three records from a different
corpus that independently claimed the same evidence. While this repository had no `ADR-72`, the finding was
covered by standing waivers. Creating `ADR-72` (allowed status transitions) made the citation resolve to an
unrelated real record. The finding vanished, each waiver matched nothing, and `waiver.matches-nothing`
reported it; the waivers were removed in #256. Nothing was wrong with their intent. The same will happen to
the next ids in the sequence (the first is in draft as #260), and the exposure is permanent: every new record
in the namespace can capture an old illustrative citation.

It is not two records. Nine standing waivers (`WAIVER-0022`, `0024` to `0031`) already excuse
`citation.resolution` findings in nine records because they cite ids "as history or illustration", 34
waived findings in the whole-repository run, and each is exposed the same way. A waiver is the wrong tool:
it is time-boxed, so it expires into a finding again, and it can say "excuse this finding" but not "this
citation must never be treated as a reference". `external_ids` (`MILE-156`) exempts an id repository-wide,
which is wrong here: `ADR-72` is a real record and must keep resolving when another record cites it as a
reference.

What is missing is a way for one record to say, about its own prose, that these ids are examples. Two
constraints apply: `ADR-64` (structured in, structured out: no prose parsing where a declared value will
do) and `SPEC-1`'s no-silent-no-op rule (a declaration that matches nothing is reported).

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. A per-record header field naming the illustrative ids, its name declared in config (proposed) | Explicit; scoped to one record; no prose parsing; a declared id the record never cites is checkable | Granularity is the id, not the occurrence: a record that cites `ADR-72` both as an example and as a real reference cannot say which; adds a header field to records, including frozen ones |
| B. An inline marker in the prose (a declared delimiter pair, or a word such as "e.g.") | Per citation; sits next to the text it excuses | Parses prose to find the marker, against `ADR-64`; markers are easy to forget or mistype and a stray one silently exempts a real reference; every example in a frozen record must be edited in the body |
| C. Keep waivers | Nothing to build | Expire into findings; cannot say "never a reference"; the capture is silent (this incident) |
| D. A central map in config (`record -> illustrative ids`) | No record edited | An ignore list keyed by record id, which `ADR-11` rejects for waivers; config churns with the corpus |
| E. Reword the records so they cite no resolvable-looking id | No engine change | `ADR-18` is Accepted and frozen and `RFC-5` Accepted; their text is the evidence, and a reworded citation hides the example's point |

## Decision

In the context of a prose citation whose meaning changes when an unrelated record is created, facing a
rule with no way to say "this is an example", we propose that **a record declares, in a header field, the
ids it cites as illustration; `citation.resolution` neither resolves nor reports a citation of a declared id
in that record, and reports a declaration that matches nothing**, to achieve a citation whose meaning does
not depend on the rest of the corpus, accepting that the unit is the id within one record, not the single
occurrence.

1. **Declaration.** `citation.resolution` takes an optional `illustrative_field` naming a header field. The
   field holds the ids, written as `external_ids` entries are written (an exact id, or an inclusive numeric
   range `PREFIX-LOW..HIGH`), so the repository learns one syntax; ids are compared numerically, so padding
   does not matter. The name is config, never hardcoded. At load it must be a field some record type lists in
   `known_fields` (`ADR-8`: the header is a closed structure), and must not be a declared pointer field:
   `pointer.resolution` would otherwise resolve it, which is the opposite of the intent. Either failure is
   a config error (`ADR-64` clause 5).
2. **Effect.** In a record that declares an id, a citation of that id in its prose is counted as eligible
   and unexamined, as an `external_ids` match is, so the population still shows it: neither resolved nor
   reported. In any other record the same id resolves as before.
3. **A declaration that matches nothing is reported.** A declared id the record never cites is a finding on
   the record, naming the field, the id and the line of the field. This holds also for a range whose every
   member is uncited, so a stale declaration is never a silent no-op.
4. **An illustration that has become ambiguous is told, not blocked.** When a declared id now resolves to a
   real record, the rule emits a notice (not a finding) naming the record and the id: the author learns that
   the example now shares a name with a real record, and the record still reports nothing, because the
   declaration is the author's statement of intent. See open question 1.
5. **Scope.** The field is read only by `citation.resolution`. It is not evidence and not a relationship,
   and no other rule treats it as a pointer.
6. **Exercise.** On acceptance the build declares the three example ids in `ADR-18` and `RFC-5`,
   with no waiver, so that `urzua check` reports nothing for them before and after those records exist. The
   regression test names the incident (an illustrative id later created as a real record) and is observed
   failing first. The other seven records under standing waivers are candidates for the same treatment
   and are not part of this decision.

## Reversibility

Hard to reverse once records use it: a header field is written into records, and `ADR-18` and `RFC-5` are
Accepted, so adding it to them is an amendment of a frozen record (open question 2). Removing the option
later would leave a field no rule reads, or require stripping it. The engine side is small and additive:
off unless a repository sets `illustrative_field`, with no effect on any rule when unset. Choosing B instead
would be reversible in the engine but would write markers into prose that must then be rewritten.

## Consequences

- An example stops depending on the corpus: creating the second and third example ids as real records changes nothing for `ADR-18`
  and `RFC-5`.
- Nine standing waivers can eventually be retired by declaring their ids in the records they cover. Doing so
  is a separate change per record, since most of those records are frozen.
- A record that cites the same id both as an example and as a real reference cannot split them; it must
  cite the real one in a way the rule reads as a reference, for example in its header pointer field, or
  not use that id as an example. This is the cost of option A against B.
- Declaring an id exempts all of its citations in the record, so a mistaken declaration hides a genuinely
  dangling reference; the check on an uncited declaration and the notice in decision point 4 limit the
  damage, not remove it.
- A new header field on ADRs and RFCs, one more thing authors may forget; absence just leaves today's
  behaviour.

### Open questions for the maintainer

1. `BUG-190` asks both that an illustration that would match a real record be reported and that `ADR-18`
   and `RFC-5` report nothing once the ids exist. Point 4 reconciles them as a notice. Is a notice enough,
   or should it be a finding at a level the repository sets?
2. `ADR-18` is an Accepted, frozen ADR. Does adding a declared header field need a dated `## Amendment`
   section (as `ADR-67` did for its changes), or is a header field that carries no decision exempt?
3. Should the declaration be per occurrence after all (option B), accepting prose parsing, because the
   same-id-two-meanings case matters more than the header field?
4. Should the field also be accepted on the other seven waived records now, or only on `ADR-18` and `RFC-5`
   as `BUG-190` says?
5. The field name (`Illustrative-ids` is the BUG's example) and whether the range syntax is worth sharing
   with `external_ids`, or one exact id per entry is enough.

## References

- `BUG-190`: the incident and the two shapes.
- `ADR-18`, `RFC-5`: the records that cite the example.
- `ADR-64`: structured in, structured out.
- `ADR-11`: a waiver is a record, not an ignore list.
- `ADR-8`: the record header is a closed structure.
- `MILE-156`: `external_ids`, the repository-wide exemption this is the per-record counterpart of.
- `BUG-186`: the same class, a declared input set aside on purpose being misreported.
- `SPEC-2`, `SPEC-3`: where the option is documented on acceptance.
- #256: removed the waivers; #260: the next id in draft.
