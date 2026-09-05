# 0002 — Own the record format; be AgDR-compatible on export

> Status: Accepted
> Embodiment: Not started
> Date: 2026-07-29
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0001 (Accepted)

## Context

**Agent Decision Records (AgDR)** is a public, CC BY 4.0 open standard for recording decisions made
by AI coding agents. YAML frontmatter plus Markdown body; required fields `id`, `timestamp`,
`agent`, `model`, `trigger`, `status`; a Y-statement one-liner; an options table; a JSON Schema and
a Node validator; and drop-in integrations for Claude Code, Copilot, Cursor, Codex, and Windsurf.

Its problem framing is nearly identical to Urzua's — *"AI coding agents now make consequential
technical decisions at machine speed… the context evaporated the moment that session ended."* That
independent convergence is useful evidence that the problem is real and that the framing isn't
proprietary to us.

Its traction is not comparable: 42 stars, 11 commits, a single individual author, one consultancy
listed as an adopter. There is no ecosystem to join — no critical mass of tooling, no
interoperability benefit available today, no community process to participate in.

**The strategic reading matters more than the artifact.** A capable individual produced the format,
schema, and validator in 11 commits. That is the correct measure of what a decision-record *format*
is worth: it commoditizes to approximately zero. This confirms rather than threatens the project's
own positioning — schema is substrate, never the lead. What AgDR conspicuously lacks is
everything above the format: no realization/drift verification, no escalation, no backfill, no
concurrency safety, no org-level layer. It records; it never checks whether the record is still
true.

The question this ADR answers is therefore not "is AgDR good" (it's a well-shaped artifact) but
"what does adopting it as our native format buy us," and the answer is: no ecosystem, no
credibility we can't get otherwise, and a constraint on the fields our own features require.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| **Adopt AgDR as the native format** | Instant "we support the open standard" story; no format debates with prospects; reuse of their JSON Schema | Constrains the schema to one individual's design; no governance process to influence it; blocks core+profile extensions (Embodiment, reciprocity, revision log) that RFC-0001 requires; ties us to an artifact that may be abandoned |
| **Own the format; export AgDR** *(chosen)* | Full control over the fields our differentiating features need; compatibility available on demand; near-zero implementation cost | Must answer "why not the open standard" repeatedly; carries the maintenance burden of a format |
| **Own the format, ignore AgDR entirely** | Simplest | Gratuitously forecloses interop; loses a cheap credibility signal; the compatibility layer is genuinely inexpensive |
| **Co-opt: contribute Urzua's model upstream** | Standards credibility; could shape the spec | Slow, requires a counterparty with no incentive structure, and cedes format control for a benefit that doesn't exist at this traction level |

## Decision

In the context of a decision-record format that a single individual demonstrably built in 11
commits, facing an "open standard" with no ecosystem to interoperate with, we decided to **own
Urzua's record format outright and provide AgDR-compatible import and export**, to keep the schema
free to carry the fields our differentiating features require (Embodiment, supersession
reciprocity, revision log, provenance and confidence, stable identifiers), accepting that we
maintain a format and must repeatedly justify not adopting the standard.

Concretely:
- Urzua's native format is the core+profile schema from RFC-0001.
- `urzua export --format=agdr` emits AgDR-compliant records; `urzua import` reads them.
- Fields with no AgDR equivalent are preserved on round-trip rather than silently dropped —
  **lossy export must warn**, since silently discarding fields is precisely the data-loss class this
  project exists to prevent.
- Good ideas from AgDR are adopted directly where they're better than ours. Their `agent`, `model`,
  and `trigger` fields are a genuine improvement on anything in RFC-0001 for agent-authored records:
  *which* agent, running *which* model, triggered by *what*, is exactly the provenance an
  accountability layer needs and RFC-0001 currently lacks. **Fold these into the core schema.**

## Reversibility

**Highly reversible.** Records are plain files with a documented schema, and a format migration is a
mechanical transform we would have to support anyway (it's the same machinery as `import`). If AgDR
ever gains real ecosystem traction, adopting it natively later is a migration, not a rewrite.

This asymmetry is the reason to decide quickly rather than deliberate: the cost of being wrong is
low and bounded, while waiting blocks the schema work that everything else depends on.

## Consequences

- RFC-0001's core schema gains `agent`, `model`, and `trigger` — adopted from AgDR. This is a
  substantive change to a `Draft` RFC and must be recorded as such.
- v0 must ship `export --format=agdr` and lossy-export warnings, small but non-optional scope.
- "Why not the open standard?" needs a written answer for the site and sales conversations. This ADR
  is that answer.
- We carry format-maintenance burden, including versioning and migration, from day one.
- No dependency on AgDR's continued maintenance — relevant given its single-maintainer, 11-commit
  history.
- Reinforces the positioning discipline: if a format is an 11-commit artifact, the product is
  unambiguously the layer above it, and any roadmap that drifts toward format work is drifting away
  from value.

## References

- `docs/rfc/0001-unified-record-schema-core-and-profiles.md` — the core+profile schema this preserves; gains
  three fields as a result of this decision
- AgDR — public open standard, CC BY 4.0
