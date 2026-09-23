---
Version: '0.9'
Date: 2026-08-20
Status: Accepted
Embodiment: Verified
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
| 5 | It runs green in one source codebase with that codebase's hand-written linter **deleted** | C |
| 6 | It reproduces every documented bug in SPEC-1's acceptance suite against real corpora (SPEC-4) | D |

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

The falsifiable one. A second codebase runs green under the same binary with no code change, and its
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
| `git-diff` | changed against a base ref | the ref | no — the mode is not built |
| `argv` | a directly-named file not in the tracked set | `null` | **yes** (`BUG-24`) |
| `none` | nothing selected | `null` | no |

The values are a serde-renamed `ScopeSource` enum (`BUG-25`), not a rendering of whichever internal
type produced them. Only variants the tool can actually emit exist on that enum; the rest of this
table is the declared vocabulary those modes will use when built, and is marked so a consumer is not
misled into branching on a value no run produces.

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
| `header.field-set-consistency` | Every header field belongs to its type's `required_fields` ∪ `known_fields`, when `known_fields` is declared (ADR-39). |
| `type.no-declared-spec` | Which configured types currently have no `spec` pointer declared at all (ADR-41/ADR-43) — a signal to review, never a mandate; a type can permanently have none declared. |
| `type.dir-matches-nothing` | A type's declared `dir` does not exist at all (`BUG-56`'s hazard for `dir`, no load-time guard). An existing, empty directory is a legitimate declared-but-unused state and is not flagged. |
| `type.record-outside-declared-dir` | A record-shaped tracked path that no type's `dir` claimed (`BUG-62`) — a config written before `RFC-35` silently loses coverage on upgrade otherwise. |
| `identity.collision` | Two records resolving to the same identifier — a corpus error the index cannot represent, reported rather than papered over (`BUG-79`). |
| `header.deprecated-shape` | Which configured types still declare a deprecated `header_shape` (`blockquote`/`bold-list`) instead of `yaml-frontmatter` (ADR-33). Parsing support for the deprecated shapes stays; this rule only surfaces which types haven't migrated. |
| `config.pointer-declaration-missing` | A type declaring either `pointer_fields` or `narrative_fields` must declare both explicitly, even as `[]` (MILE-90/ADR-44) — omitting one is not the same as declaring zero fields of that kind. |
| `config.pointer-field-not-known` | Every field named in a type's `pointer_fields`/`narrative_fields` must also appear in that type's own `required_fields`/`known_fields` (MILE-90/ADR-44) — otherwise `header.field-set-consistency` would never have heard of it. |
| `config.pointer-narrative-overlap` | A field must be exactly one kind: named in both `pointer_fields` and `narrative_fields` for the same type is a contradiction (MILE-90/ADR-44). |
| `pointer.resolution` | A type's config-declared `pointer_fields` ∪ `narrative_fields` (MILE-90/ADR-44) resolves to a real record when it names one — a dangling reference is a finding, a resolved one is silent (`pointer.target-status` and `narrative-field.stale` separately judge what a resolved target's status means). No hardcoded field list — an undeclared type is skipped, not defaulted. |
| `pointer.target-status` | A reference resolves, but its target's `Status` is one the repository declared unacceptable (`not_in`). Split from `pointer.resolution` in `MILE-80`. |
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
- **Header closure's `x-` escape hatch** (RFC-10): a declared-and-justified extra key, distinct from
  `header.field-set-consistency`'s per-type `known_fields` list. Not implemented — today an unlisted
  field is either invisible (no `known_fields` declared for the type) or a finding (declared), with
  no third "explicitly excused" state.
- **Declared scope** (RFC-11 §6) and **boundaries** (RFC-11 §1) — neither is built; RFC-11's own
  open questions (nested-scope inheritance, glob-vs-subject width) are still unresolved.

Severity per rule comes from config (SPEC-3) in principle; in practice severity is still hardcoded
per rule at the source (`Error`/`Warning` only, no `Info`), not yet read from any config field —
tracked as MILE-80, which also names the still-open question of a richer severity taxonomy versus a
separate log-verbosity axis. `status` reports what was found; whether that fails the build is the
caller's choice and lives in `blocking`.

## Output contract

Implements RFC-3. Human format on stderr; the machine channel is stdout and carries nothing else,
so a consumer never has to find the JSON inside prose.

```json
{
  "status": "ok | warn | error | not-run",
  "filesExamined": 91,
  "rulesExecuted": 14,
  "scope": { "source": "tracked-sweep", "base": null },
  "blocking": false,
  "findings": [
    {
      "rule": "header.duplicate-key",
      "severity": "error",
      "file": "docs/adr/ADR-42-a-record.md",
      "line": 14,
      "message": "...",
      "suggestedAction": "..."
    }
  ]
}
```

`rulesExecuted` is the field that distinguishes this from every check in the source corpora.
`filesExamined` alone answers "did it read anything"; it does not answer "did it *check* anything",
and a rule set narrowed by a config error yields a confident zero over a swept corpus.

`status` is `not-run` — never `ok` — when zero files were selected. Keyed on the file count, not on
a missing `base`.

**The block above is illustrative (RFC-3's original camelCase framing), not the shipped shape** --
the real field names are `snake_case` (`files_examined`, `rules_executed`), `status` is the real
`ReportStatus` enum (`ok | findings-present | not-run`, not the four values shown above), and there
is no stderr rendering at all (ADR-26/46: stdout is the only stream, unconditionally). This drift
predates this revision and is filed separately (see the new bug this revision's own change references
in `docs/bugs/`) rather than fixed here. What IS new in this revision: the real, shipped shape also
carries an optional `notices: [{severity, subject, message}]` array (ADR-46) -- non-fatal
observations (`Info`/`Warning` only) that never affect `status`/exit code, omitted entirely when
empty.

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
