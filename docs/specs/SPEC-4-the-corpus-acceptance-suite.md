---
Version: '0.7'
Date: 2026-08-20
Status: Draft
Author: beauwilliams
Subject: 'The corpus acceptance suite -- documented bug classes `check` must reproduce against real corpora.'
Parent: SPEC-1
---
# SPEC-4 — The corpus acceptance suite

## Purpose

A validator is only as trustworthy as the defects it has been shown to catch. Propose an executable
acceptance suite built from documented, anticipated bug classes — each with a known-correct
outcome — run against synthetic corpora designed to exercise real corpus variance (irregular
formatting, wrapped fields, mixed header shapes) rather than only clean, hand-crafted fixtures.
`Parent`'s target, `SPEC-1`, lists the bug classes this suite must reproduce.

This spec turns that bug-class catalog into an executable suite, and is a prerequisite for
SPEC-2's criterion 3.

**It is deliberately not on the bootstrap path.** SPEC-2 Phase A closes the self-hosting loop
against this repository's own `docs/` — corpus zero — which needs nothing here. The synthetic
corpora take real engineering effort to build well; the fastest deliverable must not queue behind
the slowest one. This is Phase D.

## Structure

```
corpora/
  <corpus-id>/
    records/          synthetic record files, engineered to exhibit specific variance
    manifest.yaml     what irregularity each record was built to exhibit
    .urzua/           the config this corpus is validated under
    expectations/
      <case-id>.yaml  one per acceptance case
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
- `manifest.yaml` records what irregularity each fixture was built to exhibit, so a failing case can
  be reasoned about without guessing at intent.

## Acceptance cases

Each case is a corpus, a config, an invocation, and an expected result:

```yaml
id: "blank-field-misdetection"
corpus: "corpus-a"
invocation: ["check", "records/"]
expect_exit: 1
expect_findings:
  - rule: "field.blank-vs-placeholder"
    file: "..."
    line: 12
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
files-examined / rules-executed report — which is why SPEC-2's no-silent-no-op criterion was
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

This is the only executable test of RFC-1 §1a's "profiles are declared, not enumerated by the
tool". The claim is otherwise asserted and unfalsifiable; running it converts the claim into an
observation. If adding the next type is not cheap, the profile mechanism is not finished.

### Corpus zero — this repository

`docs/` is a corpus with the property none of the synthetic ones have: it is live, and its defects
are found by the tool as they are introduced rather than reconstructed after the fact.

| Case | What |
|---|---|
| Three header formats in one corpus | RFC and ADR share a shape; SPEC-1 uses it while SPEC-2 to SPEC-4 use a pipe-delimited line with bold keys. |
| Six `Implements:` pointers at `Draft` records | Every RFC that `docs/specs/` declares it implements is undecided. |

Both are the seed rules for SPEC-2 Phase A. Neither is fixed ahead of the checker on purpose: a
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

## First runner (MILE-101)

The narrow first version is a Rust integration test, `rust/crates/urzua-cli/tests/acceptance.rs`
(`harness = false`, so its report always prints), run by `cargo test` and therefore `make ci`. Corpora
live in `rust/crates/urzua-cli/tests/acceptance/corpora/<corpus-id>/` with the layout above. What is
built, and where it differs from the sketch:

- A case's `invocation` runs with the corpus copied into a throwaway git repo as its working directory,
  so paths are relative to the corpus root and the configuration is whatever the corpus holds. Pass an
  alternate config with the global `--config` flag. Optional `setup` is a list of invocations run first
  (e.g. `init`).
- `corpus: "@repo"` runs in place against this repository (corpus zero); nothing is copied.
- Expectations: `expect_exit`, `expect_stdout_contains`, `expect_json` (`at` is a JSON pointer, with
  `equals` or `gt`), `expect_findings` / `expect_no_findings` (match on `rule`, `file`, `severity`).
- `manifest.yaml` is read: every fixture file must be listed with a non-empty irregularity and every
  listed path must exist, so the record of intent cannot rot.
- `known_gap: {ref, reason}` marks a case whose expectation is the correct behaviour that does not
  hold yet. It still runs; failing is the expected outcome and is reported `KNOWN-GAP`. Passing fails
  the suite until the marker is removed, so a gap cannot go stale and a fix flips the case visibly.
- **Private corpora.** `URZUA_ACCEPTANCE_EXTRA` is a `:`-separated list of directories, each either one
  corpus (it has `expectations/`) or a root of corpus directories, laid out as above. They run in
  addition to the checked-in ones. Unset, the runner prints a one-line skip note rather than staying
  silent. This lets a maintainer run fixtures that cannot be published against the same runner.

### Added by MILE-170

- **Mutation proof.** Every case names the rules it proves: the rules in its `expect_findings` and
  `expect_rules_ran`, plus any listed in `proves`. After the case passes, the runner re-runs it with
  each of those rules set to `off` in the config the case runs under (after `setup`, so an `init`ed
  config is mutated too) and the case must then fail. This is "a case that passes against a build with
  the rule removed is not a case", executed on every run. A mutation that makes the config invalid
  (exit 2) is a harness failure, not a proof. A case that asserts something no single rule produces
  (a positive control, a config error, `init` output) gives `mutation_exempt` a reason instead; a case
  with neither is rejected. `expect_rules_ran` asserts the same thing for each rule it lists (status
  `ran` over a non-empty population), so only its first rule is switched off; the others count once that
  held, which keeps a case over corpus zero to two runs.
- **Coverage report.** An unfiltered run prints how many rules in `ALL_RULES` have a case whose
  mutation proof held and lists every rule that has none (`uncovered  <rule>`). It reports and does
  not fail: a rule added by another change has no case yet, and the list is where that is seen.
- **More expectations.** `expect_rules_ran`, `expect_mirrored` (two path prefixes whose findings name
  the same non-empty set of rules), and `message_contains` on a finding matcher. Cases run on a few
  threads and an identical read-only run over `@repo` is made once.
- **Corpora.** `corpus-d` plants one irregularity per field-quality and structural rule (blank,
  pending, untrimmed and case-variant fields, an undeclared status, a body bullet standing in for a
  header field, a wrapped value, a comment line inside the header, a one-sided supersession, an
  annotation that cites references, a missing section, a prose citation and a cited path that do not
  resolve, a record one directory below its type's `dir`, a mistyped `dir` beside a working type).
  `corpus-e` is the "Nth record type" case. `corpus-f` covers the shape, forbidden-field,
  case-variant-key and cross-type rules. Corpus zero has one case per rule family, each asserting
  that the family's enabled rules run over a non-empty population and that the whole check is clean.
- **The "Nth record type" case.** `corpus-e` declares a throwaway type, `gizmo`, named nowhere in the
  source, beside `adr`. The same planted violations sit in both type directories and must be met by the
  same rules; the control run with the declaration removed examines none of the gizmo files.
- **The generative property** is `rust/crates/urzua-cli/tests/acceptance_property.rs`, run by
  `cargo test`. A fixed-seed generator makes it reproducible. Field names from a wide alphabet are each
  accepted as declared or reported by name; wrapping a record's body in every fence variant, a comment
  or a quote leaves the verdict unchanged; relation values with annotations that cite references
  round-trip to the reference that begins each entry. Each property is also run with its rule off (or,
  for invariance, against an input that really changes the verdict) and must then fail.

### Added by the rest of MILE-170

- **Commits, `--base` and shallow runs.** A case may list `commits`, each writing, renaming or removing
  files and committing; the fixture commit is tagged `base`, so `check --base base` gives the changed-path
  rules a diff. `shallow: true` runs the case in a `--depth 1` clone made after the commits.
  `expect_notices` matches the report's notices by `subject` and `message_contains`. Git history is part
  of the fixture, so a case's commits are written in its own file, not in the corpus.
- **`corpus-g` (git history).** A shallow clone yields the `embodiment-drift-git` notice and not a clean
  zero; a value-only edit of a cited file (same length, same token counts, so invisible to an
  occurrence-count query) is drift; a renamed record keeps its prior state; drift at warning severity under
  `fail_on: warning` is a blocking exit, never exit 0; a change to `src/queue.rs.bak` and `src/queue_ext.rs`
  is not drift of the cited `src/queue.rs` (the derived-value case: a near-neighbour query computes a
  plausible wrong answer); a `Implements:` back-pointer to an identifier that only a record's prose
  mentions is dangling (identifiers are defined by role, not mention); a locator two records cite is a
  promotion candidate.
- **`corpus-h` (gates and ceiling).** `diff.source-requires-record` and `diff.source-and-records-mixed`
  over a second commit; `boundary.published-path-reference`; `citation.issue-exists` against a snapshot
  checked into the corpus (no network); `status.delivery-agreement` in both directions; and the permanent
  ceiling: a required `Context` section holding off-topic text passes `section.required`, because no
  mechanical check can say the text is about the decision. That case records the limit; it does not claim
  the gap is handled.
- **`corpus-i` (one namespace across directories).** One `adr` type declaring `extra_dirs`: a pointer
  resolving into the other directory, an id shared by records in both (`identity.collision`), a file
  with no identity in the extra directory, a declared extra directory that does not exist, and
  `urzua new` numbering past the highest id in either (`MILE-130`). urzua's own `.urzua/config.yaml`
  has no type spanning directories, so no natural own-config use exists and the corpus is the exercise.
- **Coverage.** Every rule in `ALL_RULES` now has a case whose mutation proof holds, and the report lists
  none uncovered.

Not built: mutation of a defect reintroduced into the source rather than a rule switched off, and the
"silent zeros" cases whose defect lives in a git query this build does not make (the rename case is covered
through the record's blame history; the `-S`/`-G` case through the value-only edit).

## Success criteria

1. Every case above is executable and named in `expectations/`.
2. Each case has been observed failing against a build without its rule.
3. Adding a corpus requires no code change — only a directory and a config, which is the same
   claim SPEC-3 makes, tested from the other side.

## Open questions

- **How is a corpus refreshed** once new defect classes are found? A refresh must diff expected
  results against the previous run, not just regenerate records, or a silent change in expectations
  could pass unnoticed.
- **Does the permanent-ceiling case belong in an executable suite at all?** It asserts something is
  *not* mechanically detectable. Encoding it as a passing test risks reading as "handled".

## References

- SPEC-1 — the acceptance suite as originally listed.
- SPEC-2 — criterion 3 depends on this.
- RFC-11 §5 — planted-violation requirement for absence assertions.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-08-20 | Split out of SPEC-1 §"Acceptance test". | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-09-17 | Config moves from `.urzua/config.toml` to `.urzua/config.yaml` (`ADR-52`, shipped in the same change). The described mechanism changes, not just its rendering. | **substantive** |
> | 2026-09-18 | Stays `Draft`, deliberately. **Why:** unlike `SPEC-1`/`2`/`3`, this spec's subject does not exist. No code reads `manifest.yaml` or a per-case `<case-id>.yaml`, and the only test files in the workspace are `check_integration.rs`, `new_integration.rs` and `purity.rs`. `Draft` is the accurate status, and flipping it with the others would have been the bulk edit `BUG-26` warned against. | **structural** |
> | 2026-11-02 | Bumped to `0.4`. **Why:** MILE-101 built the first runner; the spec now records the shape that met a real case: the runner location, the case fields actually supported, `known_gap` markers, a read `manifest.yaml`, and the `URZUA_ACCEPTANCE_EXTRA` private-corpus hook. | **substantive** |
> | 2026-11-04 | Bumped to `0.5`. **Why:** `MILE-170` built the rest of the harness: mutation proof of every case, the coverage report of rules with no case, the "Nth record type" case, the generative property, corpora `corpus-d` to `corpus-f` and one corpus-zero case per rule family. Recorded under "Added by MILE-170"; the success criteria are unchanged and criterion 2 now holds for every case by construction. | **substantive** |
> | 2026-11-04 | Bumped to `0.6`. **Why:** `MILE-170` finished the harness: cases can make commits, pass `--base` and run shallow; `corpus-g` and `corpus-h` add the git-history, drift, derived-value, diff-gate, boundary, issue-snapshot, delivery and permanent-ceiling cases; every shipped rule now has a proven case. Recorded under "Added by the rest of MILE-170". | **substantive** |
> | 2026-11-04 | Bumped to `0.7`. `corpus-i` (one type over two directories) added: pointer resolution, collision, identity filename and missing-directory cases, and `urzua new` numbering across both. **Why:** `MILE-130` -- urzua's own repository has no type spanning directories, so the corpus is where the feature is exercised. | **substantive** |
