---
Stable-Id: 01M6NNNZJ92WC419AF3NQP471H
Status: Draft
Date: 2026-11-05
Version: '0.1'
Author: beauwilliams
Subject: 'The `condition` record type -- a dependency on a fact that is observed rather than built, declared in configuration alone.'
Parent: —
---
# SPEC-23 — The condition record type

## Purpose

A `condition` is a record for a fact nobody builds: an open question, or a signal such as an adopter
asking for a feature. `RFC-48` sorts what can stop a record into work, which a `Depends-on` link to a
record already expresses, and facts, which no record expressed. A `condition` is that record, so a
milestone, ADR or RFC that waits on a fact points at it with `Depends-on`, and `urzua roadmap`
(`SPEC-3`) and the derived `blocked` value read the wait like any other. The type is the first built
entirely from configuration: it uses a declared directory, prefix and identity, a status vocabulary
with success and failure sets, `transitions` and `statuses.require`, and nothing else. The engine has
no knowledge of the name `condition`; the shape under any other name behaves the same.

## Schema

```yaml
record_types:
  condition:
    dir: "docs/conditions"
    prefix: "COND"
    identity: {from: filename, pattern: '^(?P<prefix>COND)-(?P<number>\d{4})-(?P<slug>.+)$'}
    required_fields: ["Status"]
    header_shape: "yaml-frontmatter"
    known_fields: ["Stable-Id", "Met-by"]
    statuses:
      values: ["Open", "Met", "Dropped"]
      success: ["Met"]
      failure: ["Dropped"]
      initial: ["Open"]
      transitions:
        Open: ["Met", "Dropped"]
      require:
        - {when: ["Met"], fields: ["Met-by"]}
    sections: {from: headings, required: [{name: Signal}]}
    pointer_fields: ["Met-by"]
    narrative_fields: []
    spec: "SPEC-23"
```

Files are `COND-NNNN-slug.md` (`ADR-36`), with a four-digit padded number, and the H1 opens
`COND-NNNN`. `COND-1` and `COND-0001` name the same record.

| Field | Values | Notes |
|---|---|---|
| `Status` | `Open`, `Met`, `Dropped` | `Open` is the only initial state: a condition is filed `Open`, never already `Met`. It moves to `Met` or `Dropped`, and both are terminal (`ADR-72`); a mistaken closure is a new condition, not a reopening. |
| `Met-by` | a record reference | The record that shows the signal happened, such as the ADR that records the decision or the milestone that names the adopter's ask. Required once `Status` is `Met` (`status.required-field-missing`). It is left unset while the condition is `Open`: it records the fact, and the fact has not happened yet, even where the record that will hold it is already known. A pointer field, so it must resolve to a record. |

`Met` is the type's success state and `Dropped` its failure state, which is all the sequencing rules
and the roadmap read.

## The Signal section

Every condition carries a `## Signal` section (`section.required`). It says exactly what counts as
met: the observable fact, who reports it and where it is recorded, written so that a reader who has
never seen the record can tell whether it has happened. Structural presence is all `check` can verify;
whether the Signal is precise is a reviewer's judgement, as for any section (`SPEC-1`).

## How a condition is read

- **`Depends-on` resolves to it.** A record of any type that declares `Depends-on` as a pointer field
  and takes part in the sequencing pair points at a condition as at any record (`ADR-65`,
  `cross_type: true`). `sequence.cycle` and `sequence.unresolved-blocker` examine the edge, and a
  reference to a condition that does not exist is reported (`pointer.resolution`). A target type that
  declares no status is reported by `relation.target-status-undeclared`, never read as satisfied
  (`ADR-60`).
- **Blocked.** A dependent of an `Open` condition is blocked and out of the ready set. A dependent of
  a `Met` one is not blocked. A dependent of a `Dropped` one stays out of the ready set and is listed
  in the roadmap's `needs_replan`, as a dependent of any failure-state record is.
- **Who moves it.** A person moves a condition to `Met` or `Dropped`. An agent drafting the work that
  waits on it files the condition `Open` and does not decide it (`AGENTS.md`).

## What's deliberately not built

- **`condition.review-due` and the `Review-by` field.** A rule that reports a long-`Open` condition is
  new engine behaviour that `RFC-48` proposes and no accepted ADR covers; it needs its own ADR first.
- **`sequence.failed-dependency`.** The same: a rule reporting a dependent of a `Dropped` condition is
  a new rule, and the roadmap's `needs_replan` is the only place a dropped condition surfaces today.
- **A condition for a decision.** `RFC-48` points a wait on a decision at the RFC or a decide-milestone
  instead. The four waits that `MILE-187` migrated (`MILE-13`, `MILE-21`, `MILE-23`, `MILE-30`) were
  all named for migration by the maintainer, so two of them are conditions on a decision (`COND-1`,
  `COND-4`) whose `Signal` cites it.
- **`Depends-on` on this repository's ADRs and RFCs.** The type resolves from any record that
  declares the field; this repository's `adr` and `rfc` types do not declare it, so only milestones
  point at conditions here.

## References

- `RFC-48` — the roadmap is computed from records; proposes the condition type.
- `ADR-72` — allowed status transitions, declared per type.
- `ADR-60` — a cross-record status read is gated on the target type declaring status.
- `ADR-65` — relation pairs are declared top-level.
- `ADR-36` — filenames carry the type prefix.
- `SPEC-3` — configuration, including `roadmap` and the sequencing pair.
- `MILE-187` — the milestone that declared the type and migrated the four waits.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Initial spec. **Why:** `MILE-187`, maintainer decision of 2026-11-05: the `condition` type of `RFC-48` is declared in configuration so waits on facts are structured and the computed roadmap sees them. | **structural** |
> | 2026-11-05 | `Met-by` is unset while `Open`. **Why:** review of `MILE-187`: `Met-by` names the record that shows the signal happened, so naming a record before then asserts evidence that does not exist; `COND-1` no longer sets `Met-by: RFC-9`. | **structural** |
