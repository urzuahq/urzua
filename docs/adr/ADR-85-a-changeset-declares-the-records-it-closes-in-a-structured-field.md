---
Stable-Id: 01M6N6RR4HNM6QKZK73FDHT6GQ
Status: Proposed
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: —
---
# 85 — A changeset declares the records it closes in a structured field

## Context

`claim.status-agreement` finds a claim in a release fragment by reading prose: a closing verb followed by
a reference (`Fixes BUG-60`). Two defects follow from that, and both are in this repository's history.

- A claim written without the verb is invisible. `BUG-66`: five bugs were named as `(BUG-60)` in a
  fragment while still `Open`, and the rule examined 2 claim inputs and reported agreement. It was caught
  in review, in about 70 minutes, before the branch merged; no release ever carried it.
- Reading prose to decide intent does not work. `ADR-81` proposed flagging a bare reference to an open
  record. Measured over this repository's fragment history it flagged 19 references and none was a claim
  (13 sat in a sentence saying the record was filed, left open or not fixed), and at HEAD it produced 6
  false findings under a gate that fails on warnings. It was rejected for that reason.

The verb vocabulary is also a standing problem of its own (`BUG-59`, `BUG-65`, `MILE-98`): the engine
should not own English.

The fix that avoids both is to stop parsing prose for the one thing that must be exact. A fragment that
closes a record says so in a field, as data, and the engine checks the data (`ADR-64`: structured in,
structured out). Prose stays prose and is never read for claims.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Keep the verb reading only | Nothing to change | `BUG-66` stays possible; the vocabulary stays a hardcoded English list |
| B. Flag a bare reference to an open record (`ADR-81`) | No new field | Rejected: 0 real hits in 19, 6 false findings at HEAD |
| C. A declared frontmatter field listing the records the change closes, checked against their status (chosen) | Exact; no prose read; precision 100 percent by construction; the engine names no field until config does | Authors must write it; a claim not declared is still not seen (below) |
| D. Put the claim in the commit message or PR body | Close to the merge | Not a file a path reads; not in the tree; not available to `check` without a forge |

## Decision

In the context of a release fragment whose claim to close a record is found by parsing prose, facing a
silent miss (`BUG-66`) and a prose-based remedy that measured at 0 real hits in 19, we decided to **let a
fragment declare the records it closes in a structured frontmatter field, and check that each listed
record is in a declared closed status by the time the PR merges**, to achieve a claim that is exact and
verifiable without reading a sentence, accepting that the author has to write the field and that a claim
left undeclared is not detected by this ADR alone.

- **The field is declared, not known.** Config names the field (this repository: `closes`) on the existing
  claim-reading rule, as an option beside `claim_paths`. The engine knows no field name. Declaring none
  changes nothing; the rule is opt-in like every other (`ADR-53`).
- **Its value is a list of record ids,** on one line: `closes: [BUG-60, BUG-61]`, or a single id. Each id
  must resolve; one that resolves to nothing is reported, never skipped. An empty list is a valid
  declaration that the change closes nothing.
- **Each listed record must be in a declared closed status** when the PR merges. Closed means the type's
  declared success or failure terminals (`is_finished`, `BUG-178`), not the fallback `closed_statuses`
  list. A listed record that is still open is an error: the fragment claims a closure the record does not
  show. On a run with `--base`, only fragments the diff adds or changes are examined, so a listed record
  that was already closed at the merge base is not an error, and a historical fragment is not re-judged
  when its record is later reopened. Without `--base` every fragment on disk is examined, as today.
- **Prose is never parsed for claims by this feature.** A reference in the body of a fragment is a
  citation. The field is the only declared claim.
- **The existing closing-verb reading becomes a deprecated convenience.** It keeps working for a
  repository that has not declared the field, unchanged. For one that has, it stays on until a later
  decision removes it, and the report discloses how many verb claims it still found so a repository can see
  when it is safe to stop. Removing it is not decided here and is a separate ADR.
- **The suggested action** for an open listed record is `edit-field` on the record's `Status`, the fix
  being to close the record or to take it out of the list (`ADR-73`); the finding does not choose.

**The release tooling tolerates the extra key (tested).** The release workflow pins knope 0.23.0
(checksummed in `prepare-release.yml`). Run in a scratch repository with this repository's
`PrepareRelease` step, `knope prepare-release --dry-run` over fragments whose frontmatter carries the
package bump plus an extra `closes` key produced the same version bump and changelog as the same fragment
without it. Results:

| Frontmatter | knope 0.23.0 |
|---|---|
| `closes: [BUG-60, BUG-61]` (one-line list) | accepted; bump and changelog unchanged |
| `closes: BUG-62` (scalar), `closes: "BUG-60, BUG-61"`, `closes: []` | accepted |
| `closes:` followed by a `- BUG-62` block list | rejected: `changesets::could_not_read_changeset`, invalid front matter |
| a one-line list continued over two lines | rejected, same error |

So the field is a **single line**, and a block list is not available. The key is ignored by knope and does
not reach the changelog. This is a property of one pinned version of one tool, so the implementing change
must keep a test that runs the pinned knope over a fragment carrying the field, and a knope upgrade
re-runs it. Whether an unknown key stays harmless in a workspace of several named packages was not
tested; this repository has one.

## Reversibility

Easy to moderate. The option is additive and opt-in, and the closing-verb reading is untouched, so
removing the feature is a config edit. What is harder to take back is the field's name and single-line form
once fragments in the wild carry them, and a future removal of the verb reading, which is a separate
decision.

## Consequences

- `SPEC-3` gains the claim-field option and `SPEC-2` the findings (a listed id that resolves to nothing; a
  listed record that is open), each with a revision row. The CLI reads fragment frontmatter as data and
  ignores every key but the declared one, including knope's own bump keys.
- This repository declares `closes` and the changeset convention in `CONTRIBUTING.md` and the changelog
  guidance gains one sentence.
- `BUG-66` closes once this is built, with a regression test that a fragment listing a still-open record in
  `closes` is an error. That is the honest limit: the incident's own shape, a fragment naming a record
  with **no** `closes` entry, is still not detected. What changes is that a declared claim is exact and a
  citation is never mistaken for one.
- The records a release closes are not in the user-facing changelog, because knope ignores the key. A
  release note that wants them still writes them in prose.
- A fragment that closes nothing and declares nothing is indistinguishable from one that forgot to.

## Open questions

1. **Should the field be required on every fragment** (an empty list meaning "closes nothing")? That is the
   only way to detect the `BUG-66` omission, at the cost of an edit on every fragment. This ADR leaves it
   optional and does not decide the requirement.
2. **Is a closed status "by the time the PR merges" checkable without `--base`?** On a branch run the
   record is read at the PR head; a record closed in a later commit of the same PR passes, which is the
   intent. A merge-queue run reads the merged tree. Nothing further is needed unless a reviewer disagrees.
3. **Where does the field live if knope is replaced?** The form is portable frontmatter, but a different
   tool may reject extra keys. The test above is the guard.

## References

- `ADR-81` -- the prose-based rule this replaces, rejected on measurement; `RFC-36` -- the proposal behind it.
- `BUG-66` -- the incident this closes; `BUG-36` -- the defect `claim.status-agreement` was built for.
- `BUG-59`, `BUG-65`, `MILE-98` -- the verb vocabulary; `BUG-178` -- what closed means.
- `ADR-64` -- structured in, structured out; `ADR-53` -- declared, opt-in; `ADR-73` -- suggested actions.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** maintainer decision of 2026-11-05 to reject `ADR-81` and make a changeset's claim a structured field; the maintainer approves before it is Accepted. | **substantive** |
