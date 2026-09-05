# 0009 — The agentic repair tier: an unreliable generator behind a mechanical verifier

> Status: Draft
> Date: 2026-08-07
> Author: (session author)
> Supersedes / Superseded-by: —

## Summary

RFC-0008 draws a hard line: Urzua writes only fields whose correct value is mechanically derivable,
and prose is permanently hand-authored. That line is right, and it leaves a large class of real,
observed staleness unrepaired — rotted derived literals in prose, rationale describing an approach
the code has since abandoned, sections whose *content* no longer
covers the *scope* they claim (SPEC-0001's "permanent ceiling"). Propose a second repair tier where
an **agent** proposes those edits, structured after the review-command pattern harnesses already
ship (`/security-review`, `/code-review`): invoked explicitly or by hook, scoped to a work list the
mechanical tier produced, and — the part that makes it safe — **every proposed change is re-verified
mechanically before a human ever sees it.** The agent is an unreliable generator behind a
deterministic verifier. It never writes; it emits a reviewable diff whose every hunk is either
evidence-backed or dropped.

## Motivation

Two facts sit in tension in this repo.

**One:** the largest and most obviously wrong value in the source corpus is the one RFC-0008
explicitly refuses to touch. §25.4's record quoted a function's derived identifier as a prose
literal, invalidated twice, *never valid at any point*. The mechanical tier cannot fix it because
the fix is deleting a sentence. It stayed wrong until a human found it by reading. It is not alone —
SPEC-0001 names content-scope correctness a **permanent** ceiling on mechanical checking, not a
maturity gap, and §27 established that "reported but not repaired" is how staleness actually
survives here.

**Two:** Q2 — the project's largest product risk — is whether inferred, LLM-authored record content
is trusted at all. Backfill's whole mitigation doctrine (provenance, confidence, stays `Draft`) is
untested. Letting an agent edit rationale is the fastest available route to the failure mode the
product exists to prevent: a corpus of confident invented reasoning.

The resolution is not to pick a side. It is that **generation and verification are different jobs
with different reliability requirements**, and the review commands harnesses already ship are the
existing proof of the pattern: a model generates broadly across a diff, and its output is only
useful because it lands in a review surface where a human adjudicates each item. That pattern is
directly portable, and Urzua is in an unusually good position to run it — unlike a generic review
command, it *already has a deterministic checker for its own artifact type*, so it can verify the
agent's output rather than merely presenting it.

## Proposal

### 1. The two tiers, and what separates them

| | Mechanical (RFC-0008) | Agentic (this RFC) |
|---|---|---|
| Repairs | derived fields, back-links, locators | prose falsified by evidence |
| Authority | writes directly, under review-then-apply | **never writes** — emits a diff |
| Correctness | derivation is the proof | mechanical re-verification is the proof |
| Failure mode | writes a wrong computed value | proposes plausible wrong prose |
| Runs | in CI, in the hook | explicitly, or hook-suggested, never silently |

The tiers compose in one direction only: the mechanical tier's detect output **is** the agentic
tier's work list. An agent is never handed "look at this corpus and improve it." It is handed a
specific finding, the specific evidence paths that produced it, and one thing to resolve.

### 2. The one rule that makes this not-Q2

**An agent may delete or repoint a statement that evidence falsifies. It may never author a new
statement of *why*.**

This is a class distinction, checkable on the diff without judging the prose:

- **Permitted** — removing a literal the tree contradicts; repointing a prose reference to a moved
  path; deleting a sentence describing an approach the code demonstrably no longer takes; narrowing
  an over-broad scope claim to what the record actually covers.
- **Refused, by class** — adding rationale, adding consequences, adding an alternative-considered,
  or replacing removed reasoning with new reasoning. A hunk that *adds* a causal claim is rejected
  before review, regardless of how good it looks.

Removal-and-flagging is the honest operation. Where a rotted rationale needs replacing, the output
is a deletion plus a `needs-human` marker naming what is now missing — not a guess. This keeps the
tier strictly outside Q2: it never manufactures the thing Q2 doubts.

### 3. The verifier is the product; the agent is an input to it

Every proposed hunk goes through a mechanical gate before it reaches a human:

1. **Evidence check** — each hunk cites the paths/lines it read (RFC-0008's detect output already
   carries these). A hunk citing evidence outside its finding's evidence set is dropped.
2. **Class check** — §2's add-a-causal-claim rejection, applied to the diff.
3. **Re-run check** — apply the diff to a scratch tree and re-run `urzua check` + `fix --detect`. A
   hunk that does not reduce findings, or that introduces one, is dropped. This is the strongest
   gate and the reason the tier is defensible at all: the agent's claim to have fixed something is
   *tested*, not trusted.
4. **Idempotence check** — running the tier twice on the same corpus must produce an empty second
   diff. A tier that keeps finding new prose to improve is rewriting for its own sake.

What survives all four is presented as a normal reviewable diff. Nothing is applied. Every applied
hunk lands in the RFC-0006 revision log classified **`substantive`** — the inverse of RFC-0008,
correctly: prose changes meaning, and an agent-proposed one is exactly what a human should be able
to find later.

### 4. Invocation

- **Explicit command** — `/urzua-refresh` (or `urzua fix --agentic`), the `/security-review` shape:
  the user asks for it, it runs against the current diff or a named record, output is a review
  surface. This is the primary form.
- **Hook-suggested, never hook-applied** — RFC-0007's harness already runs checks the agent didn't
  choose to run. It may *surface* "three records reference code this branch moved; run
  `/urzua-refresh`." It may not run this tier automatically. A hook that silently rewrites prose is
  the thing this whole repo exists to prevent.
- **Distribution** — as harness skills/plugins in `ts/`, per the roadmap's Phase 2. The tier is
  harness-agnostic in design: the CLI does detection, gating, and re-verification; the harness
  supplies the model and the review surface.

### 5. Why this doesn't reopen the LLM-off-by-default position

RFC-0005 §1 chose mechanical-first for *computing embodiment*, and the roadmap's non-goals cede
generic drift and refuse semantic search. None of that changes. The agentic tier computes nothing
and decides nothing — every determination of *what is wrong* is still mechanical (§1), and every
determination of *whether the fix worked* is still mechanical (§3). The model occupies exactly one
slot: proposing English text for a defect a deterministic tool already localized. Off by default,
explicit to invoke, and removable without the rest of the product losing a capability.

## Open questions

- **Does the re-run gate actually have teeth for prose?** For a rotted literal, yes — a check can be
  written for it. For content-scope correctness (SPEC-0001's permanent ceiling), there may be **no
  mechanical signal that improves**, which would leave §3's strongest gate inert on the hardest
  class. Possible answer: the tier only accepts findings that *have* a mechanical signal, and the
  ceiling stays a human's job. That would be a real narrowing and it is not yet decided.
- **Which model tier, and does it matter?** Untested. Also whether the tier should run N independent
  proposals and keep only hunks that agree — cheap, and directly targets confident-wrong prose.
- **Does `needs-human` accumulate into the same unread pile** as §24/§26's warnings? Probable, and
  the same remedy (a declared expected-set) may be needed from day one.
- **Attribution**: RFC-0002 resolves an attestor identity for applies. What identity does an
  agent-proposed, human-applied hunk carry — the human's, the model's, or both? Both, probably;
  undesigned.
- Does this tier belong in the product at all, or is it a harness skill that ships separately and
  keeps the CLI's mechanical purity absolute? Leaning "CLI provides the gates, harness provides the
  model," which is what §4 says, but that is a positioning call as much as a technical one.

## Non-goals

- **Authoring records.** Backfill is a separate feature with its own trust question (Q2).
- **Autonomous application.** No path in this RFC writes to the tree without a human applying a diff.
- **Replacing review.** SPEC-0001 states review is a permanent required step for content-scope
  defects; this tier narrows the surface a reviewer must cover, and never claims to close it.
- **A general "improve the docs" agent.** Every run is scoped to a mechanical finding. No finding,
  no work list, no run.

## References

- RFC-0008 §2's explicit non-goal ("LLM-proposed repairs — a different, not-yet-proposed feature")
  and its detect output, which is this tier's input.
- RFC-0007 — the enforcement harness that may surface this tier but may not trigger it.
- RFC-0005 §1 — mechanical-first, unchanged by this proposal.
- SPEC-0001 — content-scope correctness as a permanent ceiling; the class this tier targets and the
  reason §3's gate may not reach all of it.
