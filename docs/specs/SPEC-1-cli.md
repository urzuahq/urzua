---
Stable-Id: 01M6R3NK5AJB7EBVDWVSMCSYZQ
Version: '0.41'
Date: 2026-07-29
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/main.rs#Command@sha256:01c366f162d355f5a6621e7616d30356b5d6be7b9fb0cc9706bac6e76ed476bf, code:rust/crates/urzua-cli/src/main.rs#emit@sha256:62c36edb484b9e6eeb1a79c7ee63673ba94f2a92452e10de789dfd5f43607fb8, test:rust/crates/urzua-cli/tests/check_integration.rs#help_exits_0_with_plain_text_not_json@sha256:b188b2cc76395b745caae8e5012ca7a91f5d0e52f9a68aabe377b0cd9fc81c3e, test:rust/crates/urzua-cli/tests/check_integration.rs#a_bad_flag_emits_json_not_a_panic@sha256:eb8bfb4d74748038835831064ab072943e8c32648f99c1202940cb169993747a, test:rust/crates/urzua-cli/tests/check_integration.rs#check_exits_0_on_a_clean_corpus@sha256:585cb12f3f55fb534592e1802d0448163e7aa3fa06e5a91709bc32ab641fecb9, test:rust/crates/urzua-cli/tests/check_integration.rs#check_exits_1_on_a_missing_required_field@sha256:95b88e795e9a4ffb4fe1983f76bf57a3a3191dfc5c579f4ff207381a3ff0c307, test:rust/crates/urzua-cli/tests/check_integration.rs#check_exits_2_when_no_config_exists@sha256:ebf81ea1fb85124a9510dd6f7e0f737e8fd55d4532b0907a23eaa7f46152a5b5, test:rust/crates/urzua-cli/tests/export_integration.rs@sha256:05e83d2904484c2059524a4e60ad40d6141e6025dae569de536528f765954689
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
> | **SPEC-2** | `urzua rules` — the rule table `check` runs, in execution order | the Phase 0 deliverable |
> | **SPEC-2**, **SPEC-3** | `urzua index` — regenerates the committed record indexes `index.freshness` checks (`indexes` in the config) | Accepted |
> | **SPEC-3** | `.urzua/config.yaml` | needed by SPEC-2 |
> | **SPEC-4** | the corpus acceptance suite | needed by SPEC-2 criterion 3 |
> | **SPEC-5** | `urzua init` — type selection, `.urzua/` layout, the adopt path | the bootstrap surface |
> | **SPEC-6** | the `milestone` record type | Accepted |
> | **SPEC-8** | `urzua fix` — eligibility test, detect/apply | Accepted |
> | **SPEC-9** | the `bug` record type | Accepted |
> | **SPEC-10** | the `waiver` record type | Accepted |
> | **SPEC-11** | `urzua audit` — a deprecated alias of `check` (`ADR-79`) | Accepted |
> | **SPEC-12** | `urzua new` | Accepted |
> | **SPEC-13** | `urzua explain` / `urzua graph` | Accepted |
> | **SPEC-14** | `urzua migrate ids` / `urzua migrate schema` / `urzua migrate config` | Accepted |
> | **SPEC-15** | `urzua doctor` | Accepted |
> | **SPEC-25** | `urzua waive --from-report` | Draft |
> | **SPEC-26** | `urzua mcp` | Draft |
>
> Its number and identity are unchanged deliberately. Roughly thirty-five records, comments and
> source files cite `SPEC-1` for the rules above; renumbering or repurposing it would be the
> reverse-reference blast radius this spec's own acceptance suite records as a bug class. Identity
> is stable, and the split is additive — RFC-11 §6, applied to this document. spec number 7 is vacated
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
**Specified in full by SPEC-5.** Adopts an existing corpus in place without moving files, inferring
record types from what's already on disk and writing `.urzua/config.yaml`. For a repository with no
records, `--types adr,rfc,spec,prd` (with `--dir` for the layout root) selects built-in profiles that
are written out into the config, with a starter template per type; on a terminal, `init` asks which
types to govern. Absent from this spec's original command surface — the gap surfaced when the
bootstrap plan needed it.

### `urzua new <type> [title]`
**Specified in full by SPEC-12.** Creates a record from the configured template with a stable ID
assigned, never asking the author to pick a number. The numbers it passed over because a branch, an open
pull request or a sibling worktree already holds them are in the report's `skipped` list, each with its
reason, and not on stderr (`ADR-93`, Amendment of 2026-11-07).

### `urzua mcp`
**Specified in full by SPEC-26.** A read-only MCP server on stdio (`ADR-99`): newline-delimited JSON-RPC on stdin and
stdout, stdout carrying protocol frames only, each tool returning a report this CLI already builds. Tools built:
`urzua_check`, `urzua_list_records`, `urzua_show_record`, `urzua_get_findings_for_path` and `urzua_get_graph`. It has no tool that writes.

### `urzua waive --from-report`
**Specified in full by SPEC-25.** Drafts one pending waiver per rule and file for the current findings
(`ADR-91`): a `Reason` holding the type's pending value and no `Expires`, so a draft suppresses nothing until
a person fills it in. Today it previews only (JSON on stdout, exit 0 when it ran, 2 when it could not);
writing the drafts is the next change.

### `urzua check [paths...]`
**Specified in full by SPEC-2.** The validator. Exit non-zero on error; stdout is always the JSON
report, unconditionally (ADR-23) — no flag needed for CI or an agent to get structured output.
`urzua check --stdin-path <path>` judges one record's proposed content from stdin against every enabled
rule without writing (`SPEC-2`), the call an editor hook relays (`ADR-78`).
`urzua check --base REF --actors FILE` hands the host's identities for the change (the pull request's
author and current approvers, a JSON file the gate action writes) to `diff.decider-required` (`SPEC-2`).
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
**Specified in full by SPEC-13.** Read-only relationship queries over data `check` already parses. `urzua explain --rule <rule-id>` is the rule mode: what one rule is, from the rule table `urzua rules` prints, with the level this repository sets. A bare argument is always a path.

### `urzua audit`
**Specified in full by SPEC-11.** Deprecated (`ADR-79`): an alias of `urzua check` over the configured
rules, with no rule set of its own, that adds one deprecation `Notice`. Never writes. Removed the
release after. `urzua check --rules <id>,...` (`SPEC-2`) selects declared rules, and is the replacement
for the narrower report `audit` used to give.

### `urzua fix`
**Specified in full by SPEC-8.** Detects fields whose stated value disagrees with what the tool
computes from evidence already in the record; apply mode writes the computed value back, gated hard.
Without `--apply` nothing is written: `--ids` narrows the report (an entry matching nothing is a failure,
as under `--apply`), and `--force`, `--by` and `--actor` are usage errors (exit 2), never ignored.

### `urzua roadmap`
**Specified in full by SPEC-2** (the roadmap output). The order of work computed from the records:
waves, the ready set, the critical path, what each record blocks and a grouping, over the sequencing
pairs `roadmap.pairs` declares (SPEC-3). Read-only, nothing stored, always JSON. A repository that
declares no pair gets a `no-sequencing-pair` note, not an empty roadmap. Exit 0 whenever it could be
computed; the roadmap reports cycles and records to re-plan in its own output and does not fail on
them. `ADR-82`.

### `urzua refs refresh`
Rewrites the committed snapshot of the repository's issue and pull request numbers that
`citation.issue-exists` (`SPEC-2`) checks citations against, from the GitHub REST API
(`MILE-143`). The declaration is configuration (`issue_refs`, `SPEC-3`): the command takes no
arguments and reads the snapshot path and repository from it. It is the one command that uses the
network, so `check` stays offline and deterministic and a stale snapshot is a file to refresh and
commit, never a flaky run.

- Requests go through `curl`, which honours the environment's proxy and certificate settings. A
  token is read from `GITHUB_TOKEN`, else `GH_TOKEN`, and sent in an `Authorization` header over
  `curl`'s stdin, never on a command line; with none the request is unauthenticated (a public
  repository answers, with the lower rate limit) and an info notice says so. `URZUA_GITHUB_API`
  replaces the default API origin `https://api.github.com` (GitHub Enterprise, and tests).
- Pages of 100 are requested by `page=N` until the listing's `Link` header has no `rel="next"`; the
  `Link` URL itself is never followed, so the token only goes to the configured origin. An entry
  with a `pull_request` key is a pull request, any other an issue.
- Output is the snapshot, `{schema_version, repository, generated_at, issues, pulls}` (`generated_at`
  is the current UTC date), both lists
  ascending and de-duplicated, pretty-printed with a final newline, so the same answers give the
  same bytes and a refresh that learns nothing new on the same UTC day leaves the file untouched
  (`outcome: current`; otherwise `created` or `written`). The file is replaced only after every
  page was fetched.
- Stdout is one JSON report (`status`, `snapshot`, `repository`, `generated_at`, `outcome`,
  `issues`, `pulls`, `newest`) and exit `0`. No network, no `curl`, an exhausted rate limit (the
  error names the reset time), a rejected token, an unknown repository, a malformed answer, or no
  declared `issue_refs` is exit `2` with `status: not-run` and the reason in `error`, and the
  existing snapshot is left as it was.

### `urzua migrate ids` / `urzua migrate schema` / `urzua migrate config`
**Specified in full by SPEC-14.** Backfills stable identifiers onto existing records, previews
what a newly-required field would break before it's added to config, and rewrites a config from the
previous schema version to the current one.

### `urzua export --format json`
Every record of every configured type as plain JSON data, so tooling reads records through the one
parser `check` uses instead of a second one. Takes no paths: it always exports the whole tracked
corpus. `--format` is required, for `export` and `import` alike: there is no default format, and a
bare `urzua export` exits 2 asking for one. `--format agdr` is not implemented (exit 2). The exact fields are under "Export contract" below.

### `urzua export --format=agdr` / `urzua import`
AgDR compatibility (ADR-2). Lossy export warns rather than silently dropping fields. Not
implemented; `MILE-21`.

## Output contract

**Every command implements `Report` and prints through one shared `emit()` function (ADR-46).**
Stdout is the only stream, success or failure, every invocation. No stderr for anything this
codebase's own code can structure into JSON, including genuine errors (ADR-26's amendment).

- **`Report` is behavior, not a shared struct.** Each command's report (`CheckReport`, `GraphReport`,
  `ExplainReport`, `ExportReport`, `RoadmapReport`, `NewReport`, `FixReport`, `MigrateSchemaReport`, `MigrateIdsReport`, `InitReport`,
  `DoctorReport`, `MigrateConfigReport`, `CouldNotRun`)
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
- **A could-not-run caused by the config's `schema_version` carries the cause as data.** When the
  version is outside the window the release reads, `CouldNotRun` adds a `schema_version_error` object
  (`found`, `supported`, `current`, `migration`; `SPEC-3`, `ADR-77`) beside the usual `error` sentence,
  and is omitted otherwise.
- **The evaluation date is an input (`ADR-97`, second amendment of 2026-11-08).** A global option
  `--as-of YYYY-MM-DD` fixes the UTC date every date comparison reads: waiver `Expires`, claim age
  (including ages taken from git history, measured to the end of that day), status expiry and the audit
  grace. Without it the date is the machine's UTC date, read once per run, and behaviour is unchanged.
  A report of a command that compares dates (`check`, `audit`, `fix`, `reverify`) carries a top-level
  `as_of` object, always present: `{"date": "YYYY-MM-DD", "source": "flag" | "clock"}`. A malformed or
  impossible date is a `CouldNotRun` (exit 2). The option is refused, never ignored, by a command that
  compares no dates, and by `fix --apply` and `reverify --apply`, whose revision rows and anchors carry
  the real date; preview with `--as-of` and no `--apply`. The wall clock is read in one place.
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
| `roadmap` | object | The computed roadmap, the same object `urzua roadmap` prints (SPEC-2). Present only when `roadmap.pairs` is declared (SPEC-3), so a repository that has not opted in sees no new key. |
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

| `sections` | array or `null` | For a type that declares `sections` (SPEC-3), the record's `##` sections in document order, split by the same parser `section.required` uses; `[]` when the record has none. `null` (the key is present) when the type declares no `sections`, so "this type has no declared document model" is never confused with "declared, but this record has no sections", and a consumer cannot mistake an unread type for an empty one. Each entry: `name` (the declared `required` name when the heading is that name or one of its aliases, otherwise the heading), `heading` (as written), `line` and `end_line` (1-based, file-relative), `content` (the lines after the heading through `end_line`, each ending `\n`). |
| `preamble` | string or `null` | For a type that declares `sections`, the text between the header and the first section (`""` when none); `null` exactly when `sections` is `null`. |

`raw_text` stays beside `sections` and remains the complete file. A record whose header is unreadable
is still split from its first line, since no header region is known.

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
> | 2026-11-04 | Bumped to `0.8`. Reworded the vacated-number sentence to say "spec number 7" in place of the retired record id, so `citation.resolution` no longer reads it as a dangling citation. The meaning is unchanged. **Why:** `MILE-156` -- the rule is now enabled in this repository. | **substantive** |
> | 2026-09-17 | Config moves from `.urzua/config.toml` to `.urzua/config.yaml` (`ADR-52`, shipped in the same change). The described mechanism changes, not just its rendering. | **substantive** |
> | 2026-09-18 | `Status: Draft` → `Accepted`, with `Embodiment`/`Realized-by` declared. **Why:** Every command this spec names ships: `new`, `check`, `explain`, `graph`, `audit`, `migrate`, `export` (with `--format`), `import`, `init`, `doctor`, `fix`. `main.rs` declares `//! Implements: SPEC-0001`. Its one outstanding promise -- *"which checks are errors vs. warnings"* as a v0 configuration surface -- was built by `MILE-80`. Verified against the binary before flipping rather than flipped in bulk -- `BUG-26` asked for exactly that, and it is why `SPEC-4` and `SPEC-5` are not flipped with these. | **substantive** |
> | 2026-11-04 | Moved two revision rows (2026-09-17, 2026-09-18) that sat above the revision log marker, outside its table, into the table. No wording changed. **Why:** the rows were orphaned from the log, so the revision-log table was not what `citation.path-exists` set aside. | **structural** |
> | 2026-11-04 | Bumped to `0.9`. Added `urzua refs refresh`, which rewrites the committed snapshot of issue and pull request numbers from the GitHub REST API (token from `GITHUB_TOKEN` or `GH_TOKEN`, `curl` transport, deterministic sorted output, exit `2` and no write on any failure); it is the only command that touches the network. **Why:** `MILE-143` -- the issue and pull request half of the cited-reference check needs a snapshot that `check` can read offline. | **substantive** |
> | 2026-11-04 | Bumped to `0.10`. A panic exits 2, the documented code for could not run, after printing its `not-run` result (Rust's default is 101); a closed stdout pipe ends the run without a panic. **Why:** `MILE-176` -- the hook's JSON contract and the process exit code disagreed, and `urzua check \| head -1` panicked. | **substantive** |
> | 2026-11-04 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/src/main.rs` 1b6a076de8bd -> e085b319c363, `rust/crates/urzua-cli/tests/check_integration.rs` 288a08b371a8 -> b668d62f2f4a, `rust/crates/urzua-cli/tests/export_integration.rs` 510343733f3f -> e1ad4b495f5b. Reason: Re-verified: spec revision log is current through v0.10; main.rs and test changes add index, refs, fix flags, --fail-on and panic exit 2, all specified. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-04 | Bumped to `0.11`. Added rows for `urzua rules` and `urzua index` to the child-spec table; both commands are specified in `SPEC-2` (the rule table, `index.freshness`) and `SPEC-3` (`indexes`). No behaviour changes. **Why:** drift review of 2026-11-04: the table named neither command, so a reader could not tell where they are specified. | **structural** |
> | 2026-11-04 | Bumped to `0.12`. The export contract gains `sections` and `preamble` (`MILE-115`). **Why:** the ADR, bug and milestone types now declare `sections`, so the condition the contract named for adding them holds. A type with no declaration gets an explicit `null`, not an absent key: absence reads as "not exported", while `null` states "no declaration" and `[]` states "declared, none present", so no consumer takes a silent default. Aliases resolve to the declared name as in `MILE-165`. | **substantive** |
> | 2026-11-05 | Bumped to `0.13`. Narrowed to the item the decision concerns; re-pin approved by the maintainer's 2026-11-04 drift decision (`ADR-74`, `MILE-185`): `main.rs` to `Command` (lines 89-237), `emit` (lines 39-56); `check_integration.rs` to `help_exits_0_with_plain_text_not_json` (lines 773-790), `a_bad_flag_emits_json_not_a_panic` (lines 792-808), `check_exits_0_on_a_clean_corpus` (lines 68-93), `check_exits_1_on_a_missing_required_field` (lines 95-117), `check_exits_2_when_no_config_exists` (lines 119-129). **Why:** an unrelated edit to a volatile file must not stale this pin again; the spans were chosen by the author of this change and not reviewed line by line by the maintainer. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/tests/export_integration.rs` e1ad4b495f5b -> ee81adceb0d7. Reason: Drift review of 2026-11-04, recommendation (a). The only change to export_integration.rs since the pin is one added fixture and test (`export_json_emits_declared_sections...`, MILE-115, 76 lines); no existing test changed, so the output-contract coverage this spec cites holds. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Bumped to `0.14`; the existing `Realized-by` evidence is unchanged. **Why:** the maintainer approved moving this spec to `Shipped`, and a spec is a living document whose version moves with each such change. | **structural** |
> | 2026-11-05 | Bumped to `0.15`. Status changed from Accepted to Shipped. **Why:** Maintainer decision, 2026-11-05: its Realized-by evidence computes Verified and an independent review judged it sound. | **substantive** |
> | 2026-11-05 | Bumped to `0.16`. Re-baselined (BUG-50, attested by this PR's review): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#check_exits_0_on_a_clean_corpus` 890414a9c2c9 -> 07cd765af78d. Reason: BUG-50 made a missing revision log a finding, so the clean-corpus fixture gained a revision log; the exit-0-on-a-clean-corpus behaviour this spec cites is unchanged **Why:** the pinned evidence changed and the PR review is the human re-verification that the claim still holds. | **structural** |
> | 2026-11-05 | Bumped to `0.17`. `check` gains `--stdin-path`, named under `urzua check` and specified in `SPEC-2`. **Why:** `MILE-191`, `ADR-78` item 2. | **substantive** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (lines 89-243) 4ec5a0737546 -> 4d5cc9cf4b35. Reason: The Command enum span changed only to add check's --stdin-path flag, which this spec now names; the rest of the pinned surface is unchanged **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Bumped to `0.15`. `urzua audit` is a deprecated alias of `check`, and `check` takes `--rules`. **Why:** `MILE-190`, `ADR-79`. | **substantive** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (lines 89-249) 4d5cc9cf4b35 -> 2e660f5d188b. Reason: The Command enum span changed to add check's --rules flag, which this spec now names; the rest of the pinned surface is unchanged **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (lines 89-248) 4d5cc9cf4b35 -> 6329cd27ceb0. Reason: audit.rs now runs check's engine over the configured rules with a deprecation notice, and Command::Audit's help describes the deprecated alias (ADR-79 as amended, MILE-190); the supersession-reciprocity behaviour ADR-30 cites is unchanged, now reported through check, and the pinned test body changed only to match **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Bumped to `0.19`. Named `urzua migrate config` beside the other two `migrate` operations and added `schema_version_error` to the could-not-run contract. **Why:** `ADR-77` / `MILE-34`. | **substantive** |
> | 2026-11-05 | Bumped to `0.20`. Adds `urzua roadmap` to the command surface and `RoadmapReport` to the report list; the export contract gains an optional `roadmap` key, present only when `roadmap.pairs` is declared. **Why:** `MILE-182`, decided in `ADR-82`: the roadmap is part of `export --format json` only for a repository that opted in. | **substantive** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (lines 89-256) 6329cd27ceb0 -> e253d0d7ef6f. Reason: The Command enum span (main.rs#Command) changed only by adding the Roadmap subcommand variant for 'urzua roadmap' (MILE-182, ADR-82), a doc comment and a unit variant; no existing variant or its attributes changed, so the command surface this spec cites is intact and the new subcommand is documented in SPEC-1 0.20 and SPEC-2. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Bumped to `0.21`. Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#help_exits_0_with_plain_text_not_json` (lines 884-899) 1e3eb7deafd1 -> b188b2cc7639, `rust/crates/urzua-cli/tests/check_integration.rs#a_bad_flag_emits_json_not_a_panic` (lines 901-915) 21de607499b2 -> eb8bfb4d7474, `rust/crates/urzua-cli/tests/check_integration.rs#check_exits_0_on_a_clean_corpus` (lines 67-90) 07cd765af78d -> 585cb12f3f55, `rust/crates/urzua-cli/tests/check_integration.rs#check_exits_1_on_a_missing_required_field` (lines 92-112) cca33395d984 -> 95b88e795e9a, `rust/crates/urzua-cli/tests/check_integration.rs#check_exits_2_when_no_config_exists` (lines 114-122) fffd3008d4cf -> ebf81ea1fb85, `rust/crates/urzua-cli/tests/export_integration.rs` ee81adceb0d7 -> ffeb24dfae4a. Reason: The cited tests only changed how their fixture directory is owned: each now holds a common::TempDir (or common::TempFile) guard that removes it on drop instead of calling remove_dir_all (BUG-193); the assertions and the command surface and export contract they pin are unchanged. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Bumped to `0.22`. Corrects the two `#311` rows that re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (the row for lines 89-249, 4d5cc9cf4b35 -> 2e660f5d188b, and the row for lines 89-248, 4d5cc9cf4b35 -> 6329cd27ceb0; the second is the live pin). Accurate reason for both: `check` gained `--rules <ID,...>` and `Audit`'s doc comment now describes the deprecated alias; no other variant changed. The second row's reason was copied from `ADR-30`'s row and cited a test (the supersession-reciprocity test) that this spec does not pin. **Why:** an independent check of the re-pin reasons found the earlier row incomplete or inaccurate; corrected here without rewriting it. | **structural** |
> | 2026-11-06 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (lines 92-273) e253d0d7ef6f -> d0015ed73762. Reason: The pinned Command enum in main.rs gained a Harness subcommand variant (urzua harness) with the HarnessTarget and HarnessRun enums defined beside it; the other variants, the pinned emit function and the pinned tests are unchanged. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-06 | Bumped to `0.24`. Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (lines 92-278) d0015ed73762 -> 6f77c3055678. Reason: The pinned Command enum span in main.rs, which already carries the Harness variant from the harness install change, gained only the optional --actor argument (agent or human) with its doc comment on the Fix variant (MILE-201, ADR-89); the ActorArg enum sits outside the pinned span. No existing variant or option changed, so the command surface this spec cites is intact. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-06 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (lines 92-285) 6f77c3055678 -> 6be4a09f031d. Reason: The Command enum span changed only to add check's --actors FILE flag (the identities diff.decider-required reads), which SPEC-1 now names under urzua check; every other subcommand in the pinned span is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-06 | Bumped to `0.25`. Names `urzua check --base REF --actors FILE` under `urzua check`, the host identities `diff.decider-required` reads (`SPEC-2`). **Why:** `MILE-200`, `ADR-89` section 4. | **substantive** |
> | 2026-11-06 | Bumped to `0.26`. `urzua export` and `urzua import` require `--format`; the `agdr` default is gone, so a bare `urzua export` says a format is required instead of reporting an unimplemented one. `urzua fix --help` says what is implemented (tier 1 and `--rebaseline`). **Why:** the second-pass review (`cli.md` #11): the defaults named a format that is not implemented and the help text no longer matched `fix`. | **substantive** |
> | 2026-11-06 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (lines 92-288) 6be4a09f031d -> c61774c9d4cb, `rust/crates/urzua-cli/tests/export_integration.rs` ffeb24dfae4a -> 188eded0b56a. Reason: main.rs#Command: export and import now require --format (the agdr default is removed) and the fix help text names what fix does; no change to any other command, flag or output; export_integration.rs: the bare `export` case now asserts the `--format` required error instead of the not-implemented one, and the `agdr` case is kept **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-06 | Bumped to `0.27`. `urzua refs refresh` dates the snapshot with the current UTC date. **Why:** maintainer decision of 2026-11-05: every date comparison uses UTC; the date was the machine's local date. | **substantive** |
> | 2026-11-07 | Bumped to `0.28`. `urzua new` names the numbers it skipped, with the reason, in the report and not on stderr. **Why:** Maintainer decision, 2026-11-07, recorded as an amendment to `ADR-93`; the detail is in `SPEC-12`. | **substantive** |
> | 2026-11-07 | Bumped to `0.29`. `urzua fix` without `--apply`: `--ids` narrows the dry-run report and an `--ids` entry that matches nothing is a failure, as under `--apply`; `--force`, `--by` and `--actor` are usage errors (exit 2). **Why:** maintainer decision of 2026-11-07, from the CLI review: a typo'd `--ids` read as "nothing to do". | **substantive** |
> | 2026-11-07 | Bumped to `0.30`. One earlier revision-log row now writes the pipe inside its code span as `\|`, so the table reads the same under the escape-aware row parser; no behaviour of the CLI changes. **Why:** `MILE-213` -- a log's columns are read by header name, and an unescaped pipe in a cell widens the row. | **structural** |
> | 2026-11-07 | Bumped to `0.31`. `urzua explain` takes `--rule <rule-id>` in place of a path, to print one rule; the command's pointer to `SPEC-13` says so. **Why:** maintainer decision of 2026-11-07 (the `ADR-96` amendment of that date). | **substantive** |
> | 2026-11-07 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (lines 94-302) c61774c9d4cb -> 166509473a45, `rust/crates/urzua-cli/tests/export_integration.rs` 188eded0b56a -> 05e83d290448. Reason: main.rs#Command: the Explain variant gained an optional --rule <RULE_ID> flag, its path argument became optional (required unless --rule is given) and its doc comment and usage line were extended; no other variant changed. export_integration.rs: its only change since the pin is the earlier one-line switch of run_urzua to common::urzua() (stubbed gh), unrelated to this PR; both cited behaviours (the Command surface, export) still hold. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (lines 94-338) 166509473a45 -> a6f09ebb5aa7. Reason: Read the diff of the pinned main.rs Command enum: it gained a Reverify variant (the new urzua reverify subcommand with its --ids, --apply, --pin-only, --by, --actor, --force and --reason flags) and an arm in main's dispatch; every existing variant is unchanged, so the command surface SPEC-1 describes still holds with one added command. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.32`. The CLI gains `urzua reverify` (`ADR-97`, `SPEC-8`), the attributed renewal of an expired claim; the pin on the `Command` enum is re-baselined above. No other command changes. **Why:** `MILE-129`, the build of `ADR-97`. | **substantive** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (lines 94-338) a6f09ebb5aa7 -> 3476a956db42. Reason: Read the diff of each pinned file since the old pin and the claim still holds. main.rs#Command: only the doc comment on the Init variant changed, so its --help text now says --dry-run prints the config the real run would write instead of calling it byte-identical to the real run; no variant, flag or argument changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.33`. The pins this spec's `Realized-by` holds on files changed in this change were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. **Why:** a revision-log entry that adds a structural change needs a `Version` bump (`revision-log.version-bump-required`). | **structural** |
> | 2026-11-08 | Bumped to `0.34`. The CLI gains the global `--as-of YYYY-MM-DD` evaluation date and a top-level `as_of` on the reports of `check`, `audit`, `fix` and `reverify`, with its refusals; no pinned symbol of this spec changed. **Why:** maintainer decision of 2026-11-08 (the second `ADR-97` amendment of that date): a verdict must be reproducible and upcoming expiries previewable. | **substantive** |
> | 2026-11-09 | Bumped to `0.35`. Note on the log: The version numbers of earlier `Bumped to` rows are not strictly increasing, and history is not rewritten: the order of the rows is the order the changes landed, and the current `Version` header is authoritative. Rows affected: `0.15` appears twice (the `Accepted` to `Shipped` row and the later `urzua audit` row), and the `0.15` `audit` row comes after `0.17`. **Why:** a review found the out-of-order and repeated numbers and a reader could be misled about which row introduced which version. | **structural** |
> | 2026-11-09 | Bumped to `0.36`. Re-pin rows in this revision log were reworded to name who re-verified, matching the tool's wording since 2026-11-09; hashes, reasons and dates are unchanged. **Why:** Maintainer decision, 2026-11-09. | **structural** |
> | 2026-11-13 | Bumped to `0.37`. The CLI gains `urzua waive --from-report` (`ADR-91`, `SPEC-25`), previewing the pending waivers it would draft; the `Command` enum pin is re-baselined in the Re-baselined row. **Why:** `MILE-205`. | **substantive** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (lines 114-378) 3476a956db42 -> 16e36c6e826f. Reason: Read the diff of each re-pinned file against main. the Command enum gained one Waive variant (urzua waive --from-report, with --rule, --path and --by); every existing variant is unchanged, so the command surface SPEC-1 describes still holds with one added command. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (lines 114-381) 16e36c6e826f -> 5d496742ffbe. Reason: Re-pinned after PR4 of MILE-205 (urzua waive --apply), the stated delta being: main.rs: `waive` gains the `--apply` flag and a WaiveArgs struct, and `waive` joins the `--as-of` refusal match; no other command or output changed. The claim holds as the changed code does not alter what this record states. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (lines 114-365) 5d496742ffbe -> fcb93f9409e5. Reason: Re-pinned after PR4 of MILE-205 (urzua waive --apply), the stated delta being: main.rs: `Command::Waive` now holds a clap WaiveArgs struct (defined in commands/waive.rs) that gains `--apply`, and `waive` joins the `--as-of` refusal match; the waive arm of main() shrinks to one line to stay under the SPEC-22 bound; no other command or output changed. The claim holds as the changed code does not alter what this record states. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Bumped to `0.39`. The CLI gains `urzua mcp` (`ADR-99`, `SPEC-26`), a read-only MCP server over stdio; the `Command` enum pin is re-baselined in the Re-baselined row. **Why:** `MILE-29`. | **substantive** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (lines 114-377) fcb93f9409e5 -> 9abff7e3a62a. Reason: Read the diff of main.rs#Command against main. The Command enum gained one Mcp variant (urzua mcp --root); every existing variant is unchanged, so the command surface SPEC-1 describes still holds with one added command. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Bumped to `0.40`. `## Commands` describes `urzua init` as it now ships: adopt, and greenfield type selection with `--types`, `--dir`, built-in profiles and starter templates (`SPEC-5`, `MILE-114`), replacing the sentence that called them target design. **Why:** `MILE-114` built them. | **substantive** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs#Command` (lines 114-388) 9abff7e3a62a -> 01c366f162d3. Reason: Read the diff of main.rs (and the Command enum in SPEC-1's pin) against main: the Init command gained the --types and --dir options with their help text and reaches init::run through a new dispatch_init; the Mcp command main added is untouched. No other command or option changed. The claim each record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Bumped to `0.41`. `urzua mcp` serves all five tools (`SPEC-26`). **Why:** `MILE-29`. | **substantive** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs#emit` (lines 44-61) 29d071bbb823 -> 62c36edb484b. Reason: Read the diff of rust/crates/urzua-cli/src/main.rs against main. emit now serializes through a WithNextAction wrapper that flattens the report and appends the next_action and next_action_reason keys after the report's own, report_value (used by urzua mcp) is added, and the panic hook JSON gains both keys as null (MILE-72); emit still prints one JSON document through the same writeln and returns the report's exit code, so the claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
