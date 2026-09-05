# 0003 — Stable record identifiers, separate from display numbers

> Status: Accepted
> Embodiment: Not started
> Date: 2026-07-29
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —

## Context

The obvious ADR/RFC/Spec convention uses sequential numbers (`0001`, `0002`),
and sequential numbers are the classic thing that breaks under concurrency. With twenty agent
branches open at once, each drafting a record and each independently picking "the next free
number," they all pick the same one. The outcomes are a merge conflict on the *filename*, or —
worse — two different decisions silently sharing an identifier, which is unrecoverable data
corruption in a system whose entire value is being a trustworthy record.

The cost isn't only collisions. It's that agents and humans burn context on **bookkeeping**:
checking which number is free, renumbering after a rebase, and repairing cross-references that
pointed at the old number. That is precisely the tax the product exists to remove, and it appears at
the exact moment the product's premise (many agents working in parallel) comes true.

This is the same problem distributed databases solved for sharded IDs: you avoid collisions either
by central coordination or by making independent generation safe. **Central coordination is exactly
what's unavailable across twenty concurrent branches**, so independent generation is the only
workable answer.

Sequential numbers nonetheless have real value that must be preserved: humans say "ADR-42," not a
26-character identifier. Chronological ordering is legible at a glance, and any existing corpus
adopting this tool is already sequentially numbered and cannot be invalidated.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| **Sequential only** (status quo) | Human-friendly; matches existing corpora | Collides under concurrency; renumbering breaks cross-references; the failure mode is silent |
| **UUIDv4 only** | Collision-free without coordination | Unordered; unreadable; unspeakable; destroys chronological legibility |
| **ULID/UUIDv7 only** | Collision-free, time-ordered, lexicographically sortable | Still unreadable and unspeakable in conversation |
| **Stable ID + display number** *(chosen)* | Collision-free authoring, human-friendly reading, cross-references never break | Two identifiers per record; requires a merge-time assignment step and tooling discipline |
| **Branch-prefixed sequentials** (`feat-x-001`) | Simple; no new concepts | Leaks branch names into permanent record IDs; still collides after merge; ugly |

## Decision

In the context of governance records authored concurrently by many agents and humans, facing
guaranteed identifier collisions and the silent-corruption risk of two decisions sharing a number,
we decided that **authors never choose an identifier**: every record is assigned a time-ordered,
lexicographically sortable stable ID at creation, with a human-facing sequential display number
assigned deterministically at merge time, and all cross-references resolved against the stable ID.
We accept carrying two identifiers per record and the tooling discipline that requires.

Specifics:

1. **Stable ID** — assigned by `urzua new`, generated independently with no coordination, time-ordered
   so creation order is preserved without a central authority. Immutable for the life of the record;
   never reused, never reassigned. This is the identity.
2. **Display number** — the familiar `0042`, assigned deterministically when the record lands on the
   trunk, ordered by the stable ID's timestamp so numbering matches creation order. Presentation
   only. It may change; nothing may depend on it.
3. **Cross-references** (`supersedes`, `superseded_by`, `implements`, `derives_from`, `related`)
   resolve against the **stable ID**, always. This is the property that makes renumbering safe:
   assigning a display number can never break a link.
4. **Filenames** carry the display number for browsability
   (`0042-some-decision.md`), with the stable ID in the record's frontmatter as the authority. A
   filename/ID mismatch is a lint error — a check that already exists in the source implementations.
5. **Back-pointers in code** (`Implements: ...`) accept either form. Display numbers are what humans
   type; the resolver maps them to stable IDs, and the linter flags any display number that has
   drifted from the record it names.
6. **Existing corpora migrate** by retaining their current sequential numbers as display numbers and
   backfilling stable IDs from git history timestamps. No renumbering, no invalidated references —
   a hard requirement, since breaking three real corpora to fix a theoretical collision would be
   self-defeating.

Exact ID encoding (ULID vs. UUIDv7 vs. a scheme with a shorter human-typable prefix) is an
implementation detail deferred to the v0 spec, not decided here. The load-bearing commitment is the
*separation* of identity from display.

## Reversibility

**Poorly reversible once records exist — this is the least reversible decision of the first three.**
Identifiers are referenced by every other record, by code back-pointers, and potentially by external
systems. Changing the scheme later means rewriting every cross-reference in every corpus, which is
exactly the class of bulk-rewrite operation that has caused documented data-loss incidents
elsewhere.

The mitigation is that the *encoding* stays deferred and swappable, while the *separation of
identity from display* — the part that's hard to undo — is the part with the strongest argument
behind it.

## Consequences

- `urzua new` becomes mandatory in practice. Hand-authoring a record file gets you no stable ID, and
  the linter will reject it. This is a real ergonomic cost and a real behaviour change for humans
  who currently just copy a template.
- Merge-time display-number assignment needs somewhere to run — a git hook, a CI step, or a merge
  queue — and must be idempotent and deterministic under concurrent merges. **This is the hardest
  part to implement correctly and the most likely source of v0 bugs.**
- Two identifiers per record raises the "which one do I use?" question for every human and agent.
  Documentation and error messages must be unambiguous.
- Cross-reference resolution becomes an indirection rather than string equality, adding a resolver
  to the validator core.
- Migration tooling for existing corpora is required before the tool is usable on any codebase with
  records already in place — so it is v0 scope, not follow-up.
- Directly complements the field-level merge semantics of concurrent-edit handling: that problem
  handles two agents editing the *same* record, this handles two agents creating *different*
  records simultaneously.

## References

- `docs/rfc/0001-unified-record-schema-core-and-profiles.md` — core field `id`, "stable, unique, never reused"
