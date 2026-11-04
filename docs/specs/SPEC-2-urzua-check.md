---
Version: '0.19'
Date: 2026-08-20
Status: Accepted
Embodiment: Drift detected
Realized-by: code:rust/crates/urzua-cli/src/commands/check.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
Author: beauwilliams
Subject: '`urzua check` -- discovery, rule execution, and the JSON output contract.'
Implements: RFC-1, RFC-3, RFC-10, RFC-11
Parent: SPEC-1
---
# SPEC-2 — `urzua check`

## Purpose

The validator, and the whole of Phase 0. Everything else in the roadmap parses and validates
records first, so this is the one piece nothing routes around. Cross-cutting rules — no-silent-
no-op, the permanent content-scope ceiling, the three-corpus acceptance bar — are stated in
`Parent`'s target, `SPEC-1`, and are not restated here except where this command narrows them.

`check` reads records, applies rules, and reports findings. It never writes. `--fix` is deferred to
RFC-8 and is out of scope here for a recorded reason: a bulk cross-reference rewrite caused a
documented data-loss incident in one source implementation, and repair is not safe until a revision
log makes it reversible.

## Success criteria

Done when all five hold:

| # | Criterion | Phase |
|---|---|---|
| 1 | `make check` runs `urzua check` in this repo's CI and fails the build on a deliberately broken record | A |
| 2 | It reports files examined, records read by any rule, rules executed, **and each rule's input population with its unit** | A |
| 3 | It exits non-zero when given paths that match zero files | A |
| 4 | Every rule family in §Rules is implemented and configurable | B |
| 5 | It runs green in one adopter's codebase with that adopter's hand-written linter **deleted** | C |
| 6 | It reproduces every documented bug in SPEC-1's acceptance suite against real corpora (SPEC-4) | D |

Criterion 6 belongs to the Phase 1 exit (`SPEC-1`, Exit bar), not to this spec's done.

Criterion 5 is the bar for the *product*. Criterion 1 is the bar for *momentum*: until the tool
validates this repository, every rule after it is written against corpora nobody has run.

A validator running alongside the linter it was meant to replace has failed, whatever its coverage
says.

## Phases

The command specified below is the finished shape. It is not the first thing to build, and the
ordering here exists to answer one question: **what is the shortest path to the tool running against
this repository's own records?**

Everything changes once that loop closes. Until it does, every rule is a claim about corpora nobody
has run; after it, each rule is exercised on real records the day it lands, and the project's own
governance is validated by the thing it is building.

**Corpus zero is `docs/` in this repository.** SPEC-4's anonymized corpora are required by
criterion 3 — reproducing the documented historical bugs — and are *not* required to close the loop.
That decoupling is deliberate: anonymization carries an unresolved question about whether the
corpora can be published at all, and the fastest deliverable must not sit behind the slowest one.

### Phase A — the loop closes

**Done when `urzua check` runs in this repository's CI and fails on a deliberately broken record.**
Nothing more. Unscoped: a path argument narrows which findings are reported (`BUG-67`), and this
repository's corpus is no longer confined to `docs/` -- the claim files a rule reads live beside it.

Scope:

- Git-tracked discovery, with `scope.source` reported.
- Header parsing for the record types in `docs/`.
- A minimal `.urzua/config.yaml` — record types and directories only. Not the full configuration surface,
  but a real file rather than a hardcoded layout, so the config thesis is exercised from day one
  rather than retrofitted.
- **Exactly two rules**, chosen because both fire on this corpus today (see below).
- The output contract in full: `status`, `filesExamined`, `rulesExecuted`, `scope`, `blocking`.
  The contract is cheap now and expensive to change once anything consumes it.
- Exit codes 0/1/2.
- A planted-violation test per rule.

Explicitly not in Phase A: the remaining rule families, severities, roles, boundaries, `doctor`,
the acceptance corpora.

**The two seed rules, and why these two.** Both are chosen so that the first honest run produces
findings rather than a green light — a validator whose first output is `ok` has told you nothing
about itself.

1. **Header format consistency within a record type.** At the time Phase A was scoped, this
   corpus's 22 records carried three header formats: RFC and ADR used one key per blockquote line;
   SPEC-1 used that shape while SPEC-2 through SPEC-4 used a pipe-delimited single line plus bold
   keys and a wrapped continuation. The third format had arrived in this repository's own corpus
   after header closure was specified — which is the failure mode RFC-10 describes, arriving in the
   corpus of the tool meant to prevent it. (Since converged onto `yaml-frontmatter` corpus-wide,
   ADR-33/ADR-42 — this historical drift no longer describes the corpus's current state, only what
   motivated choosing this as one of the two Phase A seed rules.)
2. **`Implements:` / `Derives-from:` resolves, and the target's status is reported.** Six pointers
   in `docs/specs/` currently name RFCs that are all `Draft`. That is RFC-12's
   decision-before-implementation gate with live subjects. Phase A only needs the pointer to
   resolve and the target status to be surfaced; whether a `Draft` target is an error, a warning, or
   the normal state of a young corpus is a decision for RFC-12, not for the checker.

Both are also the cheapest possible proof that the checker reads structure rather than greps text.

### Phase B — the rule set

The remaining rule families in §Rules, severity resolved from configuration (SPEC-3), and
`doctor`. Each rule lands with its planted-violation test and is exercised against `docs/` the day
it merges.

### Phase C — the second corpus

Part of the Phase 1 exit (`SPEC-1`), not Phase 0's. The falsifiable one. A second codebase runs green under the same binary with no code change, and its
hand-written linter is deleted. A failure here is a schema
finding to record rather than a code path to add.

### Phase D — the historical suite

SPEC-4's corpora and the documented bug classes. This is what turns "it works on two corpora" into
criterion 3, and it is last because it is the only phase whose blocker is not engineering.

## Discovery

**Default scope is what the VCS tracks**: `git ls-files` ∪ staged paths, intersected with the record
directories configured in SPEC-3. Never a raw directory walk.

This is not a preference. One source implementation globbed the filesystem directly, and a single
untracked scratch file under the records directory failed *every* run from that checkout, including
runs touching zero records. The tracked set is also what a reviewer sees, which makes the check's
input equal to the review's input.

Explicit paths on argv override discovery and are used as given -- for a directly-named file. A
directory argument still scopes the tracked-only sweep rather than walking the filesystem raw
(`BUG-24`): the "never a raw directory walk" guarantee above governs a directory argument the same as
the default sweep, and only a path naming one file precisely bypasses the tracked-set filter.

| `scope.source` | Meaning | `base` | Produced today |
|---|---|---|---|
| `tracked-sweep` | the tracked record set | `null` | **yes** |
| `git-diff` | changed against a base ref | the ref | **yes** (`MILE-123`) |
| `argv` | a directly-named file not in the tracked set | `null` | **yes** (`BUG-24`) |
| `none` | nothing selected | `null` | no |

The values are a serde-renamed `ScopeSource` enum (`BUG-25`), not a rendering of whichever internal
type produced them. Only variants the tool can actually emit exist on that enum; the rest of this
table is the declared vocabulary those modes will use when built, and is marked so a consumer is not
misled into branching on a value no run produces.

`base` is always present in `scope`: the ref as the caller gave it for a `git-diff` run, `null` for
every other source.

**`git-diff` scope** (`MILE-123`) is selected by `urzua check --base <ref>`. It is a reporting scope,
not a reading scope, exactly as a path argument is (`BUG-60`):

- The changed set is every path that differs between the merge base of `<ref>` and `HEAD` and the
  working tree, so a branch is not charged for what its base gained since the fork, and staged and
  unstaged edits to tracked files count. Untracked files never do (the tracked-only guarantee
  above). A rename is a deletion plus an addition, and a deleted path is simply not a record any
  more.
- The **examined set** is the changed records plus their dependents, and `files_examined`,
  `records_read_by_any_rule` and the findings are limited to it. A dependent is an unchanged record
  that (a) names a changed or deleted record's id in any header field (a pointer, narrative or
  relation field), (b) shares a changed record's id, or (c) cites changed or deleted evidence in
  its `Realized-by`, or (d) cites a normalised `Realized-by` locator that a changed or deleted
  record also cites, so a cross-record finding such as `embodiment.locator-promotion-candidate`
  lands on the same record a full check reports it on, or (e) is the `Scope` (or any record, for
  `"*"`) of a waiver that changed or was deleted, as it was at the merge base or as it is now, since
  that waiver may have stopped waiving a finding on it. A header value is split on commas first, so
  `ADR-1,ADR-3` names both. Deleted records are read from the merge base, since they are no longer in the
  tree; a rename is a deletion of the old id plus an addition. Without dependents a change
  elsewhere that breaks an unchanged record -- a deleted target, a renamed id, a changed `Status`,
  a reciprocal supersession, drifted or deleted evidence -- would be hidden, and a gate that hides
  what a full check reports is not a narrower check but a wrong one. One hop only: a dependent's own
  dependents are not pulled in.
- Reference resolution and every other corpus-wide computation still index the whole tracked
  corpus, so an unchanged target is never reported missing. A finding about the configuration, and
  one from a rule that reports outside the corpus (`RULES_REPORTING_OUTSIDE_THE_CORPUS`), survive
  the scope exactly as they survive a path argument (`BUG-86`, `BUG-121`).
- A change to the configuration file the run loaded puts **every record** in scope (the rules
  themselves changed), and `scope.whole_corpus_reason` says so. A configuration that is a symlink
  counts as changed when either the link's own path or its target's path differs. A path argument
  still narrows this afterwards. A changed file below a declared dir that no type owns keeps its
  `type.record-outside-declared-dir` finding, though it is not a loaded record.
- `scope` carries the account of the examined set: `changed_paths` (every path git reports,
  deletions included), `dependents` (the unchanged records pulled in) and `whole_corpus_reason`
  (`null` unless the configuration changed). These fields appear only on `git-diff` runs.
- Path arguments still apply and narrow the examined set further.
- A diff with no changed record and no configuration change examines nothing and reports `ok`
  (exit 0, `files_examined: 0`), not `not-run`, provided some rule is enabled and looked at
  anything (a configuration with no rule enabled is `not-run` here exactly as in a full run): the question asked had the answer "nothing to
  judge". It is told apart from a whole-corpus pass by an info `notice` and by `scope`
  (`changed_paths` is empty, or holds no record), and a blocking finding that survives the scope
  (about the configuration) still makes it `findings-present`. A ref that does not resolve, or
  one with no merge base with `HEAD` (the error says "no merge base"), is a could-not-run (exit 2),
  never an empty selection.
- `--base` together with a directly-named file git does not track is refused (exit 2): the diff
  lists tracked paths only, so the file could not be examined, and dropping it silently is the
  failure `BUG-24` fixed.

A `null` base is not "no scope" — a full sweep legitimately has one. Any consumer keying "did this
run" on a missing base is reading the wrong field, and the four sources exist so it does not have to.

**Fallback with no git repository**: fall back to a directory walk, and say so in `scope.source`.
Never silently — the fallback has the failure mode above, so a run using it must be identifiable
from output alone.

## Rules

Rules are data where possible, not code (`urzua-core`). Each rule declares an id, the record types
it applies to, the **statuses it applies to**, and its default severity.

Status applicability is part of the declaration rather than an `if` inside each rule. A record can
legitimately hold a number without stating a decision — a tombstone (RFC-13) — and a check that
presupposes a decision is not wrong, it is *scoped*. A corpus that cannot express that forces its
records to invent content or its linters to be switched off.

### Shipped, by actual rule id

This is the complete, current set — generated from `urzua_core::rules::RULE_METADATA` by
`scripts/generate-rule-table.py` (`BUG-52`), never hand-edited, so it cannot drift from
`ALL_RULES` the way its hand-maintained predecessor did. Regenerate with `make rule-table`; `make
ci` fails if the committed table is stale. Not a delta on top of the aspirational language below,
which stays only for what genuinely isn't built yet.

<!-- rule-table:start -->
| Rule id | What it checks |
|---|---|
| `header.required-fields` | Each type's `required_fields` (SPEC-3) are present; a missing header region or a duplicate key is also reported here. |
| `header.layout-consistency` | A record's header line layout (one-per-line vs. pipe-delimited) matches its type's declared `header_layout`, when one is declared (ADR-38). |
| `header.field-set-consistency` | Every header field belongs to its type's `required_fields` ∪ `known_fields`, when `known_fields` is declared (ADR-39). The optional `allow_extension_prefix` option (`MILE-141`), off unless declared `true`, exempts header keys beginning `x-` in any case; an unprefixed undeclared key is still a finding. |
| `type.no-declared-spec` | Which configured types currently have no `spec` pointer declared at all (ADR-41/ADR-43) — a signal to review, never a mandate; a type can permanently have none declared. |
| `type.dir-matches-nothing` | A type's declared `dir` does not exist at all (`BUG-56`'s hazard for `dir`, no load-time guard). An existing, empty directory is a legitimate declared-but-unused state and is not flagged. |
| `type.record-outside-declared-dir` | A record-shaped tracked path that no type's `dir` claimed (`BUG-62`) — a config written before `RFC-35` silently loses coverage on upgrade otherwise. |
| `identity.collision` | Two records resolving to the same identifier — a corpus error the index cannot represent, reported rather than papered over (`BUG-79`). |
| `header.deprecated-shape` | Which configured types still declare a deprecated `header_shape` (`blockquote`/`bold-list`) instead of `yaml-frontmatter` (ADR-33). Parsing support for the deprecated shapes stays; this rule only surfaces which types haven't migrated. |
| `config.pointer-declaration-missing` | A type declaring either `pointer_fields` or `narrative_fields` must declare both explicitly, even as `[]` (MILE-90/ADR-44) — omitting one is not the same as declaring zero fields of that kind. |
| `config.pointer-field-not-known` | Every field named in a type's `pointer_fields`/`narrative_fields` must also appear in that type's own `required_fields`/`known_fields` (MILE-90/ADR-44) — otherwise `header.field-set-consistency` would never have heard of it. |
| `config.pointer-narrative-overlap` | A field must be exactly one kind: named in both `pointer_fields` and `narrative_fields` for the same type is a contradiction (MILE-90/ADR-44). |
| `pointer.resolution` | A type's config-declared `pointer_fields` ∪ `narrative_fields` (MILE-90/ADR-44) resolves to a real record when it names one — a dangling reference is a finding, a resolved one is silent (`pointer.target-status` and `narrative-field.stale` separately judge what a resolved target's status means). No hardcoded field list — an undeclared type is skipped, not defaulted. |
| `pointer.target-status` | A reference resolves, but its target's `Status` is one the repository declared unacceptable (`not_in`). The optional `fields` option restricts it to the named declared pointer/narrative fields, so a dependency field (`Implements`) can be judged apart from a lineage field (`Derives-from`); an excluded field is not part of the rule's population, and a `fields` entry naming no declared field is a config error. Split from `pointer.resolution` in `MILE-80`. |
| `header.pointer-field-clean` | A type's `pointer_fields` entries are clean, comma-separated references only, format-enforced (MILE-90/ADR-44) — an empty entry between commas or freeform trailing prose is flagged. Never applies to `narrative_fields`, which tolerate prose. |
| `narrative-field.stale` | A type's config-declared `narrative_fields` pointer (e.g. a milestone's `Blocked-on`) resolves to a target whose `Status` has reached a terminal state (e.g. a cited bug is now `Fixed`) — a signal to re-examine (ADR-42, generalized beyond `Blocked-on` by MILE-90/ADR-44). `terminal_statuses` is declared per repository. |
| `field.quality` | Field presence with real semantics: `blank`, `placeholder`, and `pending` are three distinct states, not one — the single most-repeated bug class in hand-written record linters. An absent field is never silently readable as any of the three. |
| `field.pending` | A required field is marked `Pending` — work declared unfinished, as distinct from forgotten. Split from `field.quality` in `BUG-38`. |
| `claim.status-agreement` | A file outside the corpus claims to close a record whose own `Status` disagrees. Scans `claim_paths`; `closed_statuses` declared. |
| `filename.title-consistency` | The filename's display number and the H1's display number agree (ADR-3). |
| `revision-log.change-class-required` | Every revision-log entry carries a `change_class` (ADR-14); missing or unclassified is blocking. |
| `embodiment.consistency` | A stated `Embodiment` agrees with the tier its `Realized-by` locators compute to (`test:` outranks `code:` outranks `spec:`); a locator that changed per git history since `Realized-by` was last touched overrides the expected value to `Drift detected` unconditionally (RFC-5 tier 1, ADR-18/ADR-32). Requires full git history — a shallow checkout makes this rule silently unable to detect anything. |
| `embodiment.locator-exists` | A `Realized-by` locator names a path that is not both git-tracked **and** present on disk, or is empty. Tracked alone passes a staged deletion; on disk alone passes a gitignored file. |
| `embodiment.locator-promotion-candidate` | The same locator cited by more than one record's `Realized-by` is a promotion candidate, surfaced as a finding, never auto-promoted (ADR-18). |
| `relation.supersession-reciprocity` | If A supersedes B, B points back; status-aware (a claim only binds once the claiming record is itself terminal-accepted). Shared with `audit` (ADR-30) — `check` runs it too, not only `audit`. |
| `config.scope-matches-nothing` | Judges the run itself, not the corpus: whether the configuration's declared rules reached what they were handed, from the other rules' own executions (`MILE-106`). |
| `field.untrimmed-value` | A field value carrying leading/trailing whitespace, reachable only through `yaml-frontmatter` — disclosed rather than silently trimmed. |
| `header.field-case-mismatch` | A declared field written under a different case than its declaration (`RFC-40`/`ADR-58`) — the one place a case-only miss is distinguished from genuine non-adoption. |
| `config.header-none-has-no-required-fields` | A type declaring `header_shape: none` has nowhere for a field to be, so a non-empty `required_fields`, `known_fields`, `pointer_fields`, `narrative_fields`, or `relation_fields` is a self-contradiction (`ADR-50`). |
| `config.relation-field-not-known` | Every field named in a type's `relation_fields` (`RFC-42`/`ADR-61`) must also appear in that type's own `required_fields`/`known_fields`, the same shape as `config.pointer-field-not-known`. |
| `config.known-fields-declaration-missing` | A type declaring no `known_fields` at all gets no field-set governance from `header.field-set-consistency` (`ADR-53`'s "declared, not voted" default) — opt-in, so a repository can require the choice be made explicit rather than left ambiguous by omission (`RFC-43`/`ADR-62`). |
| `relation.target-status-undeclared` | A resolved pointer or narrative reference whose target type never declares `Status` at all — `ADR-60`'s gate would otherwise silently exclude it from every status-reading rule, with no disclosure of why (`RFC-45`/`ADR-63`). |
| `field.leading-reserved-indicator` | A declared field's value starts with a reserved YAML indicator character (`@`, `*`, `&`, `!`, `%`, `|`, `>`), which needs quoting to parse at all — the same avoidable, decorative-prefix hazard `BUG-18` found for `Author`/`Deciders`, generalized so the next instance is caught rather than found by hand. |
| `header.case-variant-key` | One record's header writes two keys differing only by case (`Status`/`status`), reported as the fact of the difference, never as which spelling is right (`MILE-113`). Opt-in and declaration-free: it reads the keys a record wrote, not a declared vocabulary, and composes beside the exact-match rules without changing them (`ADR-57`). An exact repeat of one spelling stays `header.required-fields`' finding. |
| `relation.pair-reciprocity` | For each declared `relation_pairs` entry with `reciprocity: required` (`RFC-47`, `MILE-122`): a claim `A.forward = B` is echoed by `B.inverse = A`, and a claim `A.inverse = B` by `B.forward = A`, across record types where the pair allows it. Needs `relation_pairs`; enabling it with none declared is a load error. Beside, not replacing, `relation.supersession-reciprocity`, which keeps the combined `Supersedes / Superseded-by` key. |
| `relation.cross-type-disallowed` | A claim in a declared pair's field whose target is a different record type, for a pair declaring `cross_type: false` (`RFC-47`, `MILE-122`). Such a claim is excluded from `relation.pair-reciprocity` and reported here, never skipped silently. |
| `citation.resolution` | A decision ID cited anywhere in a record's prose (`ADR-12`, with a configured type prefix, standing alone) resolves to an existing record, compared numerically so zero-padding does not matter (`MILE-136`). The header, which `pointer.resolution` governs, and fenced code blocks are not read; an inline code span is. Opt-in and option-free; its population counts the citations examined, so a run over none is distinguishable from a clean one. |
| `boundary.published-path-reference` | A record identifier (any identifier carrying a configured type prefix, resolvable or not) is cited from inside a `published_paths` entry (a path prefix, normalised at config load; globs are not supported and a glob is treated literally), which publishes a pointer no external reader can resolve (`RFC-11`/`ADR-9`, `MILE-125`). An identifier split across lines is not detected. A declared path matching no tracked file is itself a finding, never a clean pass; the rule's level is the boundary's severity. |
| `section.required` | Each `##` section a type's declared `sections.required` names is present in every record of that type (`MILE-4`, `RFC-17`). A type that declares none is out of scope; a record whose status is in the declaration's `exempt_statuses` (a record that made no decision, `MILE-7`) is counted out of scope, not asked for the sections. Presence only: it never judges what a section says. |
| `section.shape` | A required section that declares a `shape` has that shape (`MILE-5`, `MILE-121`): `y-statement` needs its TL;DR block to carry the five phrases `In the context of`, `facing`, `we decided`, `to achieve`, `accepting`, word-boundary matched, and reports a marker with no block as its own error; `table-min-rows` needs a table with at least `min_rows` data rows. A missing section is `section.required`'s to report, not this rule's. Shape only, never meaning. |
| `diff.source-requires-record` | A `--base` diff changes a path under a declared `source_paths` prefix but changes no governed record (a `.md` file directly in a record type's `dir`) (`MILE-124`). One finding per changed source path, so a waiver can scope to one. It asks only that a record changed, never that it is the right one. Judges the changed set, so it reports `not-applicable` on a run with no `--base`, never clean. Deleted paths count as changed. |
| `diff.source-and-records-mixed` | A `--base` diff changes both a governed record and a path under a declared `application_paths` prefix (less any `exclude_paths`), a prompt to split the change (`MILE-142`). One finding naming the first path of each kind, in path order. A governed record is never application source, whatever the prefixes say. Reports `not-applicable` on a run with no `--base`, never clean. |
| `filename.identity-pattern` | A file directly in a type's `dir` whose name does not match the type's declared `identity.pattern` (or, for a type declaring none, the `TYPE-NNNN-slug.md` convention) is reported rather than left out of every rule's population (`MILE-120`). When every file in the directory is off-pattern, such as a bare-number corpus (`0001-slug.md`), it is one finding per type naming the expected file name for each file, since renaming is the supported route (`RFC-46` was rejected). The census counts each file examined. |
| `index.freshness` | A committed index of one record type (a file, or a marked list inside a README, in a declared `format` with `{id}`, `{file}` and `{title}` placeholders) lists exactly the records: a record with no entry, an entry naming no record, a duplicated entry, an entry whose file or title differs from the record's, and a line the format does not match are each an error (`MILE-139`). A missing artifact, or one whose declared `begin`/`end` markers are absent, is an error too. Needs `indexes` (`path`, `type`, `format`, optional `begin`/`end`); the engine reads the artifacts and compares, it never regenerates one. The population counts the artifacts compared, so a run over none is distinguishable from a clean one. |
| `embodiment.back-pointer-resolution` | A source comment line `Implements: ADR-12` (a comment leader such as `//`, `#`, `--` or `*`, then the marker, then comma- or space-separated record ids) in a file under a `back_pointer_paths` entry counts as code-tier realization evidence for the record it cites, in `embodiment.consistency` and `urzua fix` (`MILE-128`, narrowing `RFC-5` section 2), and a back-pointer citing a record that does not exist is reported here. Only whole-line comments are read; the marker is case-sensitive; ids are compared numerically. Opt-in and needs `back_pointer_paths` (path prefixes, normalised at config load, no globs); evidence is collected only while this rule is enabled. A declared path covering no tracked file is itself a finding, and a non-text file is counted as unreadable, never as clean. |
<!-- rule-table:end -->

### Not yet built

Design language from this spec's original Phase B scope, with no shipped rule behind it yet:

- **Status enum validation, tolerant of trailing free-text annotation**, and validated against
  corpus *prose*, not only instantiated values: a value that no record currently holds may already
  be committed to by another document, and is real a release before anything is set to it. Nothing
  validates `Status` against an enum for any record type today (found live via MILE-71's red-team of
  the milestone `Status` field).
- **Role rules** — Author / Reviewers / Deciders per type and status, with self-acknowledgement
  detection: an author is not their own reviewer of record.
- **Required sections** per profile, generalized beyond `change_class` on revision entries (the one
  instance that's actually built).
- **Declared scope** (RFC-11 §6) and **boundaries** (RFC-11 §1) — neither is built; RFC-11's own
  open questions (nested-scope inheritance, glob-vs-subject width) are still unresolved.

Severity per rule comes from config (SPEC-3) in principle; in practice severity is still hardcoded
per rule at the source (`Error`/`Warning` only, no `Info`), not yet read from any config field —
tracked as MILE-80, which also names the still-open question of a richer severity taxonomy versus a
separate log-verbosity axis. `status` reports what was found; whether that fails the build is the
caller's choice and lives in `blocking`.

## Output contract

Implements RFC-3, as amended by ADR-7 (real field names, not RFC-3's illustrative casing) and
ADR-26/ADR-46 (stdout is the only stream; there is no stderr rendering at all). Stdout carries nothing
but the JSON object below, so a consumer never has to find it inside prose.

```json
{
  "status": "ok",
  "files_examined": 51,
  "records_read_by_any_rule": 51,
  "rules_executed": [
    {
      "rule": "header.required-fields",
      "population": { "unit": "field", "eligible": 1248, "examined": 1248, "out_of_scope": 0, "unreadable": 0 },
      "status": "ran"
    },
    {
      "rule": "claim.status-agreement",
      "status": "not-enabled"
    }
  ],
  "scope": { "source": "tracked-sweep", "base": null, "record_types": ["adr", "bug", "rfc"] },
  "blocking": false,
  "findings": [
    {
      "rule": "pointer.target-status",
      "severity": "warning",
      "file": "docs/adr/ADR-48-x.md",
      "line": null,
      "message": "Derives-from: ADR-29 resolves, but its Status is Superseded"
    }
  ],
  "notices": [
    { "severity": "warning", "subject": "identity.collision", "message": "..." }
  ]
}
```

(`rules_executed`/`findings` truncated above for length; a real run lists every declared rule and every
finding.)

`rules_executed` is the field that distinguishes this from every check in the source corpora --
`files_examined` alone answers "did it read anything," not "did it *check* anything," and a rule set
narrowed by a config error yields a confident zero over a swept corpus. Each entry's `status` is
`"ran"`, `"not-enabled"` or `"not-applicable"` (`RuleStatus`); a `"ran"` entry carries `population` (`eligible`/`examined`,
plus `out_of_scope`/`unreadable` -- `ADR-55`'s disclosure of what the rule was handed versus what it
judged), a `"not-enabled"` entry carries none. `"not-applicable"` is an enabled rule the run cannot supply
what it judges: the diff rules (`diff.source-requires-record`, `diff.source-and-records-mixed`) judge the
changed set, so on a run with no `--base` they report it, carry no population and count as not having
looked, so a run whose only enabled rules are diff rules is `"not-run"`, never `"ok"`. On a `--base` run
their findings are about changed source paths rather than records, so the diff scope does not filter them
out, and a path argument does not either. Their path options (`source_paths`, `application_paths`,
`exclude_paths`) are normalised like `published_paths`; "a governed record changed" means a changed `.md`
file directly in a record type's `dir`, deleted ones included, and a governed record is never application
source. Their population is counted in `changed-path`.

`status` is the real `ReportStatus` enum: `"ok"`, `"findings-present"`, or `"not-run"` -- never `"ok"`
when zero files were selected, keyed on the file count, not on a missing scope, nor when no rule
looked at anything. The one exception is a `git-diff` run whose diff reaches no governed record
while some rule is enabled and ran: that is `"ok"` with `files_examined: 0` and an info notice. `notices` (`ADR-46`)
is non-fatal observations (`Info`/`Warning` only) that never affect `status` or the exit code, omitted
entirely when empty -- the same is true of a `Finding`'s own `waived` field, omitted unless a waiver
covers it.

## Exit codes

| Code | When |
|---|---|
| 0 | no findings at or above the configured blocking severity |
| 1 | blocking findings |
| 2 | the check could not run — bad config, unreadable path, explicit paths matching zero files |

2 is distinct on purpose. A tool that cannot run and a tool that ran and found problems are
different states, and collapsing them is how "0 errors" comes to mean "never executed".

## Testing

- **Every rule ships with a planted-violation test** (RFC-11 §5). An absence assertion carries no
  information until its failure has been observed; a rule with no such test is unverified, not
  passing.
- **Corpus tests run against SPEC-4's anonymized corpora**, not synthetic fixtures, for every
  bug in SPEC-1's acceptance suite.
- **The no-silent-no-op property is itself tested**: a run over an empty selection must produce
  `not-run`, and a run with a rule set of zero must not report `ok`.
- **A derived rule reports the evidence it found, not only the findings it emitted.**
  Files-examined and rules-executed are not sufficient: four documented derivations each returned a
  clean, plausible zero, and **three of them would have passed a
  files-and-rules report** — the records were examined and the rules did run. What was empty was the
  evidence the rule had to reason over. A history-reading rule that resolved no history for any
  record reported a fully clean corpus; the cause was a `git log` invocation that stops at a rename,
  in a corpus that had just renamed its record homes.

  So a rule whose findings depend on derived input reports that input's size:
  `transition: 91 records, 0 with readable history` is self-refuting on its face, and `0 findings`
  is not.

  The defect is `eligible: N, examined: 0` — the rule was handed candidates and reached a verdict on
  none of them. That is §5 of RFC-11 applied to a rule's input rather than its output. `eligible: 0`
  is a *different* state and not a defect: it is the cold start of enabling a rule before its scope
  exists, which `header.layout-consistency` occupies permanently on this repository. Neither is a
  `check` verdict — what gets checked is the adopter's declaration (`ADR-53`) — and the separation
  between disclosing the population and judging it is `MILE-106`'s.
- **Fixture containment.** The rule tests deliberately contain record-shaped strings, and a
  self-referential fixture that the checker then scans is a documented trap. Fixtures live outside
  the discovered record set, and that exclusion is asserted rather than assumed.

## Out of scope

- `--fix` in any form. See RFC-8; needs the revision log first.
- **Writing.** `audit` shares `relation.supersession-reciprocity` with `check` (ADR-30) and adds
  dangling-reference reporting on top, but both commands are read-only. The data-loss incident this
  spec originally cited was about a bulk cross-reference *rewrite*, not about reading reciprocity —
  `check` running the same read-only rule `audit` does was never actually the risk.
- Judging whether a section's *content* covers the scope it claims. SPEC-1 records this as a
  permanent ceiling on mechanical checking, not a maturity gap. `check` is never sold as replacing a
  human reader.

## References

- SPEC-1 — v0 CLI: success criteria, acceptance suite, the permanent ceiling.
- SPEC-3 — configuration, which supplies record types, severities and boundaries.
- SPEC-4 — the corpora this is tested against.
- RFC-3 (output), RFC-10 (header), RFC-11 (boundaries and scope), RFC-13 (status-scoped
  checks), ADR-3 (identity).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Bumped to `0.10` (`BUG-21`). §Output contract's JSON block rewritten to the real shipped shape: `snake_case` fields, the real 3-value `ReportStatus`, `rules_executed` as the real per-rule array (not a count) with its `population`/`status` shape, `records_read_by_any_rule`, `notices`, and a `Finding`'s real fields (no `suggestedAction`, which never existed). The 2026-09-11 entry below named this drift and filed it rather than fixing it; this revision is that fix. | **substantive** |
> | 2026-09-21 | Criterion 2 no longer says "in both output formats", and re-keyed the zero-population rule in §Acceptance. **Why:** two claims that had gone untrue. There is one output format -- `ADR-23` and `ADR-26` removed the second -- so the criterion was unsatisfiable as written and no amount of correct implementation could have met it. And "a rule reporting a zero population is `not-run`" was written when a rule reported one number; now that a population carries `eligible` and `examined` separately, the two zeros mean opposite things, and treating `eligible: 0` as a fault makes enabling a rule before its scope exists fail the build (`BUG-84`). The criterion also now requires the unit, because a count without one held three different denominators (`BUG-40`). | **substantive** |
> | 2026-08-20 | Split out of SPEC-1, which retains the cross-cutting rules. | **structural** |
> | 2026-09-07 | Added the Embodiment consistency, drift, and locator-promotion rules (ADR-18/ADR-32) to the rule set — previously implemented but never listed here. Noted that the rest of this section's rule names predate and don't match the actual shipped rule ids, as a named gap rather than silently compounding it. | **substantive** |
> | 2026-09-20 | Criterion 1 and Phase A now say `urzua check`, not `urzua check docs/`. **Why:** the scope argument selects which findings are reported, so naming `docs/` excluded every finding about a file outside it -- `claim.status-agreement` reports on the claim file in `.changeset/`, and the gate therefore could never fail on a false claim, which is the one thing that rule exists to catch (`BUG-86`). The criterion's intent is that the tool checks this repository and fails on a broken record; `docs/` was the whole corpus when it was written and no longer is. | **substantive** |
> | 2026-09-09 | Rewrote §Rules for MILE-90/ADR-44: `pointer.resolution`/`header.pointer-field-clean` are now config-driven (`pointer_fields`/`narrative_fields` per type, no hardcoded field list); `blocked-on.stale` renamed `narrative-field.stale`, generalized beyond `Blocked-on`; added the three new config-level rules (`config.pointer-declaration-missing`, `config.pointer-field-not-known`, `config.pointer-narrative-overlap`) and the two rows this table had never listed at all (`type.no-declared-spec`, `header.deprecated-shape`) despite both already shipping. Corrected the stale "ten rules" framing to the real, current seventeen. **Why:** this table drifting behind the actual shipped rule set is the exact recurring gap this spec's own 2026-09-07 entry already named once; MILE-90 touched every one of these rule functions directly, making this the natural point to close the gap rather than let it recur a third time. | **structural** |
> | 2026-09-07 | Rewrote §Rules as a complete, accurate list of all ten actually-shipped rule ids (adding `header.layout-consistency`/ADR-38 and `header.field-set-consistency`/ADR-39, neither previously mentioned at all), separated from the design language that's still unbuilt. Corrected §Out of scope's claim that cross-record reconciliation is `audit`-exclusive -- `check` has run `relation.supersession-reciprocity` directly since ADR-30, and the actual data-loss risk this spec was guarding against was a bulk *rewrite*, never a read-only reciprocity check. **Why:** per ADR-14's amendment adopted earlier the same day, a spec's body must stay a complete, replayable specification of its subject at every revision, not accumulate "not yet reconciled" notes as a substitute for actually updating it -- leaving this stale on the very day that policy was adopted would have been an immediate, visible contradiction. | **substantive** |
> | 2026-09-07 | Added `blocked-on.stale` as an 11th rule (ADR-42); `pointer.resolution`'s row updated to include `Parent`/`Blocked-on`, which it had already gained (ADR-40) without this table being updated. **Why:** the same "spec must stay complete" policy applies to every rule addition, not just the ones made on the day the policy was adopted -- letting this table go one rule stale again immediately would have repeated the exact drift this spec was just corrected for. | **substantive** |
> | 2026-09-08 | Reworded the header-format-consistency seed rule's rationale (§"The two seed rules") from a present-tense claim about the corpus's current state to historical framing. **Why:** ADR-33/ADR-42's corpus-wide `yaml-frontmatter` migration made the original wording -- "this corpus of 22 records carries three header formats" -- false the moment it landed; caught by adversarial review before the migration shipped, rather than left as another stale-prose instance for a future pass to find. | **substantive** |
> | 2026-09-08 | Bumped to `0.5`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-09-11 | Noted `notices` as a real, shipped addition to the output shape (ADR-46), and named -- rather than silently compounded -- the pre-existing drift between §Output contract's illustrative RFC-3-era JSON block and the real shipped shape (camelCase vs. `snake_case`, stderr rendering that no longer exists, a `status` enum that doesn't match `ReportStatus`). Filed as its own bug rather than fixed in this revision. | **substantive** |
> | 2026-09-17 | Config moves from `.urzua/config.toml` to `.urzua/config.yaml` (`ADR-52`, shipped in the same change). The described mechanism changes, not just its rendering. | **substantive** |
> | 2026-09-18 | `Status: Draft` → `Accepted`, with `Embodiment`/`Realized-by` declared. **Why:** `check` is the most-built command in the tool and has its own integration-test file. This spec reached `Version: 0.8` -- eight revisions -- while still marked as a draft of something unbuilt. Verified against the binary before flipping rather than flipped in bulk -- `BUG-26` asked for exactly that, and it is why `SPEC-4` and `SPEC-5` are not flipped with these. | **substantive** |
> | 2026-09-19 | Rule count corrected from seventeen to twenty-one, and `ALL_RULES` named as the source this list must match. **Why:** `MILE-80` and the fixes after it added `pointer.target-status`, `field.pending`, `claim.status-agreement` and `embodiment.locator-exists` without updating the spec that calls itself *"the complete, current set"* -- so `check`'s own spec understated what `check` runs by four. Nothing checks the two agree, which is filed separately. | **substantive** |
> | 2026-09-19 | The four rules shipped since `MILE-80` added to the table: `pointer.target-status`, `field.pending`, `claim.status-agreement`, `embodiment.locator-exists`. **Why:** the count was corrected to twenty-one in the same change that left the list at seventeen rows, so the section contradicted itself in adjacent lines and the rule this repository declares `error` was undocumented in the spec calling itself complete. `BUG-52` is the missing mechanism. | **substantive** |
> | 2026-09-19 | `embodiment.locator-exists`'s row states the on-disk requirement. **Why:** it read *"absent from the git-tracked set"*, which was the rule's first form and silent on a staged deletion -- `git rm` drops a path from `ls-files` while leaving it in `diff --cached`. The spec described a check the code no longer performs. | **substantive** |
> | 2026-09-23 | Rule count corrected from twenty-one to twenty-nine; the eight rows `ALL_RULES` had gained since without this table being updated added (`type.dir-matches-nothing`, `type.record-outside-declared-dir`, `identity.collision`, `config.scope-matches-nothing`, `field.untrimmed-value`, `header.field-case-mismatch`, `config.header-none-has-no-required-fields`, `config.relation-field-not-known`). **Why:** found by a `RFC-42`/`ADR-61` review, which noticed this list's own self-declared invariant ("the complete, current set... `ALL_RULES` is the source it must match") had gone untrue by seven rows before its own change added an eighth. `BUG-52` remains open as the actual mechanism gap — nothing enforces this agreement automatically, so this is a manual sync, not a fix. | **substantive** |
> | 2026-09-23 | `argv` marked produced, scoped to a directly-named file; the Discovery prose states the directory-argument exception explicitly. **Why:** `BUG-24` fixed the gap this table had marked unbuilt -- an explicit argv path naming one file now overrides the tracked-only filter, while a directory argument still respects "never a raw directory walk" (`ADR-6`), which the table's blanket "explicit paths ... override discovery" line did not previously distinguish. | **substantive** |
> | 2026-09-23 | The rule table is now generated, not hand-maintained: `urzua_core::rules::RULE_METADATA` is the single source, `scripts/generate-rule-table.py` renders it between `<!-- rule-table:start/end -->` markers, `make rule-table-check` (wired into `make ci`) fails the build on drift. Backfilled the two rows `ALL_RULES` had gained since the last manual sync (`config.known-fields-declaration-missing`, `relation.target-status-undeclared`) and corrected `config.header-none-has-no-required-fields`'s description for its round-22 broadening. **Why:** `BUG-52` named this as the real fix -- not a comparison rule re-deriving the same fact a second time, but collapsing to one source so there is nothing left to drift. `BUG-52` is `Fixed`. | **substantive** |
> | 2026-11-02 | `pointer.target-status` gains the optional `fields` option, restricting the rule to named declared pointer/narrative fields; the rule's row in the generated table is updated. **Why:** one `not_in` list judged lineage (`Derives-from`) and dependency (`Implements`) fields alike, so a Superseded lineage target warned as stale (`BUG-172`). Omitted, the rule is unchanged. | **substantive** |
> | 2026-11-03 | `header.case-variant-key` added to the generated rule table: an opt-in, declaration-free rule reporting two keys in one record's header that differ only by case. **Why:** `MILE-113` -- a type declaring no vocabulary had nothing that flagged a `Status`/`status` pair, and `ADR-57` rules out making `duplicate_keys()` case-insensitive, so the signal is a separate rule that changes no exact-match verdict. | **substantive** |
> | 2026-11-03 | Adds `boundary.published-path-reference` to the rule table, with its required `published_paths` option, a list of path prefixes (never globs; a glob is treated literally), normalised at config load (leading `./`, trailing `/` and whitespace removed; an absolute entry, a `..` component or a backslash is a config error). An identifier split across lines is not detected. **Why:** `RFC-11` (decided in `ADR-9`) declares a set of paths that publish beyond the repository, and a record identifier cited from inside one is a violation (`MILE-125`). The rule's `level` is the boundary's severity; a declared path covering no tracked file is itself a finding (a prohibition fails closed), and a file that is not text is counted unreadable, never clean. | **substantive** |
> | 2026-11-04 | Criterion 5 now reads "one adopter's codebase" and "that adopter's hand-written linter" in place of "one source codebase"; criterion 6 is marked as part of the Phase 1 exit, so "all five hold" is now the full Phase 0 set (`SPEC-1`, Exit bar); `Version` `0.11` -> `0.12`. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | Bumped to `0.14`. `relation.pair-reciprocity` and `relation.cross-type-disallowed` added to the generated rule table; both require declared `relation_pairs` (`SPEC-3`) and `relation.supersession-reciprocity` is unchanged beside them. **Why:** `MILE-122` -- reciprocity over any declared pair of inverse fields, including across record types (`RFC-47`). | **substantive** |
> | 2026-11-04 | `citation.resolution` added to the generated rule table: an opt-in rule reporting a decision ID cited in a record's prose that resolves to no record, with a population counted in citations (new `citation` population unit). **Why:** `MILE-136` -- `pointer.resolution` reads only declared header fields, so a dangling prose citation passed every rule. | **substantive** |
> | 2026-11-04 | Bumped to `0.17`. `index.freshness` added to the generated rule table: an opt-in rule comparing a committed index of one record type (a whole file, or a list between declared `begin`/`end` marker lines in a README) with the records, in a declared line `format` with `{id}`, `{file}` and `{title}` placeholders. It reports a record with no entry, an entry naming no record, a duplicated entry, a stale file name or title, a line the format does not match, a missing artifact and a missing marker pair; it never regenerates anything and does not compare order. Its required `indexes` option (`path`, `type`, `format`, optional `begin`/`end`) is validated at config load, and its population counts the artifacts compared. **Why:** `MILE-139` -- a repository that commits an index had nothing that stopped it going stale. | **substantive** |
> | 2026-11-03 | Bumped to `0.13`. `git-diff` marked produced: `urzua check --base <ref>` reports only on records changed since the merge base of the ref and `HEAD` (working tree included), `scope` gains an always-present `base` (`null` unless `git-diff`), and §Discovery states the mode's contract, including that it is refused alongside an untracked named file. **Why:** `MILE-123` -- a pull-request gate over a large corpus should not sweep every record, and the table had reserved the value while marking it unbuilt so no consumer would branch on it before a run could produce it. | **substantive** |
> | 2026-11-04 | `git-diff` scope examines the changed records **plus their dependents**, not the changed files alone: unchanged records that name a changed or deleted record's id, share a changed id, cite changed or deleted `Realized-by` evidence, or share a normalised `Realized-by` locator with a changed or deleted record. A configuration change puts every record in scope; an empty diff is `ok` with an info notice rather than `not-run`; `scope` gains `changed_paths`, `dependents` and `whole_corpus_reason` on `git-diff` runs; a ref with no merge base names that in its error. `Version` `0.13` stays (the `--base` row above is the same unreleased change). **Why:** review of `MILE-123` reproduced a hole -- deleting a target left its referrer unexamined, so `check --base` reported nothing where a full check reported `pointer.resolution` -- and an empty diff that could not be told from a clean pass of the whole corpus. | **substantive** |
> | 2026-11-04 | `git-diff` scope also treats as dependents the records covered by a changed or deleted waiver's old and new `Scope`; splits header values on commas when finding references; keeps `type.record-outside-declared-dir` findings for changed files; treats a symlinked configuration as changed by either path; an empty diff with no rule enabled is `not-run`. §Output `status` states the empty git-diff exception. **Why:** late review of `MILE-123` reproduced each as a case where `check --base` hid or mis-stated what a full check reports. | **substantive** |
> | 2026-11-04 | Bumped to `0.18`. `section.required` and `section.shape` added to the generated rule table: opt-in rules reading a type's declared `sections.required` and `exempt_statuses` (`SPEC-3`), with a population counted in `(record, required section)` slots (new `section` population unit). **Why:** `MILE-4`, `MILE-5`, `MILE-7` and `MILE-121` -- SPEC-2 named required sections per type as a rule category and nothing built it. | **substantive** |
> | 2026-11-04 | `Embodiment` recomputed by `urzua fix`: `Drift detected`. **Why:** every file named in `Realized-by` has changed since this line was last verified, and this pass did not re-verify it, so the record states the drift instead of a stale `Verified`; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/commands/check.rs, test:rust/crates/urzua-cli/tests/check_integration.rs` -- stated `Verified` disagreed with the computed value. | **structural** |
> | 2026-11-04 | Bumped to `0.16`. Adds `diff.source-requires-record` and `diff.source-and-records-mixed` to the generated rule table, a new `not-applicable` `RuleStatus` and `changed-path` population unit, and the `source_paths`, `application_paths` and `exclude_paths` rule options (`SPEC-3`'s path-option normalisation). **Why:** `MILE-124` and `MILE-142` -- two diff checks an adopter's hand-written linter makes, source change without a record change and a change mixing records with application source, need a declared home; they run only on `--base` and say so otherwise rather than passing. | **substantive** |
> | 2026-11-04 | Bumped to `0.15`. `header.field-set-consistency` gains the optional `allow_extension_prefix` option, off unless declared `true`: a header key beginning `x-` in any case is exempt from the rule, and an unprefixed undeclared key is still a finding. The option is declared per rule, not per type, so a type's `known_fields` still lists every field the rule judges by default; `pointer.resolution` and `narrative-field.stale` read only declared pointer and narrative fields, so a prefixed key is never one of them unless declared. The 'Not yet built' entry for the `x-` escape hatch is removed, and the rule-table row is regenerated from `RULE_METADATA`. **Why:** `MILE-141` -- the adopter's linter admits `x-` keys through its closed key set, so without an equivalent a repository either declares every local extension in config or loses typo detection on real keys. | **substantive** |
> | 2026-11-04 | `filename.identity-pattern` added to the generated rule table: an opt-in rule reporting a file directly in a type's `dir` whose name yields no identity under the type's declared `identity.pattern` (or the `TYPE-NNNN-slug.md` convention when none is declared). When every file of a type is off-pattern it reports once on the directory with each file's expected name; `audit` runs it too. **Why:** `MILE-120` -- such a file was a loaded record every identity-reading rule skipped in silence, and `RFC-46` was rejected, so renaming is the supported route. | **substantive** |
> | 2026-11-04 | Bumped to `0.19`. `embodiment.back-pointer-resolution` added to the generated rule table, with its required `back_pointer_paths` option (path prefixes, normalised at config load like `published_paths`): an `Implements: <id>` source comment is code-tier evidence for the cited record in `embodiment.consistency` and `urzua fix`, and a back-pointer citing no record is reported. Evidence is collected only while the rule is enabled; its findings are kept under `check <path>` scoping (`RULES_REPORTING_OUTSIDE_THE_CORPUS`). **Why:** `MILE-128` -- a first adopter relies on such comments, so the evidence model needed a place for them (`ADR-66`). | **substantive** |
