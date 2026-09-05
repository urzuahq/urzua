# 0001 — A unified record schema: core fields plus declared profiles

> Status: Accepted
> Date: 2026-07-29
> Author: (session author)
> Supersedes / Superseded-by: —

> **Split, 2026-07-30.** This RFC originally also covered RFC→ADR→Spec layering, the
> Embodiment/`realized_by` claim-graph model, and the living-spec revision-log problem — four
> separable decisions in one document, with two of its three inline "Correction" notes belonging
> to content that's now been split out. Kept growing rather than being split as each new decision
> arrived, which made it hard to accept/reject one part independently of the others. Split into:
>
> - **This RFC (0001)** — the core schema/profile shape only (§1, §1a below).
> - **RFC-0004** — RFC → ADR → Spec layering (was §2).
> - **RFC-0005** — Embodiment as a schema-level concept, `realized_by` as a claim graph (was §3,
>   §3a, §3b — the largest and most-corrected part of the original document).
> - **RFC-0006** — the living-spec problem, append-only revision log (was §4).
>
> Content in each split-out RFC is unchanged from this document's original text, per this repo's
> own rule against silent rewrites. RFC-0002 previously cited "RFC-0001 §3a"/"§3b"; updated to
> cite RFC-0005 directly as part of this same split.

## Summary

Propose a single underlying record schema shared by RFC, ADR, and Spec (and, pending an open
scoping question about whether Product/PRD documents fit the same model, eventually those too).
This is Urzua's own first real design decision — written using this project's own lightweight RFC
shape, bootstrapping the process this repo exists to generalize.

> **Correction, 2026-07-30.** §1's `type` field as a closed enum is the wrong shape. The set of
> record types shouldn't be decided by the tool at all — Urzua needs to work for whatever record
> types an org actually uses, including ones invented after the tool ships. See new §1a, which
> draws on prior art from extensible-schema architectures (Terraform providers, Kubernetes CRDs).

## Motivation

A decision-record system that builds ADR + an RFC-shaped proposal document + Spec as three
*separately typed* documents, each with its own template, its own linter, and — critically — no
shared underlying schema, tends to rediscover the same conceptual bugs (blank-field misdetection,
reciprocity-checker data loss, silent no-op validation) independently in each one instead of fixing
them once. A unified schema is the direct fix: if RFC/ADR/Spec are the same
core record with different required fields and a different position in the lifecycle, then a
validator, a backfill engine, and a drift-detector only need to be built once.

## Proposal

### 1. One core schema, three (or more) profiles

A record has a **small, fixed core** — the fields every record type shares — plus **profile-specific
extensions** layered on top: a tiny common core plus level-specific extensions.

**Core fields (every record, regardless of type):**

| Field | Meaning |
|---|---|
| `id` | stable, unique, never reused |
| `type` | `rfc` \| `adr` \| `spec` \| (future: `product`, `prd`) |
| `title` | — |
| `status` | type-specific enum, see below — the *decision/proposal lifecycle* state |
| `author` | real identity, not free text |
| `reviewers` / `deciders` | real identities; presence-and-enforcement rules are type-specific (an RFC in Discussion doesn't need a Decider yet; an Accepted ADR does) |
| `date` | last-meaningfully-updated date, not creation date |
| `supersedes` / `superseded_by` | reciprocal pointers, status-aware binding: a claim only binds once the *claiming* record is itself in a terminal-accepted state |
| `related` | non-binding cross-references |

**Profile extensions (type-specific, on top of the core):**

- **RFC**: `motivation`, `proposal`, `open_questions`, `non_goals` — pre-decision, no code
  back-pointers possible yet by definition.
- **ADR**: `context`, `considered_options` (Y-statement/MADR-lite structure), `decision`,
  `consequences`, plus **`embodiment`** — the realization-tracking axis, independent of `status`
  (see RFC-0005).
- **Spec**: `purpose`, `design`, `interfaces`, `testing`, `security_considerations` (optional),
  `implements` (back-pointer to the ADR(s) it realizes) — plus the living-spec mechanism in
  RFC-0006.

This is deliberately **not** one flat schema with optional fields for everything — that's how you
get a document type that's nominally one thing but is actually three different documents wearing
the same file extension, which is closer to what happens when a schema is left to drift
organically — a specs directory quietly filling with RFC-shaped proposal documents under no
governing template — than what anyone actually wants.

### 1a. Correction — profiles are declared, not enumerated by the tool

The `type` field above is written as a closed enum (`rfc | adr | spec | ...future`). That's the wrong
shape. It bakes "which record types exist" into the tool itself, which contradicts the actual
requirement: an org should be able to define a new record type — a PRD, a design doc, an
implementation-level decision record, anything — **without Urzua shipping code for it.**

The core/profile split in §1 above is still right and doesn't change: a small fixed core (this
section's table) plus type-specific extensions on top. What changes is *where the profile comes
from*. Reframe as a **provider model** (after Terraform's provider architecture and Kubernetes CRDs): the
engine — validation, the
reciprocity/supersession checker, the revision log, Embodiment computation — is built once and knows
nothing about "ADR" or "RFC" specifically. A **profile** (plausibly a JSON Schema variant, following
the OpenAPI/CRD precedent rather than a bespoke DSL — open question below) declares a record type's
extension fields, its `status` enum and legal transitions, and whether Embodiment applies to it at
all. RFC, ADR, and Spec become the three profiles Urzua ships *by default*, not the three types the
schema hardcodes. An org adding a fourth profile is a data change, not a tool change.

This also reframes what "unified schema" is selling: the substrate isn't "one schema for three known
document types," it's "one engine that works for whatever types your org actually has" — arguably a
sharper substrate claim than the original, though the project's own rule to never lead with
substrate still holds.

### 1b. Addition, 2026-08-11 — status transitions are timestamped; `date` alone cannot carry the lifecycle

The core table above gives every record exactly one temporal field, `date`, defined here as
last-meaningfully-updated. **A single date cannot express a lifecycle the schema already models.**
`status` is declared as an enum *with legal transitions* (§1a), and nothing anywhere records when a
transition happened. A record accepted later than it was written — which is the normal case, not the
edge case — loses its acceptance date entirely.

The reason this is being added now rather than left as a nicety: consider a record accepted eight
days after it was written — the normal case, not the edge case. An agent accepting it invents a
header field on the spot to hold the acceptance date, because the schema has nowhere to put it. The
instinct is correct; the schema is the thing at fault. Corpus-wide, every record accepted later than
written has the same hole, recoverable only by git archaeology.

**Proposal: a `transitions` list in the core, not a named `accepted_on` field.**

```yaml
transitions:
  - to: Accepted
    on: 2026-08-11
    by: <identity>
```

One entry per status change, append-only, each carrying the state entered, the date, and the
identity responsible. `date` keeps its existing meaning and is unchanged.

**Why a list rather than the single field the incident actually wanted.** Adding `accepted_on`
solves the case in front of us and leaves the identical hole for every other transition —
`Superseded`, `Rejected`, `Deprecated`, and any transition a declared profile invents under §1a,
which the tool cannot enumerate in advance by design. Bolting on one named field per transition as
each is encountered is *precisely* the ad-hoc schema growth §18 is a finding about, done more slowly
and with better manners. A list is also the only form compatible with §1a: profiles declare their own
status enums, so the schema cannot name their transition fields.

**Three things this unlocks that are currently unbuildable**, each of which needs a date the schema
does not hold: proposal-to-acceptance latency (the first governance-health metric anyone asks for),
staleness of a long-`Proposed` record, and attestation expiry (roadmap Phase 3). The `by` field also
gives the decider a timestamp, which `deciders` alone does not.

**Backfill:** transitions are partly derivable from git history — the commit that changed a
`status` value dates and attributes that transition mechanically. That makes the historical
population an RFC-0008 Tier 1 repair (derived, single-valued, non-destructive) rather than hand
authoring, with backfilled entries carrying provenance and not asserted as observed (Q2 doctrine).

**How partial, in practice.** This derivation, run over a 91-record corpus, yielded a previous state
for 21 records and nothing for 70. The shortfall is not
only squashed commits. A record committed straight to its accepting state has no transition to
observe at all — and that is the cheapest way to skip a lifecycle, so the case the derivation most
needs to catch is the one it structurally cannot see. Verified on a purpose-built corpus: such a
record passes a legal-transition check cleanly.

That is an argument *for* this proposal, and worth stating as one: **the reason transitions belong
in the record is that they cannot be reliably recovered from outside it.** Backfill seeds what the
record carries from then on; it is not a mechanism the corpus can lean on. A checker deriving
transitions must report its input population per the SPEC-0002 rule — `91 records, 21 with readable
history` — because "no illegal transitions" over 70 unexamined records is a description of the
tool's blindness, not the corpus's health.

**Deliberately not decided here:** rollout into existing corpora. Populating this across a live,
large corpus is a migration with its own sequencing, out of scope for this RFC.

### 1c. The realization axis belongs on Spec too — and splitting an overloaded field means the old value *leaves*

`embodiment` is listed above as an ADR profile extension. Every record type that can be built needs
it, and adding it is not the hard part.

Consider a corpus whose spec records carry a single `Status` enum holding `Draft`, `Ready`,
`Superseded` **and** `Implemented` — three lifecycle states and one realization state in one field.
A spec cannot say "design final" *and* "built", and the corpus documentation has absorbed the
ambiguity by defining `Ready` as *"design final, implementation may be in progress or done"*.
Measured before any change: **12 of 37** specs had a `Status` that could not answer "is this
built?" — some claiming the realization value (so their lifecycle state was unknowable), the rest
carrying code back-pointers while still marked `Draft` (built, with no way to say so).

Two rules generalize out of the fix.

**The overloaded value must leave the original enum.** Keeping `Implemented` in `status` *alongside*
an `embodiment` field permits `status: Implemented` + `embodiment: not-started` — a direct
self-contradiction the checker would then have to catch forever. Removing it makes that state
**unrepresentable rather than checkable**. Stated generally:

> Two fields answering one question is worse than one field answering two: the ambiguity at least
> cannot disagree with itself.

A split that leaves the old value in place has bought a permanent consistency rule instead of
removing the need for one.

**Every reader moves in the same change, because the failure is silent.** Narrowing the enum without
moving its consumers yields a corpus that validates clean while a downstream report classifies every
record as unknown-status — no error, just wrong data. In that corpus the readers were two governance
documents, a phase gate keyed on the retired value, and — found only by a test asserting the
authoring path still worked — **the record template itself**, which continued to offer the removed
value as a choice. A template that opens in a state its own checker rejects is unusable, and nothing
had said so. For Urzua this is a `check` rule: **a template must validate against its own type's
profile**, which is cheap and catches the whole class.

The per-type ladders differ and the difference is declared, not flattened: a Spec has no `specified`
state, because the spec *is* the specification, so the value would be true of every spec the moment
it exists. An ADR keeps it, because an ADR can be decided with its spec unwritten.

### 1d. A relation field needs a stated grammar, or a parser gets written against whatever examples exist

A field holding references to other records — `supersedes`, `amends`, and their
reverses — is not free text and is not a bare list. Real corpora annotate each
reference, and the annotations mention *other* records:

```
supersedes: ADR-018 — for the remaining architecture not already
            superseded by ADR-020 and ADR-035
```

One claim, two mentions. A reciprocity check reading every reference in that
value demands that two live, accepted records be marked superseded.

The rule that works is structural: **a claim is the reference that begins an
entry; everything after it is annotation.** Entries are separated by a delimiter
*followed by a reference*, not by any delimiter, because annotations legitimately
contain commas.

Three things make this worth stating in the schema rather than leaving to
implementations.

**Without a stated grammar, the parser is written against the examples that
exist.** One implementation propped this up with a regex matching a single
phrasing — `already superseded by X`. It was the only thing preventing a false
blocking error on an accepted record, and a second way of writing the same
caveat would have broken it.

**The grammar must be enforced, not assumed.** Seven of seventy-three values in
that corpus were prose rather than lists, and five hid *real, reciprocated*
claims mid-sentence where no parser could find them — typically a final claim
joined with "and" instead of a comma. The check is not "does this parse" but
"does every entry begin with a reference".

**An entry naming a non-record target is legitimate and must stay legal.** A
record may supersede a document section, or an informal decision that was never
a record at all — there is nothing to reciprocate, and forcing every target to
be a record would mean inventing history. The grammar permits it precisely
because such an entry cannot hide a claim. The cost is that a record written in
a *superseded naming scheme* also reads as a non-record and is silently ignored,
so a profile that has renamed its records needs a companion check for legacy
spellings.

**Claims are type-qualified.** A record amending both `ADR-051` and `SPEC-013`
yields two claims against different types; returning bare numbers sends the
reciprocity check looking for the wrong record.

### 1e. Reciprocity is a two-way assertion or it is half a check

Checking that every forward claim is answered leaves the reverse unguarded: a
record can list anything in its backward field and nothing asks. A backward
field is what a reader of the *target* consults, so a false entry there sends
readers to a record that denies the relation.

Both directions cost one loop each. The asymmetry is easy to ship by accident
because the forward direction is the one you think of first, and the corpus
looks clean either way.

## Open questions

- Does a `transitions` list make `date` redundant, or does last-meaningfully-updated remain a
  distinct fact worth holding separately from any status change? Leaning distinct — a substantive
  content revision at unchanged status is a real event neither field would otherwise record — but
  two temporal fields that mostly agree is exactly the shape of thing that drifts.
- Does `Product`/`PRD` extend this same core+profile model, or does it need a genuinely different
  content model (narrative, not decision-shaped)? Deliberately not resolved here — needs its own
  RFC once there's a clearer sense of what a product-guide/user-journey record actually needs to
  capture that a decision record doesn't.
- What format should a profile declaration actually take — JSON Schema directly, an OpenAPI-style
  superset (buys versioning/conversion semantics for free, per the CRD precedent), or something
  purpose-built and lighter-weight? No comparative evaluation done yet. Related: does profile
  *versioning* (an org changing its own ADR profile's required fields later) need to work from day
  one, or can it be deferred past the first linter?

## Non-goals

This RFC does not attempt to design the paging/notification mechanism, the dashboard/UI, or any
specific storage/database technology. It's scoped to the core record schema and profile-declaration
model only — RFC-0004, RFC-0005, and RFC-0006 cover layering, Embodiment/realization, and the
living-spec problem respectively.

## References

- RFC-0004, RFC-0005, RFC-0006 — the layering, Embodiment/claim-graph, and living-spec content
  split out of this document's earlier draft.
