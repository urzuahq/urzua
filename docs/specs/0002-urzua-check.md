# SPEC-0002 — `urzua check`

> Version: 0.1 | Date: 2026-08-20 | Status: Draft
> **Implements:** RFC-0001, RFC-0003, RFC-0010, RFC-0011
> **Parent:** SPEC-0001 (v0 CLI). Cross-cutting rules — no-silent-no-op, the permanent
> content-scope ceiling, the three-corpus acceptance bar — are stated there and are not restated
> here except where this command narrows them.

## Purpose

The validator, and the whole of Phase 0. Everything else in the roadmap parses and validates
records first, so this is the one piece nothing routes around.

`check` reads records, applies rules, and reports findings. It never writes. `--fix` is deferred to
RFC-0008 and is out of scope here for a recorded reason: a bulk cross-reference rewrite caused a
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
| 6 | It reproduces every documented bug in SPEC-0001's acceptance suite against real corpora (SPEC-0004) | D |

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

**Corpus zero is `docs/` in this repository.** SPEC-0004's anonymized corpora are required by
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

1. **Header format consistency within a record type.** This corpus of 22 records carries three
   header formats: RFC and ADR use one key per blockquote line; SPEC-0001 uses that shape while
   SPEC-0002 through SPEC-0004 use a pipe-delimited single line plus bold keys and a wrapped
   continuation. The third format arrived in this repository's own corpus after header closure was
   specified — which is the failure mode RFC-0010 describes, arriving in the corpus of the tool
   meant to prevent it.
2. **`Implements:` / `Derives-from:` resolves, and the target's status is reported.** Six pointers
   in `docs/specs/` currently name RFCs that are all `Draft`. That is RFC-0012's
   decision-before-implementation gate with live subjects. Phase A only needs the pointer to
   resolve and the target status to be surfaced; whether a `Draft` target is an error, a warning, or
   the normal state of a young corpus is a decision for RFC-0012, not for the checker.

Both are also the cheapest possible proof that the checker reads structure rather than greps text.

### Phase B — the rule set

The remaining rule families in §Rules, severity resolved from configuration (SPEC-0003), and
`doctor`. Each rule lands with its planted-violation test and is exercised against `docs/` the day
it merges.

### Phase C — the second corpus

The falsifiable one. A second codebase runs green under the same binary with no code change, and its
hand-written linter is deleted. A failure here is a schema
finding to record rather than a code path to add.

### Phase D — the historical suite

SPEC-0004's corpora and the documented bug classes. This is what turns "it works on two corpora" into
criterion 3, and it is last because it is the only phase whose blocker is not engineering.

## Discovery

**Default scope is what the VCS tracks**: `git ls-files` ∪ staged paths, intersected with the record
directories configured in SPEC-0003. Never a raw directory walk.

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
legitimately hold a number without stating a decision — a tombstone (RFC-0013) — and a check that
presupposes a decision is not wrong, it is *scoped*. A corpus that cannot express that forces its
records to invent content or its linters to be switched off.

### The rule set

- **Field presence with real semantics.** `blank`, `placeholder` and `pending` are three distinct
  states, not one — a commonly rediscovered gap in hand-written record linters, and the single
  most-repeated bug class. An absent field must not be silently readable as any of the three.
- **Status enum validation, tolerant of trailing free-text annotation**, and validated against
  corpus *prose*, not only instantiated values: a value that no record currently holds may already
  be committed to by another document, and is real a release before anything is set to it.
- **Role rules** — Author / Reviewers / Deciders per type and status, with self-acknowledgement
  detection: an author is not their own reviewer of record.
- **Filename ↔ title ↔ display number ↔ stable ID consistency** (ADR-0003).
- **Required sections** per profile, including `change_class` on revision entries.
- **Header structure** (RFC-0010): the header is closed. Anchored by signature key, bounded by the
  first section heading, closed key set with an `x-` escape hatch, exactly one entry per key, and
  multi-line continuations preserved.
- **Declared scope** (RFC-0011 §6): a record's scope resolves against tracked paths. A scope
  matching nothing is *unverified*, not clean. A record cited from outside its declared scope is a
  finding that names both readings — miscitation, or a scope never widened to match practice.
- **Boundaries** (RFC-0011 §1): identifiers inside a `published` boundary are violations;
  realization markers inside one are not evidence.

Severity per rule comes from config (SPEC-0003), not from the rule's own opinion. `status` reports
what was found; whether that fails the build is the caller's choice and lives in `blocking`.

## Output contract

Implements RFC-0003. Human format on stderr; the machine channel is stdout and carries nothing else,
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
      "file": "docs/adr/0042-a-record.md",
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

- **Every rule ships with a planted-violation test** (RFC-0011 §5). An absence assertion carries no
  information until its failure has been observed; a rule with no such test is unverified, not
  passing.
- **Corpus tests run against SPEC-0004's anonymized corpora**, not synthetic fixtures, for every
  bug in SPEC-0001's acceptance suite.
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
  is not. A rule reporting a zero population is `not-run`, not `ok` — §5 of RFC-0011, applied to a
  rule's input rather than its output.
- **Fixture containment.** The rule tests deliberately contain record-shaped strings, and a
  self-referential fixture that the checker then scans is a documented trap. Fixtures live outside
  the discovered record set, and that exclusion is asserted rather than assumed.

## Out of scope

- `--fix` in any form. See RFC-0008; needs the revision log first.
- Cross-record reconciliation — that is `audit`, and its write path is where the data-loss incident
  happened.
- Judging whether a section's *content* covers the scope it claims. SPEC-0001 records this as a
  permanent ceiling on mechanical checking, not a maturity gap. `check` is never sold as replacing a
  human reader.

## References

- SPEC-0001 — v0 CLI: success criteria, acceptance suite, the permanent ceiling.
- SPEC-0003 — configuration, which supplies record types, severities and boundaries.
- SPEC-0004 — the corpora this is tested against.
- RFC-0003 (output), RFC-0010 (header), RFC-0011 (boundaries and scope), RFC-0013 (status-scoped
  checks), ADR-0003 (identity).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-08-20 | Split out of SPEC-0001, which retains the cross-cutting rules. | **structural** |
