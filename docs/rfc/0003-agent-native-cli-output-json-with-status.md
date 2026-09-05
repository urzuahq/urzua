# 0003 — Agent-native CLI output: every entry point emits JSON with a status field on stdout

> Status: Accepted
> Date: 2026-07-30
> Author: (session author)
> Supersedes / Superseded-by: —

## Summary

Every governance-tooling CLI entry point (lint, audit, and any future subcommand) should treat
stdout as a data channel, not a log — a single JSON object with a `status` field, always, as its
primary output. Human-readable prose, progress messages, and CI-specific annotation formats (e.g.
GitHub Actions' `::error file=...::message` syntax) move to stderr, which remains free-form.
Extends the precedent already set by RFC-0002's `--refresh` detect mode (JSON-on-stdout,
read-only) to the tools' *normal*, non-refresh invocations too — lint and audit, not just the
refresh subcommand.

## Motivation

A tool like `adr-audit.ts` might already half-do this: `docs/adr-audit/summary.json` written
to disk specifically because a human-readable report "isn't scraping generated markdown to get
the data back out." That instinct — a
downstream consumer (there, a PR-comment script;
here, potentially any agent) needs structured data, not prose — is correct, but it only reached
the *audit* tool's file output, not the *lint* tool's console output, and not the *audit* tool's
own stdout during a normal run (only `--refresh` got this treatment, added in RFC-0002's
implementation).

The actual audience for these CLIs has shifted. They're invoked constantly by an agent working
inside the repo, not primarily by a human reading a terminal. A human reading formatted `WARN`/`ERROR` lines is
now the secondary case, not the primary one — the tool should optimize its default output for the
consumer that's actually there most of the time.

## Proposal

### 1. stdout is JSON, stderr is everything else

Every CLI entry point's **final, primary output on stdout is exactly one JSON object**, always
including a `status` field. Shape sketch (per-tool fields vary; the `status` field and the
stdout/stderr split are the load-bearing parts, not the exact schema):

```json
{
  "status": "ok" | "warn" | "error",
  "errorCount": 0,
  "warnCount": 3,
  "issues": [ { "type": "WARN", "file": "...", "message": "..." } ]
}
```

Everything that isn't the final JSON object — progress messages ("Found 32 ADRs. Scanning specs
and code..."), the formatted `WARN`/`ERROR` lines a human would scan while iterating locally, and
GitHub Actions' `::error file=...::message` annotation lines — goes to **stderr**. This isn't a
new constraint invented for this RFC; GitHub Actions annotations are already documented to work
from either stream, so moving them to stderr costs nothing there. A human running the tool
directly in a terminal still sees everything (stdout and stderr both render to the same terminal
by default); an agent piping stdout alone gets clean, parseable JSON with no prose mixed in.

### 2. Exit code stays the actual gate; JSON is additive, not a replacement for it

The process exit code remains the authoritative pass/fail signal for shell scripts, pre-commit
hooks, and CI step success — this RFC does not propose replacing `process.exit(errorCount > 0 ? 1
: 0)` with "parse the JSON to determine pass/fail." The JSON's `status` field is for an agent (or
a human) that wants to *understand* the result, not for scripts that only need to know whether to
proceed.

### 3. `status` values are tool-defined, not a single global enum

A lint tool's meaningful states (`ok`, `warn`, `error`) differ from an audit tool's (`clean`,
`drift`) or a future tool's own domain. This RFC doesn't mandate one shared enum across every
tool — consistent with RFC-0001 §1a's already-established position that Urzua shouldn't hardcode
a closed set the tool owns, when the actual domain concept varies per record/tool type. What's
mandated is that `status` exists, is a string, and is the first thing worth reading in the object.

### 4. This generalizes the existing `AuditSummary`/`DriftReport` shape, doesn't replace it

A `buildAuditSummary()`-style function (audit's on-disk `summary.json`) and RFC-0002's
`detectRealizedByDrift()` (refresh's stdout `DriftReport`) already use a `{ generated, ...,
entries: [...] }` shape with per-entry status-bearing fields. This RFC's stdout convention is the
same shape family, applied consistently to every entry point's primary output rather than to some
of them — not a competing format.

### 5. The per-run result is the collection point for anything that wants history

A corpus cannot keep its own governance history:
an in-repo log has to be written by CI, CI holds a read-only token against a protected branch, and
a log that silently fails to append is worse than none — it looks like a record and is not one.

The consequence for this RFC is that **the JSON emitted per run is the history**, or rather the only
part of it a repository-side tool can honestly produce. A run reports what it found and what it
examined; whatever wants a trend keeps the runs. That inverts the natural assumption — the tool does
not maintain a series inside a corpus it can only read — and it puts a requirement on the shape:

- **Every run must be self-dating and self-scoping.** A collected result is useless without knowing
  when it was produced, against which commit, and over what scope. `filesExamined` and `scope` cover
  the last; the commit and timestamp must be in the payload rather than inferred from where it was
  collected.
- **The shape must be stable across versions**, because a trend is a comparison across time and a
  field that changes meaning silently corrupts every comparison spanning the change.

This is what makes the roadmap's drift-trend feature buildable without asking a customer's repository
to store anything: the CLI already emits the datapoint on every run, in CI, on every merge. A
service that receives them has the series for free, and the same payload serves the single-run
agent-facing case this RFC was written for. **One output, two consumers, no second mechanism.**

Worth separating from a nearby question it is often confused with: "is this alert new" does not need
a series at all — it is answerable per record from the corpus's own history at report time, and
should be, because it must work in a fresh clone with nothing collected yet.

**Two constraints make the collected series event-shaped rather than state-shaped.**

The first is environmental and general: **a protected default branch with a read-only CI token is
the normal configuration, not an unlucky one.** Any design where the tool writes history back into
the repository requires an organization to grant an exception for a log file, which most will
decline and none should have to. That rules out an in-repo series as a product mechanism, not merely
as a local inconvenience. What CI can write without any such exception is its own output — check
runs, job summaries, and pull-request comments — and those are durable and queryable through the
forge API.

The second follows from the tool working: **once drift blocks a merge, the default branch is
drift-free by construction.** A state-over-time series against the main line is then a row of zeros
with transient spikes where an unrelated merge invalidated a record's claim before the next run
corrected it. Storing that faithfully would be building infrastructure to record an absence.

So the thing worth collecting is the **event**, not the state: drift caught on a proposal, drift
resolved, the interval between them, a record whose claim has gone unverified for N days. Those are
generated at moments the tool already runs, they carry their own timestamp, and they land in surfaces
a read-only token can write. A dashboard assembled from events answers "how often does this happen
and how long does it take to fix" — which is the governance-health question — where a state series
answers "is it broken right now", which the gate already answers by refusing the merge.

One in-repo mechanism survives the constraint, and it is worth naming because it needs no exception
at all: a record's own revision log (RFC-0006) is written by the change that causes it, travelling
through the normal review path. History that the authoring change records itself is always available;
history that a job appends afterwards is not.

## Open questions

- Should a tool's stdout JSON *also* be written to a stable on-disk path (mirroring the existing
  `summary.json` convention), so a review can reference a path instead of re-running the tool to
  reproduce output? RFC-0002 left the same question open for `detect`'s report; likely the same
  answer applies here, undesigned in either place yet.
- For a tool whose current human-readable output is large (dozens of WARN lines across a big
  corpus), does the stdout JSON's `issues` array get truncated/paginated, or is "large JSON" an
  acceptable cost given the alternative (prose an agent has to re-parse) is worse? Leaning toward
  no truncation — a consumer that wants a summary can compute one from the array — but not fully
  reasoned through.

## Non-goals

- Redesigning what each tool actually checks — this is purely an output-format change.
- A shared cross-tool JSON schema/status enum — see §3, deliberately left per-tool.
- Removing GitHub Actions annotations — they move streams (stdout → stderr), not away.

## References

- RFC-0002 (`0002-review-before-apply-refresh-workflow.md`) — the `detectRealizedByDrift()`
  precedent this RFC generalizes from refresh-only to every entry point.
- RFC-0001 §1a — the "don't hardcode a closed set the tool doesn't own" reasoning informing §3's
  per-tool `status` values.
- The motivating scenario: an agent working inside a repo repeatedly invoking a lint/audit CLI
  directly and needing to re-parse formatted `WARN`/`ERROR` text lines to determine what happened,
  when the tool already computes the structured version of that same information internally.
