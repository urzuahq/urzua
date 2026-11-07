---
Stable-Id: 01M6N33VQH2FQYGG0GDG80FV6X
Status: Accepted
Accepted-on: 2026-11-05
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: RFC-11
---
# 80 — Boundaries are declared paths with a kind, a reason and a severity

## Context

`RFC-11` (Accepted 2026-09-05) proposes that a boundary, a set of paths that publish beyond the corpus,
is a declared fact that every tool reads, and that prohibitions follow from it rather than being
configured one by one. `ADR-9` decided the general rule (declared width compared to matched width, a
prohibition fails closed, an absence check ships with a planted violation) and left the boundary
mechanism itself to be built. `MILE-125` is that build, and the maintainer decided on 2026-11-05 to build
`RFC-11` sections 1 to 3.

What exists today is the first slice: `boundary.published-path-reference` reads a `published_paths`
list, reports a record identifier cited inside one, and takes its level as the boundary's severity. What
is missing is the rest of section 1 and the questions the RFC left open for sections 1 to 3:

- A boundary has no **kind** and no **reason**, so `published` is the only thing it can mean and nothing
  records why a path is declared. `RFC-11` section 4 makes the reason part of the contract under review.
- **Severity is the rule's level**, one value for every path. Two boundaries that differ (a public mirror
  that must block, a vendored directory that need only warn) cannot be told apart.
- **Prohibitions are not derived from the kind.** A `generated` path, which is overwritten, has no rule
  saying a human-authored back-pointer may not live there.
- **Violation and evidence are not kept apart by construction.** `RFC-11` section 3 warns that widening
  one shared detector to catch prose citations would start counting prose as realization evidence, so a
  decision would read as built because a comment said "see ADR-42".
- **There is no way to except a path or a line.** `RFC-11`'s last open question names the trap: a
  linter's own test fixtures contain record-shaped strings on purpose, and a boundary that includes them
  either reports them or is quietly narrowed by an ad-hoc ignore list, which `RFC-11` and `RFC-13` both
  reject.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Keep `published_paths` and add a sibling list per new prohibition (`generated_paths`, `vendored_paths`, ...) | No config change | Each prohibition needs its own key and the set drifts; carries no reason or per-path severity, the opposite of `RFC-11` section 1 |
| B. Declare each boundary as a structured entry: path globs, a kind, a required reason, a severity (chosen) | One declaration, several consequences; reviewable reasons; severity per boundary | A new config key and a compatibility path for `published_paths` |
| C. Exceptions by a repository-wide ignore list | Smallest to build | An ad-hoc exclusion detached from the boundary it excuses, which `RFC-11` and `RFC-13` reject |
| D. Exceptions only per boundary (declared `exclude` globs) | Reviewed in the config, no source edits | A single line in an otherwise covered file cannot be excused without excluding the whole file |
| E. Exceptions only by an inline marker | Local to the line | A fixture directory needs a marker on every line, and nothing shows the exception in the config |
| F. Both: declared `exclude` globs per boundary, and a declared inline marker for one line, with an unused marker reported (chosen) | Each exception is at the width of the thing excused; a stale exception is visible | Two mechanisms to learn and test |

## Decision

In the context of published, vendored and generated paths whose rules are today encoded as one list and
one rule level, facing the three gaps `RFC-11` names (width, severity, duplication) and the question of
how a boundary excuses its own fixtures, we decided that **config declares boundaries as structured
entries, each a set of path globs with a kind, a required reason and a severity; prohibitions follow from
the kind; exceptions are declared both per boundary and per line; and the whole mechanism is opt-in**, to
achieve one declaration that every tool reads, accepting a second config shape beside `published_paths`
for as long as that key is read.

- **A boundary is `paths` (globs), `kind`, `reason`, `severity`, and an optional `exclude` (globs).** The
  kind is one of `published`, `vendored` or `generated`; any other value is a config error. `reason` is
  required and must be non-empty: it is specification, not decoration (`RFC-11` section 4). `severity`
  is `error` or `warn` and is the level of every finding the boundary produces. The shape is structured
  config, never a string mini-language (`ADR-64`); glob matching is the one `MILE-158` already defined.
- **Prohibitions follow from the kind, and the kind is the only place they come from.**
  - A record identifier inside a `published` boundary is a violation. Any identifier carrying a declared
    type prefix counts, resolvable or not, as `boundary.published-path-reference` already decides.
  - A back-pointer inside a `generated` boundary is reported, because a human-authored marker there is
    overwritten.
  - A `vendored` boundary is declared and carries no prohibition in this decision. Code the repository
    does not author can cite whatever it likes, and `RFC-11` section 1 names no consequence for it. The
    kind exists so a later prohibition has somewhere to attach, and so an evidence or scope question can
    ask whether a path is vendored.
- **Violation and evidence use separate matchers.** The boundary check uses the wide matcher (any record
  identifier). The evidence collector keeps its narrow, structured matcher (`Implements:` / `Verifies:`)
  and never widens to the boundary's. A realization marker inside a `published` boundary is not evidence,
  whether or not it is also a violation (`RFC-11` section 3). Each matcher has a test that the other's
  text does not satisfy it: prose citing a record is not evidence, and an evidence marker is not
  mistaken for the only violation form.
- **`published_paths` is read as a published boundary.** An entry in the existing key is a `published`
  boundary whose severity is the rule's level and which is exempt from the required `reason`, so a
  config that works today keeps working with the same findings. When both forms are declared, both apply.
  The key stays supported; this decision does not deprecate it.
- **Exceptions work both ways.**
  - A boundary's declared `exclude` globs remove paths from that boundary for the violation check, the
    way a fixture directory is excused. An `exclude` entry that matches no tracked file is reported, as a
    declared path that covers nothing already is: nothing declared may match nothing silently.
  - A declared inline ignore marker excuses the single line it appears on. The marker text is declared
    in config and the engine compiles in none. A marker that no longer suppresses any finding is
    reported (a declared exception needs a check that it still applies, `RFC-11`), at the boundary's
    severity.
- **Opt-in.** Nothing runs unless the repository declares a boundary or `published_paths`. Declaring
  neither is not a finding (`ADR-53`).
- **A boundary that covers no tracked file is itself a finding.** This is the prohibition-fails-closed
  rule of `ADR-9`, kept as it is today and extended to every kind.

**Not decided here, for the maintainer before this is Accepted:**

1. **How a rule's `level` and a boundary's `severity` relate.** Today the rule's level is the boundary's
   severity. With a severity per boundary, the proposal is that the boundary's severity sets the level of
   its findings and the rule's `level` only enables the rule. If the rule's level is meant to cap a
   boundary's severity instead, say so.
2. **The marker's exact placement.** This ADR decides a declared marker that excuses one line. Whether
   the marker also excuses the line below it (a marker on its own line) is a build detail the maintainer
   may want to settle, since it changes what an unused marker is.
3. **Whether a path in a boundary's `exclude` can still supply evidence.** The proposal is that a
   `published` boundary's exclusion is for the violation check only, and the evidence rule keys on the
   boundary's `paths` alone, so an excluded fixture directory does not become evidence by being excused.
4. **Whether `generated` also reports a record identifier, not only a back-pointer.** `RFC-11`
   names only the back-pointer; generated text can quote an identifier legitimately.
5. **What a legitimate reference becomes inside a published boundary** (stripped, inlined, or a public
   URL), and whether boundaries can be derived rather than declared. Both stay open `RFC-11` questions.
   This decision reports the violation and rewrites nothing.

## Amendment (2026-11-07): the five open questions were resolved as drafted

The list above, headed "Not decided here, for the maintainer before this is Accepted", was written while the
ADR was Proposed. The maintainer accepted the ADR on 2026-11-05 with the drafted choices, so each of the five
reads as the Decision and the proposals in that list state them. `Status` stays `Accepted` and the Decision
text above is unchanged.

## Reversibility

Moderate. The boundary entry is additive config and `published_paths` keeps working, so adopters who
declare nothing new see no change, and removing the new key later costs a config migration, not a data
migration. The commitments that are hard to take back are the closed `kind` vocabulary and the meaning of
`severity`: a config written against them breaks if either is reinterpreted. The separation of the two
matchers is cheap to keep and costly to lose, because merging them silently inflates Embodiment.

## Consequences

- `SPEC-3` gains the `boundaries` option shape and `SPEC-2` the new findings, each with a revision row;
  `published_paths` stays documented as the compatible form.
- `urzua`'s own `.urzua/config.yaml` declares its published paths as boundaries with reasons, and `check`
  reports none unwaived (`MILE-125`).
- The evidence collector gains a skip for paths under a `published` boundary, so an existing `Implements:`
  comment inside one stops counting. A repository that relied on that sees Embodiment drop and a boundary
  finding appear, which is the intent.
- Three new reportable states: an `exclude` entry matching nothing, an inline marker suppressing
  nothing, and a boundary covering nothing. Each is a deliberate loud failure, so a config edit that
  renames a directory is reported rather than silently disabling the boundary.
- Fixtures that deliberately contain record-shaped strings must sit under an `exclude` or carry the
  marker, and a test asserts the fixtures are handled by declaration and not by luck.
- Two exception mechanisms to document and test.

## References

- `RFC-11` -- sections 1 to 3 are built here; section 4 is partly built by the required reason and the
  stale-exception reports; section 6 (scope) is not touched.
- `ADR-9` -- the general width rule this applies; fail-closed prohibitions.
- `ADR-53` -- every rule is a declared, opt-in policy.
- `ADR-64` -- structured config in, structured output out; no string mini-languages.
- `MILE-125` -- the milestone this unblocks; `MILE-158` -- the glob matching reused.
- `RFC-13` -- rejects ad-hoc exclusion lists.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** maintainer decision of 2026-11-05 to build `RFC-11` sections 1 to 3 for `MILE-125`; building part of an RFC needs its own decision record, and the maintainer approves before it is Accepted. | **substantive** |
> | 2026-11-05 | Status changed from Proposed to Accepted, with the drafted choices on RFC-11's gaps. **Why:** Maintainer decision, 2026-11-05. | **substantive** |
> | 2026-11-07 | Amendment added: the five questions listed as open in the Decision were resolved as drafted when the ADR was accepted. **Why:** the Decision still carried "for the maintainer before this is Accepted" after acceptance; `Status` stays `Accepted` and the Decision text is unchanged. | **structural** |
