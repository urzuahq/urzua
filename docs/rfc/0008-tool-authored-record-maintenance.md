# 0008 — Tool-authored record maintenance: derived fields are a cache the tool owns

> Status: Draft
> Date: 2026-08-07
> Author: (session author)
> Supersedes / Superseded-by: —

## Summary

Propose that Urzua **write** records, not only read them. Every field whose correct value is
mechanically derivable from evidence already in the tree is a **cache of a computation**, not
authored content, and the tool that computes it owns keeping it fresh. Define the eligibility test
that separates a cached field from an authored one, a taxonomy of the repairs this permits, and the
delivery mechanism — reusing RFC-0002's detect/apply split wholesale, writing every change into
RFC-0006's append-only revision log as `structural`, and never touching prose. The claim is that
this is not a `--fix` convenience: without it the embodiment model in RFC-0005 cannot survive a
moving codebase, because every record becomes stale-by-default the moment the code around it moves.

## Motivation

The direct trigger: an audit run reported a decision record whose
stated embodiment (`Implemented`) disagreed with the embodiment the audit had *just computed from
the tree* (`Verified`). The tool held both values in one pass, printed a warning, and left the
record wrong — because writing was not on the table. The drift was confirmed pre-existing, i.e. it
had survived every prior run of the same check.

That is not one bug. Reading §27 back through the earlier findings, the same shape appears **five
times across three sessions**, every instance reported-not-repaired:

| Wrong value in a record | Correct value was | Finding |
|---|---|---|
| Embodiment field (`Implemented` vs computed `Verified`) | computed in the same pass | §27 |
| Realization back-link paths (4 occurrences at a superseded script) | knowable from the tree | §25.2 |
| A derived identifier quoted as a literal, never valid at any point | recomputable from source | §25.4 |
| A relationship claimed without its reciprocal half | derivable from the half that exists | §26 |
| Display numbers / numbering gaps | derivable from the corpus | §26 |

The reflex against auto-writing is well-founded and comes from real damage: the reciprocity-checker
data-loss incident, which is why `--fix` was explicitly deferred out of `docs/specs/0001-v0-cli.md`'s
`audit`. But that incident rewrote **authored** cross-references embedded in prose, in a corpus with
no revision log to make the rewrite recoverable. Both of those conditions are now addressable, and
neither describes the table above. Collapsing "may rewrite a derived field" and "may rewrite prose"
into one prohibition is precisely what left five known-wrong values sitting in a real corpus.

The asymmetry is the argument: **if a computation is trusted enough to warn a human, it is trusted
enough to write.** Warning-only is not the conservative choice — it is the choice that produces a
corpus of records nobody believes, plus a warning stream too long to read (§24, §26).

## Proposal

### 1. The eligibility test — a property of the field, not of the operation

A field is **tool-writable** if and only if all four hold:

1. **Derivable** — its correct value is computable from evidence present in the tree, with no
   inference. Same mechanical-not-LLM bar RFC-0005 §1 already sets for computing Embodiment.
2. **Single-valued** — exactly one correct value exists at any moment. A field where two values could
   both be defensible is an authored field wearing a derived field's clothes.
3. **Non-destructive on write** — the value it replaces is a stale copy of the same computation, not
   something a human composed. Overwriting it destroys no authorship.
4. **Reversible** — the prior value is recoverable from the revision log entry the write produces.

Anything failing any clause is **permanently hand-authored**: rationale, consequences, alternatives
considered, and every statement of *why*. No confidence score makes prose eligible. This is the
line, and it is deliberately drawn at the field, not at the tool's certainty on a given day.

### 2. Repair classes

Three tiers, in increasing order of nerve required — and Tier 1 is worth shipping alone.

**Tier 1 — Recompute a derived field in place.** The stated value disagrees with the computed value;
write the computed value. Covers embodiment status, corpus fingerprints, display numbers, index
entries, and any `computed:`-namespaced field the schema grows later. This is the §27 case. It is
purely idempotent: running it twice changes nothing the second time, which is a testable property
and should be a fuzz target.

**Tier 2 — Complete a half-stated relationship.** One record names another; the counterpart does not
reciprocate. Write the missing half (§26). Strictly additive — it *adds* a back-pointer, never edits
or removes an existing one. Status-aware per SPEC-0001: a claim only binds once the claiming record
is in a terminal-accepted state, so an unaccepted record's claim produces no write.

**Tier 3 — Repoint a stale locator.** A realization back-link names a path that has moved (§25.2).
Only permitted where the move is *unambiguously* recoverable — git rename detection above a
similarity threshold, one candidate only. Ambiguity produces a finding with a `suggested_action`, not
a write. This is the tier closest to the data-loss incident and the one to ship last, behind the
other two proving themselves.

**Explicitly not a tier: deleting a rotted derived literal from prose (§25.4).** The correct fix
there was deletion of a sentence, which is a prose edit. The tool reports it; a human deletes it.
That finding stays a finding, and the fact that the most-obviously-wrong value in the corpus is the
one thing this RFC refuses to touch is the clearest available illustration of where the line sits.

### 3. Delivery — reuse RFC-0002 exactly, do not invent a second mechanism

RFC-0002 already designed detect/apply for one repair class (`realized_by` hash refresh). This RFC
claims that design generalizes and should not be re-litigated per class:

- **`urzua fix` (detect mode, default)** — read-only. Emits the same structured JSON shape as
  RFC-0002's detect: one entry per repairable fact, carrying `record`, `field`, `currentValue`,
  `computedValue`, `tier`, `evidence` (the specific paths/lines the computation read), and
  `suggested_action`. Safe in CI; this alone is a Phase 1 feature.
- **`urzua fix --ids <...> --by <identity>`** — scoped apply against reviewed IDs, identity resolved
  by RFC-0002 §2's tiering (`gh api user` → `git config` → explicit override).
- **`urzua fix --force --by <identity>`** — batch bypass. Never the default.
- **Tier gating**: `--tier` defaults to 1. Tiers 2 and 3 are opt-in per invocation.

Every applied write appends an RFC-0006 revision-log entry classified **`structural`**, naming the
computation and the evidence it read. A tool-authored write that could ever be legitimately
classified `substantive` is out of scope by construction — if a repair changes what the record
*means*, clause 3 of the eligibility test already excluded it.

### 4. Concurrency

Backfill and repair are the same shape of problem: many units of work writing records at once
(Q16). This RFC does not solve that and must not pretend to — apply is single-process, holds a
corpus-level lock, and re-verifies each ID still shows as repairable at apply time before writing
(RFC-0002's leaning, adopted here as a requirement rather than an open question, because a stale ID
under a batch apply writes a value computed against a tree that no longer exists).

### 5. What this makes possible that warning-only cannot

The point of §1's framing is that it changes the default state of a corpus. Under warning-only, a
record is correct at authoring time and decays monotonically; every code movement produces a
warning, warnings accumulate into an unread stream (§24, §26), and the corpus's trustworthiness
falls until nobody uses it. Under tool-authored maintenance the derived layer is *always* current,
and the warning stream reduces to the things a human genuinely must decide. That is also the honest
answer to "why would anyone install this" for a codebase that already has records: not backfill
(they have records), but that their records stop rotting.

### 6. A derivation is only as trustworthy as its own test

The eligibility test in §1 rests on a derived value being recomputable from evidence. That assumes
the recomputation is right, and a wrong derivation is a distinct failure from a stale field: it
produces a well-formed, plausible value that no amount of re-running corrects.

Consider the shape of this: a value derived from repository
history used a query that silently answered a slightly different question, returning a date that was
correctly formatted, monotonic, and older than the truth — so the output read as a worse instance of
the problem being measured rather than as a broken measurement. Every unit test on the formatting
passed; the defect was entirely in what the formatter was handed.

Two requirements follow for any field this RFC would let a tool author.

- **A derivation needs a test that fails against the plausible wrong implementation.** For values
  derived from history or from a search, the plausible wrong implementation is a near-neighbour
  query that also returns well-formed answers, so a test that merely passes against the correct one
  demonstrates nothing. Constructing that test usually forces the derivation to accept its inputs as
  arguments rather than read them from ambient state, which is the same shape that makes it
  testable at all.
- **The query itself belongs in the record.** Two searches that differ by one flag are
  indistinguishable in their output. If the record says only "derived from history", the difference
  is unreviewable; if it names the query, a reader can disagree with it.

This does not change the eligibility test. It adds a precondition to acting on it: a tool may only
overwrite a field whose derivation is tested against being wrong, not merely against being absent.

## Open questions

- **Does an auto-repaired record need to signal that it was auto-repaired in its own body**, or is
  the revision-log entry sufficient? Leaning revision-log-only — a `computed:` namespace in the
  frontmatter would carry the same information structurally and is the cleaner form, but that is a
  schema change to RFC-0001, not settled here.
- **Should Tier 1 run automatically in the pre-commit hook** rather than as a separate command? It is
  idempotent and non-destructive, which is the strongest possible case for it — but it makes the hook
  mutate the tree, which is a materially different thing for a hook to do and needs its own decision.
- **Where does the boundary sit between "repair the record" and "the record needs superseding"?**
  This is Q9 arriving from a new direction: enough accumulated structural repair may itself be
  evidence the decision has drifted rather than merely moved. Undesigned.
- **Cross-branch**: §25 established that a coherence check cannot see referents on another branch.
  Repair inherits that limit and makes it sharper — repairing against a partial view could write a
  *wrong* value where warning-only merely produced a false alarm. Tier 3 in particular must refuse to
  write when it cannot establish branch scope. Mechanism undesigned.
- Does the eligibility test's clause 2 (single-valued) hold for display numbers under concurrent
  merges? SPEC-0001 already flags that as the hardest correctness problem in v0.

## Non-goals

- **Rewriting prose.** Rationale, consequences, and alternatives are hand-authored permanently.
- **LLM-proposed repairs.** Every write in this RFC is mechanically derived or it does not happen.
  An LLM-assisted *suggestion* layer is a different feature, now proposed separately in **RFC-0009**,
  which consumes this RFC's detect output as its work list and never writes.
- **Unattended/cron apply.** Inherited from RFC-0002's non-goals: every apply path requires a named
  identity.
- **Creating or deleting records.** Repair operates on records that exist. Backfill (creation) is a
  separate feature with its own unresolved trust question (Q2).
- **Repairing Embodiment**, despite it satisfying every clause of §1's eligibility test. When a
  stated Embodiment exceeds the computed one, two repairs exist — supply the missing evidence, or lower the claim — and they are
  indistinguishable to the tool. Auto-repair would resolve every such case by lowering the claim,
  because that is the mechanical answer, and it is the wrong one precisely when the evidence is
  missing rather than the claim being wrong. **This is the first field found where the eligibility
  test admits something it should not**, so it is a hole in §1 rather than a special case; RFC-0012
  §5 carries the argument.
- **Solving the concurrent-writer race** (Q16) — §4 sidesteps it with a lock rather than answering it.

## References

- RFC-0002 — the detect/apply split and identity resolution this RFC reuses rather than redesigns.
- RFC-0005 §1–2 — mechanical-not-inferred computation, and the `realized_by` pointer model whose
  staleness Tier 3 repairs.
- RFC-0006 — the append-only revision log that makes clause 4 (reversibility) true.
- `docs/specs/0001-v0-cli.md` — the deferred `--fix` and the reciprocity-checker data-loss incident
  behind that deferral; this RFC is the argument for narrowing that deferral rather than lifting it.
