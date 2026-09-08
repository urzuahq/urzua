---
Version: '0.1'
Date: 2026-07-29
Status: Draft
Embodiment: Not started
Author: '@beauwilliams'
Implements: ADR-1, ADR-2, ADR-3
Derives-from: RFC-1 (Accepted)
---
# SPEC-1 — `urzua` v0 CLI

## Purpose

Ship the smallest thing that is **genuinely used in two real codebases** and proves the founding
claim — one implementation instead of three — with documented bug histories from independently
built tooling as its acceptance test.

**v0 is the substrate, deliberately.** The market wedge is backfill and the moat is drift
detection, and that schema is substrate you never lead with. That's correct for selling and wrong
for dogfooding: the target codebases already have records, so backfill gives them nothing on day
one. The substrate is what you use tomorrow; the wedge is what you sell later. It gets built first
because you cannot sell a decision graph you have never run against a real corpus.

> **This spec is the parent of a sequential set, since 2026-08-20.** It keeps the cross-cutting
> material every other record cites — success criteria, the acceptance-suite bug list, the
> no-silent-no-op rule, and the permanent content-scope ceiling — and the buildable units now have
> their own specs:
>
> | Spec | Covers | Status |
> |---|---|---|
> | **SPEC-2** | `urzua check` — discovery, rules, output contract | **the Phase 0 deliverable** |
> | **SPEC-3** | `.urzua/config.toml` | needed by SPEC-2 |
> | **SPEC-4** | the corpus acceptance suite | needed by SPEC-2 criterion 3 |
> | **SPEC-5** | `urzua init` — type selection, `.urzua/` layout, the adopt path | the bootstrap surface |
> | **SPEC-6** | the `milestone` record type | Accepted |
> | **SPEC-8** | `urzua fix` — eligibility test, detect/apply | Accepted |
> | **SPEC-9** | the `bug` record type | Accepted |
> | **SPEC-10** | the `waiver` record type | Accepted |
> | **SPEC-11** | `urzua audit` | Accepted |
> | **SPEC-12** | `urzua new` | Accepted |
> | **SPEC-13** | `urzua explain` / `urzua graph` | Accepted |
> | **SPEC-14** | `urzua migrate ids` / `urzua migrate schema` | Accepted |
> | **SPEC-15** | `urzua doctor` | Accepted |
>
> Its number and identity are unchanged deliberately. Roughly thirty-five records, comments and
> source files cite `SPEC-1` for the rules above; renumbering or repurposing it would be the
> reverse-reference blast radius this spec's own acceptance suite records as a bug class. Identity
> is stable, and the split is additive — RFC-11 §6, applied to this document. `SPEC-7` is vacated
> (retracted into SPEC-6 v0.2, ADR-14) and deliberately not reused for any of the above, per RFC-13's
> principle that a vacated number is a record, not a gap to silently fill.

**Explicitly out of scope for v0:** escalation/paging (blocked on an open question — building
the escalation model before knowing what teams actually do is guessing at the hardest part),
backfill (the market wedge, not the dogfood wedge), any dashboard or service, and LLM-assisted
anything. v0 is mechanical, offline, and deterministic.

## Success criteria

v0 is done when, and only when:

1. It runs green in **at least two** real target codebases, with different primary languages, via
   pre-push hook and CI, with zero host-repo runtime added to either.
2. It reproduces **every documented bug** in the acceptance suite against real corpora
   (see §5). Not synthetic tests only — fixtures engineered to exhibit the same variance a real
   corpus does.
3. It replaces the hand-written linter in at least one codebase outright, with that linter deleted.
4. `urzua new` is what people and agents actually invoke, because hand-authoring is now the slower
   path.

Criterion 3 is the real bar. A tool that runs *alongside* the thing it was meant to replace has
failed, whatever its test coverage says.

## Commands

### `urzua init`
**Specified in full by SPEC-5.** Selects which record types the repository keeps, writes
`.urzua/` with config and templates, and adopts an existing corpus in place without moving files.
Absent from this spec's original command surface — the gap surfaced when the bootstrap plan needed
it.

### `urzua new <type> [title]`
**Specified in full by SPEC-12.** Creates a record from the configured template with a stable ID
assigned, never asking the author to pick a number.

### `urzua check [paths...]`
**Specified in full by SPEC-2.** The validator. Exit non-zero on error; stdout is always the JSON
report, unconditionally (ADR-23) — no flag needed for CI or an agent to get structured output.
Covers the rule set documented in the acceptance suite:

- **Field presence with real semantics** — blank ≠ placeholder ≠ pending are three distinct states,
  not one. This is a commonly rediscovered gap in hand-written record linters and is the
  single most-repeated bug class.
- **Status enum validation tolerant of free-text annotation** — a rule written against an assumed
  canonical format can be invalidated by real corpus variance; the rule must be
  tested against a real corpus before it is trusted.
- **Role rules** — Author/Reviewers/Deciders presence requirements per record type and status, with
  self-acknowledgement detection (an author cannot be their own reviewer of record).
- **Filename ↔ title ↔ display-number ↔ stable-ID consistency** (ADR-3).
- **Required-section presence** per profile, including `change_class` on revision entries.

**Non-negotiable: no silent no-op.** A check that finds nothing must be distinguishable from a check
that ran on nothing. `check` reports files examined and rules executed, and exits non-zero if it
matched zero files when paths were given. One source implementation reported "0 errors" while
silently examining an empty set — a zero-error report that isn't evidence the check ran is the
failure mode this tool exists to eliminate, and shipping it would be self-refuting.

### `urzua explain <path>` / `urzua graph`
**Specified in full by SPEC-13.** Read-only relationship queries over data `check` already parses.

### `urzua audit`
**Specified in full by SPEC-11.** Cross-record reconciliation, separate from per-record validation:
supersession reciprocity and dangling cross-references. Never writes.

### `urzua fix`
**Specified in full by SPEC-8.** Detects fields whose stated value disagrees with what the tool
computes from evidence already in the record; apply mode writes the computed value back, gated hard.

### `urzua migrate ids` / `urzua migrate schema`
**Specified in full by SPEC-14.** Backfills stable identifiers onto existing records, and previews
what a newly-required field would break before it's added to config.

### `urzua export --format=agdr` / `urzua import`
AgDR compatibility (ADR-2). Lossy export warns rather than silently dropping fields.

## Configuration

**Specified in full by SPEC-3**, which moved it to `.urzua/config.toml`. **Config-driven is the whole thesis** — the same binary must serve a
Python repo and a TypeScript monorepo with different directory layouts, different record types, and
different role requirements, without either forking it. If v0 needs code changes to work in the
second codebase, the founding claim is falsified and that's worth discovering in week two rather
than year two.

Configurable: record types and their directories, required fields per type and status, role
requirements, status enums, back-pointer patterns, and which checks are errors vs. warnings.

## Monorepo layout

Polyglot, organized by language at the root (ADR-4). The repo is
`github.com/urzuahq/urzua`; the Rust workspace is one directory inside it, not the root.

```
rust/
  Cargo.toml              workspace
  crates/
    urzua-core/           schema, record parsing, validation rules — pure, no I/O
    urzua-id/             stable ID generation and resolution (ADR-3)
    urzua-agdr/           AgDR import/export (ADR-2)
    urzua-cli/            command surface, config loading, output formatting
                          → builds the `urzua` binary
ts/                       agent-harness integrations, dashboard (reserved, empty)
platform/                 deployment targets (reserved, empty — nothing is hosted)
corpora/                  real-world test corpora, anonymized (see §5)
docs/                     this repo's own records — the tool validates itself in CI
Makefile                  root dispatch: make check / test / build
```

`urzua-core` stays pure with rules as data rather than code where possible — a design property the
source implementations converged on independently (shared field-lookup module, pure-function
signatures) after re-diverging when they didn't.

Release artifacts build from `rust/`, so release automation must not assume a workspace at the repo
root.

**CI from day one:** cross-compilation matrix (macOS arm64/x86, Linux) with path filters so a
docs-only change doesn't trigger a full build, plus `urzua check` running against this repo's own
`docs/` — the project's governance validated by the tool it's building, in the CI that builds it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-07-29 | Initial spec. | **structural** |
> | 2026-07-29 | Layout changed from a root Cargo workspace to a polyglot layout with language directories at the root, per ADR-4. The original assumed Urzua was a Rust project; it is a product with a Rust component, and the integrations, dashboard, and infrastructure on the roadmap are not Rust-shaped. Crate names and responsibilities are unchanged. | **substantive** |
> | 2026-08-20 | Added `urzua init` to the command surface (SPEC-5). It was absent from the original surface entirely; the gap surfaced only when a bootstrap plan needed the command. Config moves to `.urzua/config.toml`. | **substantive** |
> | 2026-08-20 | Split into a sequential set. `check` → SPEC-2, configuration → SPEC-3, acceptance suite → SPEC-4. This spec keeps its number and the cross-cutting rules that ~35 records and source files cite; the child specs restate nothing except where they narrow it. | **substantive** |
> | 2026-07-29 | Added a second set of five acceptance-test bug classes to §"Acceptance test: the three-corpus suite": enum completeness against corpus prose, check-exists-vs-check-wired, git-tracked discovery scoping, pre-retrofit reverse-reference scanning, and the permanent structural-vs-content-scope ceiling. Added a paired open question on `urzua doctor`. | **substantive** |
> | 2026-09-05 | Moved Embodiment computation out of `audit` and into `check` (as a rule) plus a new `urzua fix` command (ADR-18/19), since it turned out to be a per-record consistency check, not a cross-record reconciliation. `audit` narrows to supersession reciprocity and dangling references. | **substantive** |
> | 2026-09-05 | `migrate` split into `migrate ids` (implemented, ADR-3/21) and `migrate schema` (not yet implemented) — two different operations under one verb, not one command with mode flags. | **structural** |
> | 2026-09-06 | `migrate schema --report` implemented (ADR-22); waiver-assist and apply remain unimplemented, each blocked on a real design question named in that ADR. | **substantive** |
> | 2026-09-06 | Stdout is always the JSON report, unconditionally -- `--format` removed entirely (ADR-23, superseding ADR-7). Added `explain`/`graph` as read-only relationship queries (ADR-24). | **substantive** |
> | 2026-09-06 | `urzua new` implemented (ADR-27): fills a checked-in template or, absent one, synthesizes YAML frontmatter from `required_fields`; refuses rather than guessing a shape when neither applies. | **substantive** |
> | 2026-09-06 | `urzua audit` implemented (ADR-30): supersession reciprocity and dangling cross-references, reusing the same rule functions `check` calls rather than a second implementation. Read-only, as originally specified. | **substantive** |
> | 2026-09-07 | `check`'s `embodiment.consistency` rule now detects drift (RFC-5 tier 1, ADR-32): a `Realized-by` locator changed, per git history, since the `Realized-by` line was last touched. No schema change. CI's `rust` job checkout needs `fetch-depth: 0` for this to work at all -- a shallow checkout makes the rule silently find nothing. | **substantive** |
> | 2026-09-07 | Normalized the header to the single-line, bold-labelled blockquote style SPEC-2 through SPEC-6 already use (still `HeaderShape::Blockquote` per RFC-10 -- no field value or parsing behavior changed). **Why:** found live, reviewing SPEC-1 against the later specs -- MILE-75 tracks whether within-type sub-format drift like this deserves its own rule; this entry is the docs-only fix for this one instance while that's decided. | **structural** |
> | 2026-09-07 | Added `milestone` (ADR-34/SPEC-6) and `bug` (ADR-35) record types -- zero `urzua-core` changes, same footprint as `waiver`. Fixed two real defects found using them: `check`'s `paths` argument silently examined the whole corpus regardless of what was requested (BUG-1), and record identifiers required an exact 4-digit filename prefix, bounding every type at 9999 records, with matching now done by numeric value rather than exact string (BUG-2). | **substantive** |
> | 2026-09-07 | `urzua new` emits type-prefixed filenames (`ADR-36-slug.md`) going forward (ADR-36); legacy `NNNN-slug.md` filenames never get renamed and resolve identically forever. Caught and fixed a second, independent instance of BUG-2's defect class in `filename_title_consistency`'s own filename/H1 parsing. | **substantive** |
> | 2026-09-07 | Extended the child-spec table with SPEC-6, 8-15; shrank `new`/`fix`/`audit`/`explain`/`graph`/`migrate`'s inline `## Commands` prose down to one-line pointers, matching how `init`/`check` already point to SPEC-5/SPEC-2 instead of duplicating their design. **Why:** those commands got their own specs (MILE-77) and the inline prose had become a second, independently-drifting description of the same design -- no rule said which one won if they ever disagreed. `SPEC-3` entry corrected to no longer claim `doctor` (split out to SPEC-15). | **structural** |

## Acceptance test: the three-corpus suite

**Specified in full by SPEC-4.** A validator is only as trustworthy as the defects it has been
shown to catch. v0 must catch every documented bug class in the acceptance suite:

- blank-field misdetection (corpus-a)
- provisional-status misclassification (corpus-b)
- reciprocity-checker data loss (corpus-c)
- decoy-corpus silent no-op (corpus-a)
- a regex invalidated by realistic corpus variance (corpus-a)
- filename/title mismatch (corpus-a)

These run against synthetic fixtures engineered to exhibit realistic corpus variance (irregular
formatting, wrapped fields, mixed header shapes) rather than only clean, hand-crafted cases — a
stronger validation set than a greenfield tool normally gets.

**Second set (corpus-a):**

- **Enum values must be checked against corpus *prose*, not just live field values.** A status enum
  was missing a value zero records used today but that another document's own prose already
  committed to as a future gate. `check`'s enum-completeness pass must scan referenced-but-unused
  values across the whole corpus, not just currently-instantiated fields — a value can be "real" a
  release before anything is set to it.
- **A rule that exists and a rule that's wired to something are different claims.** One source
  implementation's spec-level checker ran clean for weeks — not because the corpus was clean, but
  because it was never actually invoked from a CI job or hook, only chained into a shared local
  command. `audit`/`check` must report their own invocation context (was this run from CI, a hook,
  or ad hoc) so "0 errors" from an orphaned check can't be mistaken for "0 errors" from an
  enforced one. Candidate for `urzua doctor` — see Open questions.
- **Discovery must be scoped to what the VCS actually tracks, not the filesystem.** One
  implementation globbed disk directly; an untracked scratch file anywhere under the records
  directory failed every check run from that checkout, including ones touching zero records. `check`
  must default to `git ls-files` ∪ staged paths, never a raw directory walk, with an explicit
  documented fallback only when there's no git repo at all (see `urzua-core`'s discovery module).
- **A structural change has a blast radius outside the file it's made in.** Stripping heading
  numbers from one document broke section cross-references living in a completely different
  document that nobody thought to check. `migrate`/`audit` needs a reverse-reference scan — "what
  else in the corpus points at the specific thing about to be restructured" — run *before* any
  retrofit, not discovered after. This is the structural-layer sibling of the reciprocity-checker
  data-loss bug above: a transformation in place A silently breaking an invariant place B depends on.
- **Structural presence is not content-scope correctness, and this is a permanent ceiling, not a
  maturity gap.** A required section existed, had the right shape, passed every check — and still
  misrepresented the actual decision, because presence-checking cannot verify that a section's
  *content* covers the *scope* it claims to. `check` should not attempt to close this gap
  mechanically; RFC-1 should say explicitly that review remains a permanent required step for
  exactly this class of defect, so v0 is never sold as replacing a human reader, only the mechanical
  failures above it.

## Open questions

- Exact stable-ID encoding — ULID vs. UUIDv7 vs. a short human-typable prefix scheme (ADR-3
  deliberately deferred this).
- Where merge-time display-number assignment runs, and how it stays deterministic under concurrent
  merges. **Hardest correctness problem in v0.**
- Whether the config can genuinely express both target codebases' rules without an escape hatch.
  If it can't, that's a finding about the schema, not a config bug — record it rather than patching
  around it.
- Whether "is this check actually wired to CI/a hook" belongs inside `check`'s own output or as a
  separate `urzua doctor` command. Leaning separate — `check` validates records; whether *itself* is
  correctly invoked is a different question with a different failure mode, and conflating them risks
  the same shape of bug as collapsing blank/placeholder/pending into one state.
- Whether corpora can be anonymized without destroying the variance that makes them a useful test.

## References

- ADR-1 (Rust), ADR-2 (record format), ADR-3 (identifiers)
- RFC-1 — core+profile schema, Embodiment, revision log
- SPEC-4 — the corpus acceptance suite that specifies the bug histories in full
- SPEC-2 (`check`), SPEC-3 (configuration), SPEC-4 (acceptance suite), SPEC-5 (`init`) — the children
