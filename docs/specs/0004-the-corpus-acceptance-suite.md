# SPEC-0004 — The corpus acceptance suite

> Version: 0.1 | Date: 2026-08-20 | Status: Draft
> **Parent:** SPEC-0001 (v0 CLI), which lists the bug classes this suite must reproduce.

## Purpose

A validator is only as trustworthy as the defects it has been shown to catch. Propose an executable
acceptance suite built from documented, anticipated bug classes — each with a known-correct
outcome — run against synthetic corpora designed to exercise real corpus variance (irregular
formatting, wrapped fields, mixed header shapes) rather than only clean, hand-crafted fixtures.

This spec turns that bug-class catalog into an executable suite, and is a prerequisite for
SPEC-0002's criterion 3.

**It is deliberately not on the bootstrap path.** SPEC-0002 Phase A closes the self-hosting loop
against this repository's own `docs/` — corpus zero — which needs nothing here. The synthetic
corpora take real engineering effort to build well; the fastest deliverable must not queue behind
the slowest one. This is Phase D.

## Structure

```
corpora/
  <corpus-id>/
    records/          synthetic record files, engineered to exhibit specific variance
    manifest.toml     what irregularity each record was built to exhibit
    .urzua/           the config this corpus is validated under
    expectations/
      <case-id>.toml  one per acceptance case
```

Corpus ids are opaque (`corpus-a`, `corpus-b`, `corpus-c`). Nothing in `corpora/` may name an
organization, repository, product, or person — a corpus is a synthetic fixture engineered to exhibit
specific variance, not a record of anything real.

## Realistic variance, engineered deliberately

The hard requirement: **build fixtures with the same irregularity real corpora accumulate, without
claiming to represent any real one.**

The variance *is* the test. A regex invalidated by irregular formatting, a status value carrying
trailing free-text, a header interrupted by a blockquote, a multi-line wrapped field — a suite of
clean, hand-crafted fixtures will never exercise these, because a fixture built to be readable is
built to avoid exactly the mess a real corpus produces by accident.

Rules:

- Substitute identifiers **consistently within a corpus**. A name becomes a stable pseudonym, so
  reciprocity and back-pointer relationships still resolve.
- **Never reflow, reformat, or normalize a fixture once it's built to exhibit a specific defect.**
  Whitespace, wrapping, and punctuation are the material under test.
- Prefer relative date intervals over absolute ones where a case tests a grace window or claim age —
  relative intervals are load-bearing for that class of test; absolute dates rarely are.
- `manifest.toml` records what irregularity each fixture was built to exhibit, so a failing case can
  be reasoned about without guessing at intent.

## Acceptance cases

Each case is a corpus, a config, an invocation, and an expected result:

```toml
id = "blank-field-misdetection"
corpus = "corpus-a"
invocation = ["check", "records/"]
expect_exit = 1
expect_findings = [{ rule = "field.blank-vs-placeholder", file = "...", line = 12 }]
```

The suite is red before the rule exists and green after. A case that passes against a build with the
rule removed is not a case.

### Field-quality and structural cases

| Case | Corpus |
|---|---|
| Blank-field misdetection | corpus-a |
| Provisional-status misclassification | corpus-b |
| Reciprocity-checker data loss | corpus-c |
| Decoy-corpus silent no-op | corpus-a |
| Regex invalidated by realistic corpus variance | corpus-a |
| Filename/title mismatch | corpus-a |
| Enum completeness against corpus prose | corpus-a |
| Check exists vs check wired | corpus-a |
| Discovery scoped to VCS, not filesystem | corpus-a |
| Pre-retrofit reverse-reference scan | corpus-a |
| Structural presence vs content scope (permanent ceiling — asserted as *not* mechanically closable) | corpus-a |
| Header field satisfied by a body bullet | corpus-c |
| Multi-line continuation silently truncated | corpus-c |
| Header slice truncated at the first heading | corpus-c |
| Machine channel reports `ok` while prose says "nothing to compare" | corpus-c |
| Agent invents a schema field mid-edit | corpus-b |
| Gate reports drift while exiting 0 | corpus-b |
| A derived value computed by a near-neighbour query | corpus-b |
| A number holding no decision fails checks presupposing one | corpus-b |

The last two are the strongest cases in the suite, because both produce **well-formed, plausible,
confidently wrong output** rather than an error — the class no smoke test catches.

### Silent zeros from a derived rule

Four cases, one shape: the rule runs, examines every record, and returns a clean zero because the
**evidence it reasons over** was empty. None raises an error, and three of the four pass a
files-examined / rules-executed report — which is why SPEC-0002's no-silent-no-op criterion was
tightened to report each rule's input population.

| Case | What the rule reports |
|---|---|
| History query stops at a rename (`git log` without `--follow`) | 0 of 91 records had readable prior state; corpus reported fully clean |
| `-S` (occurrence count) used where `-G` (line content) was needed | Every value-only edit invisible, so no field change was ever detected |
| A record's *filename* pattern reused to resolve *references* | A reference written in prose matched nothing; the rule could not fire |
| Defined identifiers collected by mention rather than by definition | A typo'd back-pointer **defined its own target** and validated against itself |

The last is the most instructive: gathering "which identifiers exist" by scanning for the identifier
pattern makes the validation circular, so the rule can never fire for anyone. Definitions must be
recognised by syntactic role — a heading, a table's key column — not by the identifier appearing.

### Architecture claims, not behaviours

One case tests whether a *design* holds rather than whether an input produces an output.

| Case | What it asserts |
|---|---|
| The Nth record type costs a declaration, not a checker | Declare a throwaway record type as data and nothing else; it must receive the full rule set, with planted violations firing, and **no code change** |

This is the only executable test of RFC-0001 §1a's "profiles are declared, not enumerated by the
tool". The claim is otherwise asserted and unfalsifiable; running it converts the claim into an
observation. If adding the next type is not cheap, the profile mechanism is not finished.

### Corpus zero — this repository

`docs/` is a corpus with the property none of the synthetic ones have: it is live, and its defects
are found by the tool as they are introduced rather than reconstructed after the fact.

| Case | What |
|---|---|
| Three header formats in one corpus | RFC and ADR share a shape; SPEC-0001 uses it while SPEC-0002 to SPEC-0004 use a pipe-delimited line with bold keys. |
| Six `Implements:` pointers at `Draft` records | Every RFC that `docs/specs/` declares it implements is undecided. |

Both are the seed rules for SPEC-0002 Phase A. Neither is fixed ahead of the checker on purpose: a
corpus with no defects cannot demonstrate that a checker works.

### Property: no input is ever silently ignored

Most defects found in a mature governance linter are not wrong answers. They are
*absences* — a field the matcher could not see, a reference read as prose, a rule
that returned empty because its input moved. A suite of example cases cannot find
these, because the missing report looks identical to a clean corpus.

One generative property covers the class:

> For any generated field name, header shape, or relation value, the checker's
> output is either a recognised-and-accepted result or a report. Never neither.

Concretely, for each record profile:

- **Field names.** Generate names from a wide alphabet — including underscores,
  punctuation, and non-ASCII. Each must be accepted as declared or reported as
  undeclared. A name that produces no output at all is a failure, and it is the
  failure that a narrow matcher produces.
- **Header location.** Wrap arbitrary content in every fence variant (backtick
  and tilde, three or more, indented, nested) and in comments, before and after
  the header. The verdict must not change. This is the inverse direction and it
  fails on *correct* input, so both are needed.
- **Relation values.** Generate lists of references with annotations that
  themselves contain references and delimiters. The claims must round-trip to the
  references that begin each entry, and no annotation reference may ever appear.

**Mutation is what makes these trustworthy.** Reintroduce each defect the suite documents and
confirm the property fails. A property test that has never been observed failing is
indistinguishable from one whose generator produces nothing interesting — a mutation can appear to
*pass* only because the module under test contained a duplicate definition and the mutation was
applied to the copy that never ran, which is exactly the false confidence this guards against.

## Success criteria

1. Every case above is executable and named in `expectations/`.
2. Each case has been observed failing against a build without its rule.
3. Adding a corpus requires no code change — only a directory and a config, which is the same
   claim SPEC-0003 makes, tested from the other side.

## Open questions

- **How is a corpus refreshed** once new defect classes are found? A refresh must diff expected
  results against the previous run, not just regenerate records, or a silent change in expectations
  could pass unnoticed.
- **Does the permanent-ceiling case belong in an executable suite at all?** It asserts something is
  *not* mechanically detectable. Encoding it as a passing test risks reading as "handled".

## References

- SPEC-0001 — the acceptance suite as originally listed.
- SPEC-0002 — criterion 3 depends on this.
- RFC-0011 §5 — planted-violation requirement for absence assertions.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-08-20 | Split out of SPEC-0001 §"Acceptance test". | — |
