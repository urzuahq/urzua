---
Stable-Id: 01M2YN6C9P33PH0A2RPFKATEBQ
Status: Accepted
Date: 2026-09-20
Author: beauwilliams
Deciders: beauwilliams
---
# 56 — How an untracked record-shaped file is surfaced

## Context

`check` governs the files git tracks. An untracked record-shaped file is invisible to it: not
examined, not counted, not mentioned. That is deliberate and tested
(`an_untracked_scratch_file_is_never_examined`) -- a scratch file in a working tree is not part of the
corpus, and governing it would make the tool hostile to drafting.

The consequence surfaced twice in one session. `urzua new` writes a record and leaves it untracked.
A reference to that record from an already-tracked file is then reported as dangling:

```text
pointer.resolution | docs/bugs/BUG-74-...md | Blocked-on: RFC-37 does not resolve to any discovered record
```

`error` severity, blocking, exit 1 -- about a record sitting on disk two directories away. Both times
the diagnosis reached for was a stale build, and both times it was wrong. The tool was correct and
said so in terms that pointed away from the cause.

The state is transient and entirely self-inflicted: `git add` resolves it. But it is produced by the
tool's own scaffolding command, on the path `MILE-103` is trying to make the *preferred* one, and the
report gives no hint that the missing record exists.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Govern untracked record-shaped files | The reference resolves; no transient window | Reverses a decided, tested behaviour; governs drafts and scratch files, making the tool hostile to work in progress |
| Leave it; `git add` is the answer | Nothing to build; semantics unchanged | The tool knows the record exists and says something misleading instead; cost paid twice in one session by the person who wrote the code |
| Name the untracked file in the diagnostic | Keeps the semantics exactly; converts a misleading message into an accurate one; additive | A rule now reads the working tree as well as the index |
| Have `urzua new` stage what it writes | Removes the window at the source | The command acquires a side effect on the index, which some workflows will not want |

## Decision

In the context of a tool that governs tracked files and a scaffolding command that creates untracked
ones, facing a diagnostic that is correct but points away from its cause, we decided that
**untracked-means-ungoverned stands unchanged, and the diagnostic names the untracked file when one
would resolve the reference**, to keep drafting free while making the report say what the tool
already knows, accepting that a reference-resolving rule must now look at the working tree in the
failure case and not only at the index.

Whether `urzua new` also stages what it writes is left to `MILE-103` as a workflow question rather
than decided here: it is about which path is faster, not about what is governed.

## Reversibility

Trivial. The change is one clause in one finding's message, behind the rule that already produces it.
Nothing about discovery, the record set, or `files_examined` moves. If the extra lookup proves
expensive or noisy it is deleted without consequence.

## Consequences

- `pointer.resolution`'s message gains a case: the reference does not resolve, *and* an untracked
  record-shaped file that would resolve it is present -- say where it is and that staging it fixes
  this.
- A reference-resolving rule reads the working tree on the failure path. It must not change the
  verdict: the finding is still an error, because the record is still not in the corpus. Only the
  explanation improves.
- The same treatment is available to any rule reporting a missing record and is not applied
  pre-emptively; one message is the evidence we have.
- `MILE-103` gains auto-staging as a candidate, with this session as the evidence that the
  hand-authoring path and the `urzua new` path have different failure modes.

## References

- `MILE-103`, on `urzua new` being the faster path.
- `BUG-24`, on explicit argv paths and untracked records.
- `ADR-55`, on the tool saying what it knows.
