# 0013 — A vacated identifier is a record, not a gap

> Status: Draft
> Date: 2026-08-18
> Author: (session author)
> Supersedes / Superseded-by: —

## Summary

A corpus that allocates display numbers sequentially will eventually vacate one — a branch renumbers
after a collision, a record is withdrawn, a draft is abandoned. Today that leaves a hole in the
sequence, and every tool can offer only two answers, both wrong: warn forever about a state nobody
can fix, or suppress the warning and lose the fact. Propose a third: **the vacated number keeps a
record.** A tombstone carries the number, a status meaning "no decision lives here", and one line
saying where the decision went. The sequence stays contiguous, the gap check needs no exception
list, and a reader following an old reference finds an answer instead of nothing. Propose also the
structural consequence — a record with no decision cannot be required to state one — and the
prevention half, which is ADR-0003 doing the job it was written for.

## Motivation

Consider a corpus where four numbers are missing, and the check reporting them
hedges between "reserved by a branch" and "skipped by mistake". It is neither: all four existed, and
the commit that vacated two names the cause in its subject — *renumber to avoid collision with
main*. A branch took `highest + 1`, `main` took the same numbers first, the branch renamed itself.

That produces three problems at once, and only the first is obvious:

1. **A warning that can never be actioned.** Backfilling the number would put a new decision between
   two old ones; renumbering to close the gap would break every `Implements:` back-pointer in code
   and every commit that names a number. The correct action is permanently "do nothing", and a
   check whose finding is always "no action" trains people to skim the whole warning stream.
2. **A reference that resolves to nothing.** An RFC in that corpus still reserves "ADR-0039 through
   ADR-0043" — a number that no longer exists. The renumbering fixed filenames and missed the prose
   pointing at them, and nothing checks RFC bodies for dangling record references.
3. **A question with no answer in the corpus.** Someone reading that RFC, or an old commit, or a
   code comment, will look for the record and find absence. Absence is indistinguishable from "never
   existed", "deleted deliberately", and "you are looking in the wrong place".

The instinct to fix this with a config allowlist (`allowedGaps: [12, 13]`) is understandable and is
the weakest option: it silences the check without recording *why*, grows monotonically, and is
invisible to the person actually asking the question, who is reading the corpus rather than the
linter's configuration.

## Proposal

### 1. A vacated number keeps a tombstone record

A tombstone is an ordinary record at the vacated number, with a status meaning **no decision lives
here** and a body of one or two sentences: what happened, and where the decision went if it went
anywhere.

```markdown
# 0012 — Vacated: renumbered to avoid a collision

- **Status:** Withdrawn
- **Date:** 2026-07-16
- **Vacated-to:** ADR-0015

This number was claimed by a branch that renumbered to avoid a collision with an
already-merged record. The decision itself is ADR-0015. Nothing was decided here.
```

The properties that make this better than the alternatives are not aesthetic:

- **The sequence is contiguous**, so the numbering check needs no allowlist, no exception logic, and
  no per-corpus configuration. The warning does not fire because there is nothing to report.
- **It answers at the point of the question.** The reader following the stale reference lands on the
  tombstone. A configuration file cannot do that.
- **It is bounded.** One file per vacancy, written once, never revisited — where an allowlist is a
  list someone must keep evaluating.
- **It uses the schema that already exists.** `Withdrawn` and `Rejected` are already statuses that
  mean "not a live decision".

`Vacated-to` is a new optional core field, and is deliberately not `superseded_by`: supersession
means a decision replaced another decision, and here there was never a decision to replace.

### 2. A record with no decision cannot be required to state one

Structural checks that assume a decision exist — a Y-statement, at least one rejected alternative, a
consequences section — must become **status-aware**, the way role requirements already are (a
`Deciders` value is required only once a record is `Accepted`).

This is the part most likely to be got wrong in implementation, because the checks predate any
status that means "no decision here", so they are written as unconditional. A tombstone that has to
invent a Y-statement to satisfy a linter is a record that lies in order to pass, which is worse than
the gap it replaced.

### 3. Prevention is ADR-0003, and this RFC does not replace it

Tombstones are cleanup. They do not stop the next collision, and nothing in this proposal should be
read as making sequential allocation safe.

`highest + 1` is a read of local state with no reservation: two branches open at once can take the
same number, and the collision is decided at merge, after both have written. ADR-0003 already chose
the fix — a stable identifier assigned at creation, a display number assigned at merge — so a
concurrent claim can never force a rename. §32 is what not having that costs in practice: two
renumberings, four permanent vacancies, one dangling cross-reference.

The relationship is worth stating precisely, because it determines what to build first: **ADR-0003
prevents new vacancies; tombstones handle the ones that already exist and the ones that arise for
reasons ADR-0003 cannot prevent** — a withdrawn decision, an abandoned draft. Both are needed, and
neither substitutes for the other.

### 4. A number has three fates, and the gap check is a work queue

Tracing a corpus over time shows a gap is not one condition. A number can be:

| State | Gap check | Correct response |
|---|---|---|
| Held by an open proposal | warns | none — it closes when that proposal merges |
| Merged | silent | none |
| **Vacated** — renumbered away, or claimed by a proposal that was closed unmerged | **warns forever** | tombstone |
| **Never allocated** — the sequence skipped it; no decision was ever assigned it | **warns forever** | tombstone, pointing nowhere |

The fourth row was added after finding one in the field: a number
absent from every ref in a repository's entire history, never added, removed or renamed, referenced
by nothing except the warning itself. It differs from a vacancy in one way that matters to what the
record can say. A tombstone for a vacated number names the successor the decision moved to; a
never-allocated number has no successor, so its record can only report an absence. It is also the
only fate with no resolving event of any kind — a reservation fills on merge, a vacancy is explained
once — so it is simultaneously the case that most needs a record and the one that gets the least
from having one.

The third and fourth rows are what make a warning permanent, and the third is invisible from the
corpus alone:
a number held by an open branch and a number held by a branch that was abandoned look identical.
Distinguishing them requires asking the forge whether the holding proposal is still open.

This reframes what the check is for. It is not a defect detector — a gap is not a defect — it is a
**queue of unresolved number claims**, where each entry is waiting on either a merge or a tombstone.
Stated that way the warning is actionable in every case it fires, which is the property it lacked.

Two consequences for the tooling:

- **Resolving the holding proposal is what makes the queue self-classifying.** A check that can ask
  "is the branch claiming 0043 still open" can label each entry rather than leaving the reader to.
  That is a forge query, so it belongs behind the same boundary as any other network-dependent
  check: available, never required, and never the difference between a clean run and a failing one.
- **Closing a proposal that drafted a record is a vacating event**, exactly like a renumbering, and
  should produce a tombstone at that moment. It is the one vacancy cause that is announced — the
  forge knows the proposal closed — so it is the one where a tombstone can be prompted for rather
  than discovered later by a warning nobody can action.

### 5. Migration is per corpus and cheap

Adopting this in an existing corpus is: write one tombstone per current gap, make the structural
checks status-aware, and delete any gap allowlist. It requires no renumbering, touches no
back-pointer, and cannot break a reference — every change is additive.

## Open questions

- **Should a check declare which statuses it applies to?** Making the structural checks status-aware
  was done in the field by special-casing one status inside each check. That works and does not
  scale: a fifth check added later silently applies to every status, including the ones that cannot
  satisfy it, which is RFC-0011's width mismatch appearing on the status axis rather than the path
  axis. Declaring each check's applicable statuses where the check is registered would make the
  scope inspectable, and would make "which checks does a tombstone have to pass" answerable without
  reading every check body. Not proposed here because the right place to declare it depends on how
  checks end up registered.
- **Does a tombstone need its own status, or is `Withdrawn` enough?** `Withdrawn` currently means "a
  proposal that was pulled", which is close but not identical to "this number was never used". A
  distinct `Vacated` is clearer and adds an enum value every consumer must learn.
- **Should the tombstone be generated?** The renumbering that vacates a number is a mechanical
  event, so the tooling could write the tombstone as part of the rename. Attractive, and it means
  the tombstone's claim is only as accurate as the automation's understanding of why the rename
  happened.
- **Do tombstones belong in the index?** Including them keeps the index a faithful map of the number
  space; excluding them keeps it a list of decisions. Leaning include-but-mark, undecided.
- **How should a closed proposal's tombstone read** when the proposal was closed because the idea
  was rejected, rather than merely abandoned? That is arguably a `Rejected` decision with real
  content, not a tombstone — the number was used, and the outcome was "no". Distinguishing
  "abandoned before deciding" from "decided against" matters, and the corpus evidence so far
  contains only the former.
- **What about a corpus that has already deleted the record entirely**, with no memory of what was
  there? The tombstone can only say "vacated, cause unknown", which is still more than a gap says.

## Non-goals

- **Renumbering to close gaps.** Rejected outright: it breaks code back-pointers, commit messages
  and PR titles, and §32 records two occasions where doing it created the problem this RFC solves.
- **A configuration allowlist of expected gaps.** See Motivation — it silences without recording, is
  unbounded, and is invisible to the reader who has the question.
- **Preventing collisions.** ADR-0003's territory (§3 above).
- **Checking prose references inside RFC bodies.** A real gap in the corpus above, but a different
  feature — this RFC only ensures the reference has something to resolve *to*.

## References

- ADR-0003 — stable identifiers and merge-time display numbers, the prevention half.
- RFC-0001 §1a — declared profiles, which is where a `Vacated` status and the `Vacated-to` field
  would be declared rather than hardcoded.
