---
Version: '0.7'
Date: 2026-07-29
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/main.rs@sha256:1b6a076de8bdc7c3f534a819d821617b1d62fad313ccb108d3d70412634e76fd, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:288a08b371a85ebb981626b959f5acdce0319f2d2d38c0c06544d77d383a5b20, test:rust/crates/urzua-cli/tests/export_integration.rs@sha256:510343733f3f3e77da53d4d62d70ca2f422779fb19ff2f5421af862aa995ace8
Author: beauwilliams
Subject: 'The `urzua` CLI: its command surface, output contract, and the cross-cutting rules every command obeys -- the parent of the per-command and per-type specs split out from it.'
Implements: ADR-1, ADR-2, ADR-3
Derives-from: RFC-1
---
# SPEC-1 — The `urzua` CLI

## Purpose

Ship the smallest thing that is **genuinely used in real codebases** (its own repository in Phase 0, a first adopter's and then two adopters' by the Phase 1 exit) and proves the founding
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
> | **SPEC-3** | `.urzua/config.yaml` | needed by SPEC-2 |
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

## Exit bar

The exit bars below are **the standard**, which is current truth and belongs here. Reaching them is
backlog, which does not -- a spec has no `Status` and no `Blocked-on`, so a completion condition
stored here surfaces nothing, which is how two of them sat unmet and untracked.

Phase 0 has a single exit. Phase 0 is done when, and only when urzua governs its own repository in
CI with zero standing warnings, and reproduces within that repository, configured in its own
`.urzua/config.yaml`, the governance functionality a real adopter environment has, on a solid
codebase. Phase 0 is bootstrapping and capability parity in urzua's own repository; adopting it
elsewhere follows in Phase 1.

Phase 1 is done when, and only when:

0. It replaces one adopter's hand-written linter, which is then deleted. -- `MILE-102`. **This is
   the real bar.** A tool that runs *alongside* the thing it was meant to replace has not replaced
   it.
1. It runs green in **at least two** real target codebases, with different primary languages, via
   pre-push hook and CI, with zero host-repo runtime added to either. -- `MILE-99` (the hook),
   `MILE-100` (the second codebase).
2. It reproduces **every documented bug** in the acceptance suite against real corpora. Not synthetic
   tests only -- fixtures engineered to exhibit the same variance a real corpus does. -- `MILE-101`.
3. `urzua new` is what people and agents actually invoke, because hand-authoring is now the slower
   path. -- `MILE-103`.

## Commands

### `urzua init`
**Specified in full by SPEC-5** (still `Draft`: only adopt mode, with `--dry-run`, is built).
Adopts an existing corpus in place without moving files, inferring record types from what's already
on disk and writing `.urzua/config.yaml`. Type *selection* (`--types`/`--dir`, built-in profiles)
and template creation are SPEC-5's target design, not current behavior. Absent from this spec's
original command surface — the gap surfaced when the bootstrap plan needed it.

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

### `urzua export --format json`
Every record of every configured type as plain JSON data, so tooling reads records through the one
parser `check` uses instead of a second one. Takes no paths: it always exports the whole tracked
corpus. `--format json` must be named explicitly; `urzua export` with no flag still selects
`agdr`, which is not implemented (exit 2). The exact fields are under "Export contract" below.

### `urzua export --format=agdr` / `urzua import`
AgDR compatibility (ADR-2). Lossy export warns rather than silently dropping fields. Not
implemented; `MILE-21`.

## Output contract

**Every command implements `Report` and prints through one shared `emit()` function (ADR-46).**
Stdout is the only stream, success or failure, every invocation. No stderr for anything this
codebase's own code can structure into JSON, including genuine errors (ADR-26's amendment).

- **`Report` is behavior, not a shared struct.** Each command's report (`CheckReport`, `GraphReport`,
  `ExplainReport`, `ExportReport`, `NewReport`, `FixReport`, `MigrateSchemaReport`, `MigrateIdsReport`, `InitReport`,
  `DoctorReport`, `CouldNotRun`)
  stays its own fully-typed struct; `Report` only requires `notices()` and `exit_code()`. There is
  no generic envelope wrapping every payload — an earlier design tried that via `#[serde(flatten)]`
  and was killed by a confirmed duplicate-JSON-key bug before it shipped.
- **`Notice` carries a non-fatal observation an agent can pattern-match on** — `severity`
  (`Info`/`Warning`, deliberately never `Error`), `subject` (a per-emitter constant, e.g.
  `"identity"`), `message`. A `notices` field never changes a report's exit code; this is structural
  (the type has no `Error` variant to set), not a convention someone could violate by mistake.
- **Exit code is a channel independent of stdout/stderr content.** `Report::exit_code()` drives it —
  0 clean, 1 blocking findings, 2 could not run — the same contract a CI script gates on (`urzua
  check docs/ || exit 1`) whether output is JSON, plain text, or nothing at all.
- **`--help`/`--version` are the one stated plain-text exception** — clap's own output, never
  JSON-wrapped, matching `cargo`'s own convention that help/version generation is never gated behind
  a machine-format flag.
- **A `std::panic::set_hook` covers the one gap `Report`/`emit()` can't reach**: an unexpected panic
  prints a minimal ad hoc JSON object to stdout before the process exits, instead of Rust's default
  raw-text-to-stderr behavior. Not routed through `Notice`/`Report` — a panic is a lower-level escape
  hatch, not another instance of the five-command contract.

### Export contract (`urzua export --format json`)

Prints one `ExportReport` object (exit 0; exit 2 with a `CouldNotRun` object if the corpus cannot be
read, as for `graph`). Because `raw_text` publishes file contents, a tracked file in a record
directory whose real path resolves outside the repository (a symlink) makes the export exit 2 with a
`CouldNotRun` naming it, rather than reading it:

| Field | Type | Meaning |
|---|---|---|
| `status` | `"ok"` | Read-only; no blocking axis. |
| `record_count` | integer | Length of `records`. |
| `types` | array | One entry per configured type, in sorted name order: `type`, `dir`, `records` (the count exported). A type with no records is listed with `0`, so "empty" and "not configured" differ. |
| `records` | array | Every record, ordered by `type` then `path`. |
| `notices` | array | Omitted when empty, as for every report. |

Each `records` entry:

| Field | Type | Meaning |
|---|---|---|
| `type` | string | The configured type name (e.g. `milestone`). |
| `path` | string | Repository-relative path. |
| `id` | string or `null` | The display identifier from the filename under the type's prefix (e.g. `MILE-115`); `null` when the filename yields none. |
| `stable_id` | string or `null` | The `Stable-Id` header value (key matched case-insensitively, as everywhere); `null` when absent. |
| `header_readable` | boolean | `false` when no header region parsed. Then `header` is empty and means "unreadable", not "no fields". A type declaring `header_shape: none` has no header by design, so its records are `true` with an empty `header`. |
| `header` | array | The header as the configured `header_shape` parser (the one `check` uses) read it, in file order: `{ "name", "value", "line" }`. A list, not a map, so a repeated key stays visible. Values are strings exactly as parsed. |
| `raw_text` | string | The file's complete text, unmodified, header included. |

**There is no `sections` field yet.** No record type declares where its sections come from
(`MILE-98`), so the body is one raw string and a consumer that needs a section (`## What`) must
extract it from `raw_text` itself. When `MILE-98` lands, sections are added as a new field beside
`raw_text`; this spec is then revised.

## Configuration

**Specified in full by SPEC-3**, which moved it to `.urzua/config.yaml`. **Config-driven is the whole thesis** — the same binary must serve a
Python repo and a TypeScript monorepo with different directory layouts, different record types, and
different role requirements, without either forking it. If v0 needs code changes to work in the
second codebase, the founding claim is falsified and that's worth discovering in week two rather
than year two.

Configurable: record types and their directories, required fields per type and status, role
requirements, status enums, back-pointer patterns, and which checks are errors vs. warnings.

## Repository layout

Specified by `SPEC-21`. `ADR-4` decided the layout is language-first; `SPEC-21` records what it
currently is, because a frozen decision cannot track a living inventory.

## Acceptance suite

Specified in full by `SPEC-4`, and built by `MILE-101`. A validator is only as trustworthy as the
defects it has been shown to catch.

## Open questions

None. Every question this spec carried has been answered or routed:

| was asked | resolved by |
|---|---|
| Stable-ID encoding -- ULID vs UUIDv7 vs a prefix scheme | `ADR-21` -- ULID |
| Where merge-time display-number assignment runs | `MILE-89` |
| Whether one config expresses both corpora without an escape hatch | `MILE-51` -- run, verdict recorded |
| Whether "is this check wired to CI" belongs in `check` or elsewhere | `SPEC-15` -- `doctor` |
| Whether corpora can be anonymized without destroying their variance | `SPEC-2`, `RFC-19` |

## References

- ADR-1 (Rust), ADR-2 (record format), ADR-3 (identifiers)
- RFC-1 — core+profile schema, Embodiment, revision log
- SPEC-4 — the corpus acceptance suite that specifies the bug histories in full
- SPEC-2 (`check`), SPEC-3 (configuration), SPEC-4 (acceptance suite), SPEC-5 (`init`) — the children
> | 2026-09-17 | Config moves from `.urzua/config.toml` to `.urzua/config.yaml` (`ADR-52`, shipped in the same change). The described mechanism changes, not just its rendering. | **substantive** |
> | 2026-09-18 | `Status: Draft` → `Accepted`, with `Embodiment`/`Realized-by` declared. **Why:** Every command this spec names ships: `new`, `check`, `explain`, `graph`, `audit`, `migrate`, `export` (with `--format`), `import`, `init`, `doctor`, `fix`. `main.rs` declares `//! Implements: SPEC-0001`. Its one outstanding promise -- *"which checks are errors vs. warnings"* as a v0 configuration surface -- was built by `MILE-80`. Verified against the binary before flipping rather than flipped in bulk -- `BUG-26` asked for exactly that, and it is why `SPEC-4` and `SPEC-5` are not flipped with these. | **substantive** |

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-07-29 | Initial spec. | **structural** |
> | 2026-07-29 | Layout changed from a root Cargo workspace to a polyglot layout with language directories at the root, per ADR-4. The original assumed Urzua was a Rust project; it is a product with a Rust component, and the integrations, dashboard, and infrastructure on the roadmap are not Rust-shaped. Crate names and responsibilities are unchanged. | **substantive** |
> | 2026-08-20 | Added `urzua init` to the command surface (SPEC-5). It was absent from the original surface entirely; the gap surfaced only when a bootstrap plan needed the command. Config moves to `.urzua/config.yaml`. | **substantive** |
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
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-09-11 | Added the `## Output contract` section (`ADR-46`, `BUG-19`): one shared `Report`/`Notice`/`emit()` contract every command implements, replacing five independently-hand-rolled JSON shapes. **Why:** `check`, `fix`, `new`/`explain`/`graph`/`doctor` each printed a different, incompatible shape, contradicting `ADR-7`'s own "every future command must emit this same shape" rule; a real duplicate-JSON-key bug was found and killed in an earlier draft of this design (a generic envelope composed in via `#[serde(flatten)]`) before it shipped. | **substantive** |
> | 2026-09-11 | Corrected this same section's opening sentence, which overclaimed "every command" when `urzua init`/`urzua migrate ids` are named exceptions two sentences later. **Why:** caught by review before merge -- the section contradicted itself within four lines. | **structural** |
> | 2026-09-11 | "Every command" is now literally true: `init`/`migrate ids` gained real `Report` types (BUG-20), so the exception carved out above is removed rather than left stale. `§urzua init` also corrected -- it claimed type selection and template creation, neither of which is built (SPEC-5 is still `Draft` for exactly that reason). | **substantive** |
> | 2026-09-19 | Narrowed to the CLI; `Version` `0.3` → `0.4`; revision log moved to the end of the document. **Why:** carried the repository layout (now `SPEC-21`), a duplicate of the acceptance suite it already said `SPEC-4` specifies in full, and five open questions every one of which had been answered or routed. The four exit criteria stay -- they are the standard, which is current truth -- and each now names the milestone reaching it, because a spec cannot report an unmet condition. The revision log had been sitting *inside* the Monorepo layout section, which is why removing that section destroyed it on the first attempt. | **substantive** |
> | 2026-09-19 | Retitled from *`urzua` v0 CLI* to *The `urzua` CLI*, and the file renamed to match. **Why:** the subject is the CLI, not one version of it. v0's exit bar is stated here because that is current truth about the standard, but the spec outlives it -- a spec whose number is permanent per subject (`ADR-14`) should not carry a version in its name. | **structural** |
> | 2026-11-02 | `urzua export --format json` implemented (`MILE-115`); `Version` `0.4` → `0.5`. **Why:** `export` declared only `agdr` and was unimplemented, so `scripts/generate-dashboard.py` grew a second parser of the corpus that could disagree with `check` (the published dashboard once showed 3 bugs against roughly 145 in the corpus). Added the command section and an "Export contract" naming every JSON field, including that the body is a single `raw_text` string because no type declares sections (`MILE-98`), and `ExportReport` in the report list. `--format agdr` and the no-flag default are unchanged (not implemented; `MILE-21`). | **substantive** |
> | 2026-11-02 | Export refuses a record symlinked outside the repository; `header_shape: none` records are `header_readable: true`. **Why:** review found `raw_text` could disclose an outside file, and headerless records were indistinguishable from malformed ones. | **substantive** |
> | 2026-11-04 | Exit bar restated: Phase 0 has a single exit (urzua governs its own repository in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the second codebase, the full acceptance suite and the `new`-is-faster measurement move to the Phase 1 exit; `Version` `0.5` -> `0.6`. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | `Realized-by` locators pinned by content hash (`@sha256:`) after re-verifying each cited file against the Decision. **Why:** Re-verified against current code; maintainer decision 2026-11-04. | **substantive** |
> | 2026-11-04 | Exit bar restated again: Phase 0 is urzua reproducing, in its own repository and its own `.urzua/config.yaml`, the governance functionality a real adopter environment has, with zero standing warnings; replacing one adopter's hand-written linter (`MILE-102`) moves to the Phase 1 exit; `Version` `0.6` -> `0.7`. **Why:** Maintainer decision, 2026-11-04: Phase 0 is bootstrapping and parity on a solid codebase, and the adopter switchover follows it. | **substantive** |
