---
Version: '0.7'
Date: 2026-08-20
Status: Draft
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
| 1 | `make check` runs `urzua check docs/` in this repo's CI and fails the build on a deliberately broken record | A |
| 2 | It reports files examined, rules executed, **and each rule's input population**, in both output formats | A |
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

**Done when `urzua check docs/` runs in this repository's CI and fails on a deliberately broken
record.** Nothing more.

Scope:

- Git-tracked discovery, with `scope.source` reported.
- Header parsing for the record types in `docs/`.
- A minimal `.urzua/config.toml` — record types and directories only. Not the full configuration surface,
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

Explicit paths on argv override discovery and are used as given.

| `scope.source` | Meaning | `base` |
|---|---|---|
| `tracked-sweep` | the tracked record set | `null` |
| `git-diff` | changed against a base ref | the ref |
| `argv` | explicit paths | `null` |
| `none` | nothing selected | `null` |

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

Seventeen rules run today. This list is the complete, current set — not a delta on top of the
aspirational language below, which stays only for what genuinely isn't built yet.

| Rule id | What it checks |
|---|---|
| `header.required-fields` | Each type's `required_fields` (SPEC-3) are present; a missing header region or a duplicate key is also reported here. |
| `header.layout-consistency` | A record's header line layout (one-per-line vs. pipe-delimited) matches its type's declared `header_layout`, when one is declared (ADR-38). |
| `header.field-set-consistency` | Every header field belongs to its type's `required_fields` ∪ `known_fields`, when `known_fields` is declared (ADR-39). |
| `field.quality` | **Field presence with real semantics**: `blank`, `placeholder`, and `pending` are three distinct states, not one — the single most-repeated bug class in hand-written record linters. An absent field is never silently readable as any of the three. |
| `pointer.resolution` | A type's config-declared `pointer_fields` ∪ `narrative_fields` (MILE-90/ADR-44) resolves to a real record when it names one; the target's status is surfaced, never judged (RFC-12 stays the policy owner of whether a `Draft` target is acceptable). No hardcoded field list — an undeclared type is skipped, not defaulted. |
| `header.pointer-field-clean` | A type's `pointer_fields` entries are clean, comma-separated references only, format-enforced (MILE-90/ADR-44) — an empty entry between commas or freeform trailing prose is flagged. Never applies to `narrative_fields`, which tolerate prose. |
| `filename.title-consistency` | Filename ↔ H1 title ↔ display number ↔ stable ID agree (ADR-3). |
| `revision-log.change-class-required` | Every revision-log entry carries a `change_class` (ADR-14); missing or unclassified is blocking. |
| `relation.supersession-reciprocity` | If A supersedes B, B points back; status-aware (a claim only binds once the claiming record is itself terminal-accepted). Shared with `audit` (ADR-30) — `check` runs it too, not only `audit`; see Out of scope below. |
| `embodiment.consistency` | A stated `Embodiment` agrees with the tier its `Realized-by` locators compute to (`test:` outranks `code:` outranks `spec:`); a locator that changed per git history since `Realized-by` was last touched overrides the expected value to `Drift detected` unconditionally (RFC-5 tier 1, ADR-18/ADR-32). Requires full git history — a shallow checkout makes this rule silently unable to detect anything. |
| `embodiment.locator-promotion-candidate` | The same locator cited by more than one record's `Realized-by` is a promotion candidate, surfaced as a finding, never auto-promoted (ADR-18). |
| `narrative-field.stale` | A type's config-declared `narrative_fields` pointer (e.g. a milestone's `Blocked-on`) resolves to a target whose `Status` has reached a terminal state (e.g. a cited bug is now `Fixed`) — a signal to re-examine, distinct from `pointer.resolution`'s routine "resolves; target Status = X" surfacing (ADR-42, generalized beyond `Blocked-on` by MILE-90/ADR-44). |
| `type.no-declared-spec` | Which configured types currently have no `spec` pointer declared at all (ADR-41/ADR-43) — a signal to review, never a mandate; a type can permanently have none declared. |
| `header.deprecated-shape` | Which configured types still declare a deprecated `header_shape` (`blockquote`/`bold-list`) instead of `yaml-frontmatter` (ADR-33). Parsing support for the deprecated shapes stays; this rule only surfaces which types haven't migrated. |
| `config.pointer-declaration-missing` | A type declaring either `pointer_fields` or `narrative_fields` must declare both explicitly, even as `[]` (MILE-90/ADR-44) — omitting one is not the same as declaring zero fields of that kind. |
| `config.pointer-field-not-known` | Every field named in a type's `pointer_fields`/`narrative_fields` must also appear in that type's own `required_fields`/`known_fields` (MILE-90/ADR-44) — otherwise `header.field-set-consistency` would never have heard of it. |
| `config.pointer-narrative-overlap` | A field must be exactly one kind: named in both `pointer_fields` and `narrative_fields` for the same type is a contradiction (MILE-90/ADR-44). |

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
  is not. A rule reporting a zero population is `not-run`, not `ok` — §5 of RFC-11, applied to a
  rule's input rather than its output.
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
> | 2026-08-20 | Split out of SPEC-1, which retains the cross-cutting rules. | **structural** |
> | 2026-09-07 | Added the Embodiment consistency, drift, and locator-promotion rules (ADR-18/ADR-32) to the rule set — previously implemented but never listed here. Noted that the rest of this section's rule names predate and don't match the actual shipped rule ids, as a named gap rather than silently compounding it. | **substantive** |
> | 2026-09-09 | Rewrote §Rules for MILE-90/ADR-44: `pointer.resolution`/`header.pointer-field-clean` are now config-driven (`pointer_fields`/`narrative_fields` per type, no hardcoded field list); `blocked-on.stale` renamed `narrative-field.stale`, generalized beyond `Blocked-on`; added the three new config-level rules (`config.pointer-declaration-missing`, `config.pointer-field-not-known`, `config.pointer-narrative-overlap`) and the two rows this table had never listed at all (`type.no-declared-spec`, `header.deprecated-shape`) despite both already shipping. Corrected the stale "ten rules" framing to the real, current seventeen. **Why:** this table drifting behind the actual shipped rule set is the exact recurring gap this spec's own 2026-09-07 entry already named once; MILE-90 touched every one of these rule functions directly, making this the natural point to close the gap rather than let it recur a third time. | **structural** |
> | 2026-09-07 | Rewrote §Rules as a complete, accurate list of all ten actually-shipped rule ids (adding `header.layout-consistency`/ADR-38 and `header.field-set-consistency`/ADR-39, neither previously mentioned at all), separated from the design language that's still unbuilt. Corrected §Out of scope's claim that cross-record reconciliation is `audit`-exclusive -- `check` has run `relation.supersession-reciprocity` directly since ADR-30, and the actual data-loss risk this spec was guarding against was a bulk *rewrite*, never a read-only reciprocity check. **Why:** per ADR-14's amendment adopted earlier the same day, a spec's body must stay a complete, replayable specification of its subject at every revision, not accumulate "not yet reconciled" notes as a substitute for actually updating it -- leaving this stale on the very day that policy was adopted would have been an immediate, visible contradiction. | **substantive** |
> | 2026-09-07 | Added `blocked-on.stale` as an 11th rule (ADR-42); `pointer.resolution`'s row updated to include `Parent`/`Blocked-on`, which it had already gained (ADR-40) without this table being updated. **Why:** the same "spec must stay complete" policy applies to every rule addition, not just the ones made on the day the policy was adopted -- letting this table go one rule stale again immediately would have repeated the exact drift this spec was just corrected for. | **substantive** |
> | 2026-09-08 | Reworded the header-format-consistency seed rule's rationale (§"The two seed rules") from a present-tense claim about the corpus's current state to historical framing. **Why:** ADR-33/ADR-42's corpus-wide `yaml-frontmatter` migration made the original wording -- "this corpus of 22 records carries three header formats" -- false the moment it landed; caught by adversarial review before the migration shipped, rather than left as another stale-prose instance for a future pass to find. | **substantive** |
> | 2026-09-08 | Bumped to `0.5`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
