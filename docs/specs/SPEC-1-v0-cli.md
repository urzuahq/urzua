---
Version: '0.3'
Date: 2026-07-29
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
Author: beauwilliams
Subject: 'The `urzua` CLI: its command surface, output contract, and the cross-cutting rules every command obeys -- the parent of the per-command and per-type specs split out from it.'
Implements: ADR-1, ADR-2, ADR-3
Derives-from: RFC-1
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

v0's completion criteria are **not stated here**. A spec is current truth (`SPEC-18`); a completion
condition is decided-but-unbuilt work (`SPEC-6`), and stored in a spec it surfaces nothing -- which is
how two of the three sat unmet and untracked while Phase 0 read as shorter than it was.

They live with the work that reaches them: `MILE-99` (the pre-push hook), `MILE-100` (a second target
codebase), `MILE-101` (`SPEC-4`'s acceptance suite). Self-hosting is the fourth and is met.

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

### `urzua export --format=agdr` / `urzua import`
AgDR compatibility (ADR-2). Lossy export warns rather than silently dropping fields.

## Output contract

**Every command implements `Report` and prints through one shared `emit()` function (ADR-46).**
Stdout is the only stream, success or failure, every invocation. No stderr for anything this
codebase's own code can structure into JSON, including genuine errors (ADR-26's amendment).

- **`Report` is behavior, not a shared struct.** Each command's report (`CheckReport`, `GraphReport`,
  `ExplainReport`, `NewReport`, `FixReport`, `MigrateSchemaReport`, `MigrateIdsReport`, `InitReport`,
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

## Configuration

**Specified in full by SPEC-3**, which moved it to `.urzua/config.yaml`. **Config-driven is the whole thesis** — the same binary must serve a
Python repo and a TypeScript monorepo with different directory layouts, different record types, and
different role requirements, without either forking it. If v0 needs code changes to work in the
second codebase, the founding claim is falsified and that's worth discovering in week two rather
than year two.

Configurable: record types and their directories, required fields per type and status, role
requirements, status enums, back-pointer patterns, and which checks are errors vs. warnings.

## Repository layout

Polyglot, organized by language at the root -- `ADR-4`, which is where the layout is specified. Not
restated here: it is a fact about the repository, not about this CLI, and two copies drift.

## Acceptance suite

Specified in full by `SPEC-4`, and built by `MILE-101`. A validator is only as trustworthy as the
defects it has been shown to catch, which is the bar that section carries.

## Open questions

None. Every question this spec carried has been answered or routed, and had been for some time:

| was asked | resolved by |
|---|---|
| Stable-ID encoding -- ULID vs UUIDv7 vs a prefix scheme | `ADR-21` -- ULID |
| Where merge-time display-number assignment runs | `MILE-89` |
| Whether one config expresses both corpora without an escape hatch | `MILE-51` -- run, and the verdict recorded |
| Whether "is this check wired to CI" belongs in `check` or elsewhere | `SPEC-15` -- `doctor`, as the section leaned |
| Whether corpora can be anonymized without destroying their variance | `SPEC-2`, `RFC-19` |

## References

- ADR-1 (Rust), ADR-2 (record format), ADR-3 (identifiers)
- RFC-1 — core+profile schema, Embodiment, revision log
- SPEC-4 — the corpus acceptance suite that specifies the bug histories in full
- SPEC-2 (`check`), SPEC-3 (configuration), SPEC-4 (acceptance suite), SPEC-5 (`init`) — the children
> | 2026-09-17 | Config moves from `.urzua/config.toml` to `.urzua/config.yaml` (`ADR-52`, shipped in the same change). The described mechanism changes, not just its rendering. | **substantive** |
> | 2026-09-18 | `Status: Draft` → `Accepted`, with `Embodiment`/`Realized-by` declared. **Why:** Every command this spec names ships: `new`, `check`, `explain`, `graph`, `audit`, `migrate`, `export` (with `--format`), `import`, `init`, `doctor`, `fix`. `main.rs` declares `//! Implements: SPEC-0001`. Its one outstanding promise -- *"which checks are errors vs. warnings"* as a v0 configuration surface -- was built by `MILE-80`. Verified against the binary before flipping rather than flipped in bulk -- `BUG-26` asked for exactly that, and it is why `SPEC-4` and `SPEC-5` are not flipped with these. | **substantive** |
> | 2026-09-19 | Success criteria now name the milestones that track reaching them. **Why:** `SPEC-18` defines a spec as a living current-truth document and `SPEC-6` defines a milestone as decided-but-unbuilt work. *"Runs green in at least two real target codebases"* is the second, stored as the first -- and a spec surfaces no unmet condition, so criteria 1 and 2 were unmet and untracked at once. The standard stays here; the work moved to `MILE-99`/`MILE-100`/`MILE-101`. | **substantive** |
> | 2026-09-19 | Narrowed to the CLI. **Why:** this spec carried four things that are not the CLI -- v0's completion criteria (backlog, now `MILE-99`/`100`/`101`), the repository layout (`ADR-4`'s fact, restated here and free to drift), a duplicate of the acceptance suite it already said `SPEC-4` specifies in full, and five open questions every one of which had been answered or routed elsewhere, two of them by ADRs. Written before the per-command and per-type specs were split out of it, and never narrowed when they were. | **substantive** |
