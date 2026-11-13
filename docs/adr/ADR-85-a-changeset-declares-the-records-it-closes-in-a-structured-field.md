---
Stable-Id: 01M6N6RR4HNM6QKZK73FDHT6GQ
Status: Accepted
Accepted-on: 2026-11-05
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: Verified
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
  claim-reading rule `claim.status-agreement`, as options beside `claim_paths` and `closed_statuses`. The engine knows no field name, record type or
  status: the listed ids are checked against whatever types the repository declares, and "closed" is each
  type's declared terminal statuses. Declaring none changes nothing; the rule is opt-in like every other
  (`ADR-53`).
- **Strictness is a config option.** `strict: true|false`, default `false`: with `false`, the field is
  checked when present. With `true`, a fragment in a declared claim path that lacks the field is a finding,
  so a fragment that names a record without declaring whether it closes it is caught; `closes: []` is the
  declaration that it closes nothing. This repository enables `strict`.
- **Its value is a list of record ids,** on one line: `closes: [BUG-60, BUG-61]`, or a single id. An empty
  list is a valid declaration that the change closes nothing.
- **Frontmatter parsing is new.** `read_claim_files` (`discovery.rs`) returns raw file text and
  `claim_status_agreement` reads only prose lines; nothing parses fragment frontmatter today. The
  implementing change adds a deliberately small parser for the declared key only: the file begins with a
  `---` block, and the key's value is a one-line flow list `[a, b]`, a scalar, or `[]`. Other keys are
  ignored. A block list (`closes:` followed by `- ` lines), a list continued over several lines, or a value
  that is not a list or an id is a new finding, `claim.field-malformed`, naming the file and line; it is never
  guessed at. A file in a claim path with no frontmatter block is not a fragment: it is counted out of scope
  and disclosed, never given a finding, which is how a non-fragment such as a `README.md` in the directory is
  excluded without the engine knowing its name. (Amended 2026-11-05: see the Amendment below for a file the release tool cannot read.)
- **An id that resolves to nothing is a new, reported finding.** Today a claimed reference that matches no
  record is eligible and skipped silently (`Outcome::Absent` in `rules/claim.rs`). With the field declared,
  a listed id that resolves to no record is an error (`claim.id-unresolved`), gated by the same opt-in
  config. It applies only to ids from the structured field. The verb-found claims keep today's behaviour
  (skipped and counted), so a repository that has not declared the field sees no change.
- **Each listed record must be in a declared closed status** when the PR merges. Closed means the type's
  declared success or failure terminals (`is_finished`, `BUG-178`), not the fallback `closed_statuses`
  list. A listed record that is still open is an error: the fragment claims a closure the record does not
  show. No diff scoping is added: `read_claim_files` reads every `.md` in the claim paths and `check`
  applies no diff filter to them, with or without `--base`, and this ADR keeps that. A historical fragment
  whose record is later reopened is therefore judged again; that is accepted as a rare, correct report that a
  claim no longer holds.
- **Prose is never parsed for claims by this feature.** A reference in the body of a fragment is a
  citation. The field is the only declared claim.
- **The existing closing-verb reading becomes a deprecated convenience.** It keeps working for a
  repository that has not declared the field, unchanged. For one that has, it stays on until a later
  decision removes it, and the report discloses how many verb claims it still found so a repository can see
  when it is safe to stop. Removing it is not decided here and is a separate ADR.
- **The suggested action** for an open listed record is `edit-field` on the record's `Status`, the fix
  being to close the record or to take it out of the list (`ADR-73`); the finding does not choose.

**The release tooling tolerates the extra key (tested).** The release workflow pins knope 0.23.0
(checksummed in `prepare-release.yml`). The test was run in a scratch repository containing this
repository's `knope.toml` with its `PrepareRelease` step: `knope prepare-release --dry-run` over fragments whose frontmatter carries the
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
re-runs it as a CI test, not a one-off. Whether an unknown key stays harmless in a workspace of several
named packages was not tested; this repository has one. That is an untested risk for any adopter with a
multi-package workspace, who should run the same test before declaring the field.

## Reversibility

Easy to moderate. The option is additive and opt-in, and the closing-verb reading is untouched, so
removing the feature is a config edit. What is harder to take back is the field's name and single-line form
once fragments in the wild carry them, and a future removal of the verb reading, which is a separate
decision.

## Consequences

- `SPEC-3` gains the claim-field options on `claim.status-agreement` and `SPEC-2` the findings (a listed id
  that resolves to nothing; a listed record that is open; a malformed value; a fragment without the field
  under `strict`), each with a revision row. The CLI reads fragment frontmatter as data and ignores every key
  but the declared one, including knope's own bump keys.
- This repository declares `closes` and the changeset convention in `CONTRIBUTING.md` and the changelog
  guidance gains one sentence.
- `BUG-66` closes once this is built, with a regression test that a fragment listing a still-open record in
  `closes` is an error. With `strict` on, as here, the incident's own shape, a fragment that
  names a record and declares nothing, is caught because the fragment lacks the field; with it off, that
  shape is still not detected. Either way a declared claim is exact and a citation is never mistaken for one.
- The records a release closes are not in the user-facing changelog, because knope ignores the key. A
  release note that wants them still writes them in prose.
- Under `strict`, every fragment carries the field (`closes: []` when it closes nothing), an edit on each
  one. The fragments already unreleased in this repository lack it, so the implementing change that enables
  `strict` migrates every existing fragment in the same change. There is no diff scoping to lean on, and
  none is added; the result is checkable with a plain `urzua check`, without `--base`.

## Resolved questions

1. Existing fragments under `strict` are migrated in the implementing change; no diff scoping is added.
2. "Closed by the time the PR merges" is checkable without `--base`: a branch run reads the record at the PR
   head, so a record closed in a later commit of the same PR passes, and a merge-queue run reads the merged
   tree.

## Amendment (2026-11-05): a fragment the release tool cannot read is malformed under `strict`

*Status of this amendment: accepted by the maintainer on 2026-11-05. `Status` stays `Accepted`.*

The Decision counts a file in a claim path with no frontmatter block as out of scope. One shape needs a
different answer: a file that tries to be a fragment and the release tool cannot read, meaning an unterminated
`---` fence or a blank line before the opening `---`. Under `strict` such a file is `claim.field-malformed`,
naming the file and line, because the pinned knope 0.23.0 rejects both with `could_not_read_changeset`;
reading them as "not a fragment" let a file through `check` that breaks the release. Without `strict` the
file is read as having no frontmatter, as before, so a repository that has not enabled `strict` sees no
change. Found by the independent review of the implementing change (`#337`), which probed knope 0.23.0 with
both shapes.

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
> | 2026-11-05 | Amended while Proposed: whether the field is required on every fragment is a `strict: true\|false` config option, default false (checked when present); under `strict` a fragment without the field is a finding, and this repository enables it. The single-line flow-list constraint from the knope test stays. Phrased over declared records and statuses, with no type named. **Why:** maintainer answers of 2026-11-05 to the open questions. | **substantive** |
> | 2026-11-05 | Amended while Proposed after review: the claim that `--base` diff scoping exists today is dropped (every fragment is read and `check` applies no diff filter), and under `strict` existing fragments are migrated in the implementing change; an id the field lists that resolves to nothing is a new finding gated by the opt-in config and applying only to the structured field; frontmatter parsing is stated as new, with a finding for a block list or malformed value and non-fragments (no frontmatter) counted out of scope; the knope 0.23.0 test is described and kept as a CI test, with multi-package workspaces noted as untested risk; the options are placed on `claim.status-agreement`; the open question about replacing knope is removed. **Why:** review blockers on the Proposed text. | **substantive** |
> | 2026-11-05 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-05. | **substantive** |
> | 2026-11-05 | Amended after Accepted: under `strict`, an unterminated `---` fence or a blank line before the opening fence is `claim.field-malformed` instead of an out-of-scope file. **Why:** an independent review of the build found that the pinned knope 0.23.0 rejects both shapes with `could_not_read_changeset`, so reading them as "not a fragment" let a fragment through `check` that breaks the release; maintainer decision of 2026-11-05. Status unchanged. | **substantive** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `no Realized-by field; cited by test back-pointers` -- stated no value disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | The 2026-11-05 change about unreadable fragments, first written into the Decision text, is moved into a dated Amendment section, and the amendment is accepted. **Why:** Maintainer decision, 2026-11-05; an Accepted decision is changed by amendment, not by editing its text. | **substantive** |
