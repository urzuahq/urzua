---
Stable-Id: 01M28K3SWK8FGBR1R4N3GJH15S
Status: Draft
Date: 2026-09-11
Author: '@beauwilliams'
---
# 27 — Urzua's position in agentic change management: one engine, one philosophy

## Summary

The engine's purpose has never been written down as a record. It's inferable from RFC-7, RFC-9 and
RFC-12 read together, but each of those reads as an unrelated proposal on its own, and nothing
states the thesis they share. Propose stating it: **Urzua is the verifiable specification layer for
agent-authored change — it governs the record a change is supposed to trace back to, never the
change itself.** Propose also the boundary that follows, which matters more than the slogan: the
engine implements that one philosophy and refuses capability that merely sits *near* records.
Everything of the shape "a tool built around the engine" — dashboards, collectors, org rollups,
tracker sync, anything hosted — is a separate **ecosystem** axis, and this RFC's second proposal is
that the axis exists, is deferred, and is not the engine's scope. The engine must stay complete and
useful with none of it built.

## Motivation

**The immediate trigger is a reader who couldn't tell what this is for.** Asked directly: *"It
looks like this could be used for a lot of different things, do you have any particular purpose(s)
in mind? It looks like decision logging and task tracking are part of it but there are large
sections that don't seem relevant to that."* That is a fair reading of the README as it stood. The
sections that looked irrelevant — Embodiment/`Realized-by`, `fix`, `graph`, the always-JSON output
contract — are the actual thesis, and decision logging is the *example*, not the purpose.

The README now answers it in a `Where this is going` section. That is the inversion this project
exists to prevent: a position asserted in prose with no record behind it, which is precisely the
failure mode the corpus is meant to catch. The README should cite this record, not substitute for
it.

**The category has now been named externally, which sharpens rather than threatens the position.**
CodeRabbit's *What Is Agentic Change Management?* (2026-08-12) defines governance for changes
authored by humans and agents, scoped from proposal through post-merge, and argues that a reviewer
working only from the generated implementation is **"structurally circular"** — independent
verification needs *"standards defined outside the generation process,"* such as acceptance
criteria, architectural rules, or migration requirements. That argument is correct and is the
strongest external statement of this project's own premise to date.

It also stops one step short, and the step it stops at is the whole product. Their standards layer
is configuration authored alongside the review tool. **A standard defined outside the generation
process is only worth checking against if the standard is itself verifiable**: accepted by a named
human, carrying evidence of what realizes it, and re-checked mechanically as the code moves
underneath it. An unverifiable external standard relocates the circularity rather than breaking it
— which is exactly what happens when the same agent writes both the decision record and the code
that claims to implement it.

**Internally, the three proposals that carry this thesis have never been tied together.** RFC-7
argues instructions are not enforcement; RFC-12 proposes a merge gate on unaccepted decisions;
RFC-9 proposes an agentic repair tier behind a mechanical verifier. Each was drafted from its own
local evidence. Read together they are one argument about making a record trustworthy enough to be
worth checking against — and no record says so.

## Proposal

### 1. The position, stated

> Urzua is a record engine for the specification layer of agent-authored change. It governs whether
> an accepted decision exists, whether it existed before the code arrived, and whether its claim
> about the code is still true. It does not review changes.

The division of labour against a diff-level reviewer is deliberate and non-competitive: **an AI
reviewer checks the diff; Urzua checks the record the diff is supposed to trace back to.** Both are
agentic change management. They operate on different objects and neither subsumes the other.

### 2. The boundary, stated as a test

For any proposed engine work, ask: **does this make a record more verifiable, or does it add a
capability around records?** The first is engine scope. The second is ecosystem, however useful.

In-scope for the engine, by that test: record schema and its declaration; validation; relationship
resolution; evidence (`Realized-by`) and drift; mechanical repair of derived fields; the enforcement
surface that makes any of the above fire independent of an agent's compliance (RFC-7, RFC-12); the
agentic repair tier gated behind mechanical re-verification (RFC-9); identity and acceptance
strength (MILE-18, MILE-19).

Permanently out, restating SPEC-1's boundary as a boundary rather than a stage: diff review, risk
scoring, reviewer routing, a model call anywhere in the mechanical tier, telemetry, and any hosted
service the engine depends on to function.

### 3. The ecosystem is a separate axis, and it is deferred

Records on that axis already exist and are not repudiated by this RFC — they are reclassified as
*not engine scope*: the forge app (RFC-14, ADR-25, MILE-30), the generated dashboard (ADR-37),
governance-health analytics and org rollup (MILE-24), tracker and compliance-export integrations
(MILE-32), and retroactive history reconstruction as a service (MILE-53).

The constraint this RFC asks to be adopted: **the engine must remain complete and useful with none
of the ecosystem built.** Any ecosystem tool consumes the engine's JSON contract from outside; none
of them may become a dependency of `check`, `fix`, or the record schema. A capability that only
works with a service running is an ecosystem capability, not an engine one, whatever it does.

### 4. Vocabulary

"Metaharness" describes the architecture accurately and should stay internal. It is not a term
anyone searches for, and it invites exactly the orchestration-shaped scope creep §2 rules out.
Outward-facing language describes the object and the check, not the layer.

### 5. What this position does not yet earn

Both gaps are named in the README rather than left to be found, and both are load-bearing on
whether this position is defensible:

- **Acceptance is a convention, not a mechanism.** A record's `Status` moving to `Accepted` is today
  an instruction in `AGENTS.md` asking agents to leave it alone. That is instruction-shaped — RFC-7's
  own argument, turned on this project (BUG-16, RFC-25, MILE-18, MILE-19).
- **One corpus.** Every genericity claim is validated against the corpus that shaped the engine
  (MILE-51). BUG-8 is what that gap looks like when it reaches the README.

## Open questions

- **Does RFC-12's gate depend on MILE-18/MILE-19 shipping first?** A merge gate keyed on a `Status`
  that any agent can set is theatre, and worse than nothing if it manufactures confidence. Leaning
  strongly toward: the acceptance mechanism is a prerequisite for the gate, not a parallel track.
  Not decided here.
- **Is a read-only MCP server (MILE-29, MILE-47) engine or ecosystem?** It carries no new capability
  — it is a transport for `check`/`explain`/`graph` — which argues engine. It also only matters with
  an agent runtime attached, which argues ecosystem. The §2 test does not cleanly resolve it.
- **Does the ecosystem axis need a record type, or is a `Track` sufficient?** Milestones already
  carry `Track`; nothing yet marks a record as out-of-engine-scope.
- **Is MILE-51 a precondition for stating this position publicly**, or is naming the gap (§5) enough?
  The README currently takes the second option.
- **Does decision-record theatre get worse under a merge gate than without one?** A gate that makes
  "decide first" mandatory in an agent-speed workflow may produce rubber-stamped records, which are
  worse than no records — drift detection would then verify against confident invented reasoning
  (RFC-9's own stated Q2 risk, arriving through a different door). Undesigned.

## Non-goals

- **Deciding the ecosystem's contents or roadmap.** This RFC proposes only that the axis exists and
  is out of engine scope. What gets built there, when, and whether any of it is hosted or commercial
  is deliberately untouched.
- **A competitive or marketing document.** The comparison in §1 exists to draw a boundary, not to
  claim superiority, and this RFC commits to no interoperation with any specific vendor or product.
- **Re-opening SPEC-1's success criteria.** Two real codebases and a deleted hand-written linter
  remain the bar; a named category does not change it.
- **Renaming, rescoping, or adding any command.** No code change follows from accepting this RFC.
- **Retroactively editing RFC-7, RFC-9, or RFC-12** to reference this one. They stand as written;
  this record is the tie-together, per the amendment-not-silent-edit model.

## References

- RFC-7 — instructions are not enforcement; the argument §5 turns back on this project.
- RFC-9 — the agentic repair tier, and the Q2 risk the last open question restates.
- RFC-12 — the decision-before-implementation gate, whose prerequisite is the first open question.
- SPEC-1 — the success criteria and the mechanical/offline/deterministic boundary §2 restates.
- RFC-14, ADR-25, ADR-37 — ecosystem-axis records reclassified by §3, not repudiated.
- BUG-8 — what an overstated README claim looks like when the corpus can't back it.
- BUG-16, RFC-25 — identity as attribution, not attestation; the acceptance gap in §5.
- CodeRabbit, *What Is Agentic Change Management?*, 2026-08-12 —
  <https://www.coderabbit.ai/guides/what-is-agentic-change-management>. Source of the
  "structurally circular" framing and the four-capability definition (validate, prioritize, explain,
  maintain) this RFC positions against.
- *The Specification as Quality Gate* (preprint, cited by the above) — AI review without an external
  specification is structurally circular; directional evidence on a planted-bug corpus, not
  conclusive, and cited here at that strength.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-11 | Initial RFC, `Status: Draft`. Filed after the README's `Where this is going` section asserted this position with no record behind it. | **structural** |
