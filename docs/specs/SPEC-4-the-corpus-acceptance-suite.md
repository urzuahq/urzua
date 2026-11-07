---
Stable-Id: 01M6R3NK5N2PD9ZYBJPZHN05W7
Version: '0.15'
Date: 2026-08-20
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/tests/acceptance.rs, test:rust/crates/urzua-cli/tests/acceptance_property.rs
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
                      (and any other fixture directory a case needs, such as src/)
    manifest.yaml     what irregularity each fixture was built to exhibit
    .urzua/           the config this corpus is validated under
    expectations/
      <case-id>.yaml  one per acceptance case
```

Corpus ids are opaque (`corpus-a` to `corpus-l`; `corpus-zero` is this repository, held in place). Nothing in `corpora/` may name an
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
id: "blank-field-is-not-absent"
corpus: "corpus-d"
invocation: ["check"]
expect_exit: 1
expect_findings:
  - rule: "field.quality"
    file: "records/ADR-0002-blank-author.md"
    severity: "error"
    message_contains: "'Author' is Blank"
```

The suite is red before the rule exists and green after. A case that passes against a build with the
rule removed is not a case.

### Field-quality and structural cases

The bug classes below are the catalog the suite was proposed from. The right-hand column says where each is
exercised now, by `corpus/case-id`. Some are exercised by the nearest case the built corpora hold, and a few
are not an acceptance case at all; those say so.

| Case | Built as |
|---|---|
| Blank-field misdetection | `corpus-d/blank-field-is-not-absent` |
| Provisional-status misclassification | `corpus-d/pending-field-is-not-blank` and `corpus-d/accepted-requires-its-declared-fields` (the nearest cases) |
| Reciprocity-checker data loss | `corpus-d/one-sided-supersession-is-reported`; `corpus-b/audit-reports-one-sided-supersession` holds the bare-number variant, which `audit` answers with rename guidance |
| Decoy-corpus silent no-op | `corpus-d/mistyped-dir-beside-a-working-type-is-reported`, `record-in-a-subdirectory-is-not-silently-ungoverned`, `all-rules-run-over-a-real-population` |
| Regex invalidated by realistic corpus variance | `corpus-d/wrapped-field-value-is-read-whole`, `lower-case-key-is-a-case-mismatch`, `untrimmed-value-is-reported` |
| Filename/title mismatch | `corpus-d/filename-and-title-number-must-agree` |
| Enum completeness against corpus prose | `corpus-d/status-with-free-text-tail-is-undeclared` covers a status outside the declared enum; the completeness of an enum against prose is not an acceptance case |
| Check exists vs check wired | Not an acceptance case: `doctor`'s CI-wired check is tested in `check_integration.rs` |
| Discovery scoped to VCS, not filesystem | Not an acceptance case: `discovery_untracked_integration.rs` |
| Pre-retrofit reverse-reference scan | Not built |
| Structural presence vs content scope (permanent ceiling, asserted as *not* mechanically closable) | `corpus-h/structure-without-content-is-not-closable` |
| Header field satisfied by a body bullet | `corpus-d/body-bullet-does-not-satisfy-a-header-field` |
| Multi-line continuation silently truncated | `corpus-d/wrapped-field-value-is-read-whole` |
| Header slice truncated at the first heading | `corpus-d/header-is-not-cut-at-a-hash-line` |
| Machine channel reports `ok` while prose says "nothing to compare" | `corpus-g/shallow-clone-is-a-notice-never-a-clean-zero` (the nearest case); `corpus-g/a-scope-changed-rule-judges-nothing-on-a-plain-check-and-says-why` |
| Agent invents a schema field mid-edit | `corpus-d/invented-field-is-undeclared` |
| Gate reports drift while exiting 0 | `corpus-g/drift-is-reported-with-a-blocking-exit` |
| A derived value computed by a near-neighbour query | `corpus-g/neighbouring-file-change-is-not-drift-of-the-cited-file` |
| A number holding no decision fails checks presupposing one | Not built |

The last two catalog entries are the strongest cases in the suite as proposed, because both produce **well-formed, plausible,
confidently wrong output** rather than an error, the class no smoke test catches. The first is built; the second is not.

### Silent zeros from a derived rule

Four cases, one shape: the rule runs, examines every record, and returns a clean zero because the
**evidence it reasons over** was empty. None raises an error, and three of the four pass a
files-examined / rules-executed report — which is why SPEC-2's no-silent-no-op criterion was
tightened to report each rule's input population.

| Case | What the rule reports | Built as |
|---|---|---|
| History query stops at a rename (`git log` without `--follow`) | 0 of 91 records had readable prior state; corpus reported fully clean | `corpus-g/renamed-record-keeps-its-prior-state` |
| `-S` (occurrence count) used where `-G` (line content) was needed | Every value-only edit invisible, so no field change was ever detected | `corpus-g/value-only-edit-of-a-cited-file-is-drift` |
| A record's *filename* pattern reused to resolve *references* | A reference written in prose matched nothing; the rule could not fire | `corpus-d/prose-citation-of-a-missing-record-is-reported` |
| Defined identifiers collected by mention rather than by definition | A typo'd back-pointer **defined its own target** and validated against itself | `corpus-g/dangling-back-pointer-is-not-defined-by-a-mention` |

The last is the most instructive: gathering "which identifiers exist" by scanning for the identifier
pattern makes the validation circular, so the rule can never fire for anyone. Definitions must be
recognised by syntactic role — a heading, a table's key column — not by the identifier appearing.

### Architecture claims, not behaviours

One case tests whether a *design* holds rather than whether an input produces an output.

| Case | What it asserts |
|---|---|
| The Nth record type costs a declaration, not a checker | Declare a throwaway record type as data and nothing else; it must receive the full rule set, with planted violations firing, and **no code change** (`corpus-e/nth-record-type-costs-a-declaration-not-a-checker`) |

This is the only executable test of RFC-1 §1a's "profiles are declared, not enumerated by the
tool". The claim is otherwise asserted and unfalsifiable; running it converts the claim into an
observation. If adding the next type is not cheap, the profile mechanism is not finished.

### Corpus zero — this repository

`docs/` is a corpus with the property none of the synthetic ones have: it is live, and its defects
are found by the tool as they are introduced rather than reconstructed after the fact. It is run in place
(`corpus: "@repo"`) under this repository's own `.urzua/config.yaml`; its cases are listed under "Added by
MILE-170" below.

The two seed cases this section once listed, three header formats in one corpus and `Implements:` pointers
at `Draft` records, no longer exist: every record here is now YAML frontmatter, and the corpus is held at
zero unwaived findings, so neither defect can be shown on it. A corpus with no defects cannot demonstrate
that a checker works, which is why the synthetic corpora carry the planted violations and corpus zero
carries the assertion that the real rules run over a real population and find nothing.

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
  `equals` or `gt`), `expect_findings` / `expect_no_findings` (match on `rule`, `file` as a path suffix, `severity` and `message_contains`; also `suggested_action`, a subset the finding's action must contain, `no_suggested_action` and `unwaived`, which skips a finding a waiver suppresses).
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
- **Coverage report.** An unfiltered run states, for every rule in `ALL_RULES`, how far the suite
  proves it, in three tiers that are never added together. A rule has a **failing case** when a case
  names it in `expect_findings` and fails with the rule off: the suite shows the rule can fire. A rule
  **ran clean only** (`ran-only  <rule>`) when its sole proof is `expect_rules_ran` or `proves`: a case
  shows the rule executes over a population, and none expects it to report anything, so the suite does
  not show it can fire. A rule with **no case** is listed as `uncovered  <rule>`. The count of rules with
  a failing case is the number the suite proves; the other two are what it does not. A rule
  with no failing case here may still have a unit or integration test that plants its violation; this
  report is about the acceptance layer only.
- **Coverage gate.** The same unfiltered run fails when a rule in `ALL_RULES` has no failing case and is
  not on the committed allow-list, `tests/acceptance/coverage-allowlist.yaml` (decided 2026-11-07). Each
  entry is a `rule` id and a `reason`; an entry with no reason fails. The list only shrinks: the gate
  holds a fixed ceiling of the ids the list may name (`CEILING` in `tests/acceptance/coverage_gate.rs`),
  so an entry for any other rule is refused, and a rule the table gains has to come with a failing case
  rather than a listing. An entry is stale, and fails, once its rule has a failing case or has left the
  rule table, so the entry is deleted in the same change that earns or removes it. A name-filtered run
  does not run the gate, for the same reason it does not report coverage. `acceptance_coverage_gate.rs`
  tests the gate, including that a new rule without a failing case fails it.
- **Scratch names.** A case's temporary repository and its mutated config are named by process, corpus
  and case id. Cases run on several threads, and two corpora (a private one merged in through
  `URZUA_ACCEPTANCE_EXTRA`, say) may each declare a case with one id, so a name keyed on the id alone would
  be a directory the second case deletes from under the first. The runner fails a run in which two queued
  cases would share one, and `corpus-c` and `corpus-m` carry a case of the same id to exercise it.
- **More expectations.** `expect_rules_ran`, `expect_mirrored` (two path prefixes whose findings name
  the same non-empty set of rules), and `message_contains` on a finding matcher. Cases run on a few
  threads and an identical read-only run over `@repo` is made once.
- **Corpora.** `corpus-d` plants one irregularity per field-quality and structural rule (blank,
  pending, untrimmed and case-variant fields, an undeclared status, a body bullet standing in for a
  header field, a wrapped value, a comment line inside the header, a one-sided supersession, an
  annotation that cites references, a missing section, a prose citation and a cited path that do not
  resolve, a record one directory below its type's `dir`, a mistyped `dir` beside a working type).
  `corpus-e` is the "Nth record type" case. `corpus-f` covers the shape, forbidden-field,
  case-variant-key and cross-type rules. Corpus zero has one case per rule family (`own-config-<family>-rules-run`), each asserting
  that the family's enabled rules run over a non-empty population and that the whole check is clean,
  and two more: its `docs/` has no unwaived error, and its findings that declare no action carry none.
- **The "Nth record type" case.** `corpus-e` declares a throwaway type, `gizmo`, named nowhere in the
  source, beside `adr`. The same planted violations sit in both type directories and must be met by the
  same rules; the control run with the declaration removed examines none of the gizmo files.
- **The generative property** is `rust/crates/urzua-cli/tests/acceptance_property.rs`, run by
  `cargo test`. A fixed-seed generator makes it reproducible. Field names from a wide alphabet are each
  accepted as declared or reported by name; wrapping a fake header in a record's body in every fence variant, a comment
  or a quote leaves the verdict unchanged (the wrapped content follows the real header; content before the header is not generated); relation values with annotations that cite references
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
- **`corpus-h` (gates and ceiling).** `diff.source-requires-record` (including its decision gate, `MILE-133`) and `diff.source-and-records-mixed`
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
- **Coverage.** Every rule in `ALL_RULES` had a case whose mutation proof held when `MILE-170` finished,
  but 22 of them held it through `expect_rules_ran` or `proves` alone, so the count of 57 of 65 overstated what the
  suite showed: it counted a rule that ran as a rule that can fire. Later rules were added with no case at
  all. The report now keeps the tiers apart (see "Coverage report"). Rules with a failing case grew from 35
  of 65 to 58 with `corpus-m` to `corpus-t` (see "Added by the hardening of CI and release tests"). Rules with a failing case are now 60 of 65, with
  `corpus-u` (`narrative-field.stale`, `waiver.matches-nothing`). Five rules remain on the allow-list (see
  "Coverage gate"): `claim.status-agreement` and `embodiment.back-pointer-decision-accepted` ran clean only;
  `index.freshness` has no mutation-provable case, because switching the rule off leaves its `indexes`
  option on a disabled rule and the config is rejected; `config.schema-version-outdated` has a failing case
  that cannot be mutated, because a version-1 config has no `rules` table to switch the rule off in; and
  `identity.open-pr-collision` has none, because a case cannot create the pull-request refs it reads. Each
  has a planted-violation test at the integration or unit level. The counts are the run's output when this was written; the coverage report is authoritative.

### Added since

- **`corpus-j` (status transitions).** The declared transition graph: an allowed edge is clean, an edge the
  graph does not list is reported, a terminal status cannot be left, a new record must start at an initial
  status, a renamed record keeps its committed status, a whole-repository run judges no transition, a status
  no edge reaches is reported and a graph naming an undeclared status is refused.
- **`corpus-k` (schema version).** The current configuration schema version loads clean, the previous one
  loads with one warning, a version newer than the current or older than the window is a structured error,
  and `migrate config` reports every key it touched and does nothing on a second run.
- **`corpus-l` (sequencing).** The roadmap over a declared acyclic pair: a cycle, including one across two
  declared pairs, is reported and never looped on, a claim that resolves to nothing is reported, a pair no
  record carries a value for is reported, a dependency in a failure terminal keeps its dependents out, the
  critical path is the smallest id sequence among the longest chains, and a repository with no acyclic pair
  has no roadmap.
- **`corpus-a`, `corpus-b`, `corpus-c`.** The bold-list and blockquote header shapes were removed (`MILE-85`),
  so `corpus-a` now holds the cases that name the removed form: `check` and `init` say so, and a
  configuration that declares it is rejected at load. `corpus-b` holds the rename guidance `audit` gives a
  bare-numbered corpus. `corpus-c` is the clean YAML frontmatter positive control.

Not built: mutation of a defect reintroduced into the source rather than a rule switched off, and the
"silent zeros" cases whose defect lives in a git query this build does not make (the rename case is covered
through the record's blame history; the `-S`/`-G` case through the value-only edit).

### Added by the hardening of CI and release tests

Failing cases for rules that had only a clean run or no case, each with a mutation proof that holds:

- **`corpus-m` (sequencing).** A three-record cycle in an acyclic `Depends-on` pair (`sequence.cycle`),
  a dependency on a record that does not exist (`sequence.unresolved-blocker`) and an `InProgress`
  record blocked by an unfinished dependency (`sequence.status-agreement`).
- **`corpus-n` (version bump).** Two specs each gain a substantive revision row after the base commit;
  only the one that leaves `Version` alone is reported (`revision-log.version-bump-required`).
- **`corpus-o` (who decides).** A move to `Accepted` authored outside the deciders group, read from an
  actors file in the corpus, is reported, and the same move authored by a decider is not
  (`diff.decider-required`).
- **`corpus-p` (override log).** An override row with no reviewer is reported on the log
  (`override.unreviewed`).
- **`corpus-q` (configuration declarations).** One config per rule, each selected with `--config`:
  a companion pair with one side undeclared, a relation role naming an undeclared field, a header-less
  type with required fields, a type with no `known_fields`, a pointer field the type does not know, a
  pointer list without a narrative list, a field in both lists, and a declared directory whose only file
  carries no identity (`config.scope-matches-nothing`).
- **`corpus-r` (field values and dates).** A value starting with a reserved YAML indicator
  (`field.leading-reserved-indicator`) and an acceptance dated before the record
  (`date.field-order`).
- **`corpus-s` (relations and pointers).** A supersession claim the other record does not return
  (`relation.pair-reciprocity`), a pointer to a record in a status the rule rejects
  (`pointer.target-status`), a pointer to a record whose type declares no status
  (`relation.target-status-undeclared`), prose in a pointer field (`header.pointer-field-clean`) and a
  type with no declared spec (`type.no-declared-spec`).
- **`corpus-t` (locators).** A locator naming a file that does not exist (`embodiment.locator-exists`)
  and an embodiment stated before the record is accepted (`embodiment.requires-accepted-status`).

## Success criteria

1. Every case in the suite is executable and named in `expectations/`. A catalog entry marked "Not built" is
   not yet part of the suite.
2. Each case that proves a rule has been observed failing with that rule switched off, on every run (the
   mutation proof). A case that proves no rule, such as a positive control or a config error, says why in
   `mutation_exempt`.
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
> | 2026-11-05 | Bumped to `0.8`. `corpus-h` gains the declared-boundary cases: a published boundary whose declared severity beats the rule's level, a back-pointer inside a generated boundary (and a plain id there that is not reported), a vendored boundary that prohibits nothing, an ignore marker that suppresses nothing, and an exclude and a boundary path that match nothing. **Why:** `MILE-125`; each case names `boundary.published-path-reference` and its mutation proof holds. | **substantive** |
> | 2026-11-05 | Bumped to `0.9`. `corpus-h` gains the decision-gate cases of `diff.source-requires-record`: a source change beside a Proposed decision is held, and one beside an Accepted decision, a milestone implementing one or a bug record is released. **Why:** `MILE-133`. | **substantive** |
> | 2026-11-07 | Bumped to `0.10`. Checked against `acceptance.rs`, `acceptance_property.rs` and the corpora, and corrected: the example case uses the matcher fields the runner reads (it had `line`, which the runner rejects) and a real case; the catalog tables say where each bug class is built and which are not, so success criterion 1 no longer claims every catalog entry is a case; the corpus zero section no longer lists two seed cases that cannot exist on a corpus held at zero findings; the coverage statement says rules without a mutation-proof case exist and that the run's report, not the spec, holds the count and the list, where it claimed none were missing; `corpus-j`, `corpus-k`, `corpus-l` and the other matcher fields are documented. **Why:** the maintainer approved moving this spec to `Shipped` (2026-11-07), which required verifying each section against the built suite. | **substantive** |
> | 2026-11-07 | Bumped to `0.11`. Status changed from Draft to Accepted. **Why:** Maintainer decision, 2026-11-07: the maintainer approved moving this spec to `Shipped`, and the lifecycle moves one step at a time (`Draft` to `Accepted` to `Shipped`). | **substantive** |
> | 2026-11-07 | Bumped to `0.12`. Realized-by names the implementing code and tests, ready for `Shipped` (`MILE-7`). **Why:** Maintainer decision, 2026-11-07: moving this spec to `Shipped` requires `Embodiment` and `Realized-by`. | **structural** |
> | 2026-11-07 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/tests/acceptance.rs, test:rust/crates/urzua-cli/tests/acceptance_property.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-07 | Bumped to `0.13`. Status changed from Accepted to Shipped. **Why:** Maintainer decision, 2026-11-07: its `Realized-by` evidence computes `Verified`, and each section was checked against the built code and tests. | **substantive** |
> | 2026-11-07 | Bumped to `0.14`. The coverage report keeps three tiers apart: rules with a failing case, rules that ran clean only (`ran-only`) and rules with no case (`uncovered`), and no longer adds them into one figure; the "Coverage" claim that every rule has a case and none is uncovered is replaced by the count, and the seven rules without a failing acceptance case are named. Scratch repositories and mutated configs are named by corpus as well as case, and the runner fails a run in which two cases would share one. `corpus-m` to `corpus-t` add failing cases for 23 rules. **Why:** the report said 57 of 65 rules were proven, but 22 of them were counted from `expect_rules_ran` or `proves`, which show a rule runs and not that it can fire, so the figure overstated the proof and the spec claimed more than the suite showed; and a case id shared by two corpora made concurrent cases delete each other's repository. | **substantive** |
> | 2026-11-07 | Bumped to `0.15`. The coverage report is now a gate: an unfiltered run fails when a rule has no failing case and is not on the committed allow-list, each entry carrying a rule id and a reason, and the list may only shrink (an entry outside a fixed ceiling is refused, and an entry for a rule that gained a case or left the table is stale). The allow-list holds the five rules still without a failing case; `corpus-u` adds failing cases for `narrative-field.stale` and `waiver.matches-nothing`. **Why:** maintainer decision, 2026-11-07: the report only reported, so a rule could be added with no failing case and CI stayed green. | **substantive** |
