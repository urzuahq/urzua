# 0015 — A waiver is a first-class record, not an ignore list

> Status: Accepted
> Date: 2026-09-04
> Author: (project lead)
> Supersedes / Superseded-by: —

> **Decided at proposal time, 2026-09-04.** See [ADR-0011](../adr/0011-a-waiver-is-a-first-class-record.md).

## Summary

Every real corpus accumulates legitimate exceptions to its own rules. The obvious mechanism — a
config-level ignore list (`ignoredFindings: [...]`) — is exactly the anti-pattern RFC-0011 and
RFC-0013 both already reject: invisible to the person reading the corpus rather than the config,
grows without bound, and can silently suppress a real error alongside the reviewed one. Propose
instead that a **waiver is a record**, of the same kind as an ADR or RFC: authored, reviewable, and
subject to the same header/discovery rules as everything else in the corpus.

## Motivation

The waiver question sits directly in front of running `urzua check` against any corpus this project
doesn't fully control — which is the entire point of eventually testing it against one (roadmap
Phase 8). A corpus with real, deliberate exceptions and no way to record them either fails `check`
forever on things a human already reviewed and accepted, or reaches for the ignore list this
project's own decisions already ruled out. Deciding the mechanism now, deliberately, is cheaper than
improvising one under the pressure of a foreign corpus's first failing run.

## Proposal

### 1. A waiver is a record with an author, a reason, and an optional expiry

A new record type, `waiver`, with a small profile: which rule it covers, which file or scope it
applies to, why (free text, required — a waiver with no stated reason is not reviewable), who
authored it, and an optional `expires` date. No expiry means the exception is asserted as
structural — the same shape SPEC-0001's own permanent content-scope ceiling already is: a real
exception this project's own design cannot mechanically close, not merely one nobody got around to
fixing. An `expires` date means the opposite: the waiver reverts to blocking automatically once
passed, and nothing about that reversion depends on anyone noticing the date arrived.

This mirrors the project's own thesis directly: **memory is data, decisions are information** — a
waiver is a decision (accept this exception, for this reason, until this date or permanently) and
belongs in the decision layer, not in a tool's config file.

### 2. A waived finding is listed, never omitted

`check`'s output includes waived findings in `findings`, each carrying a `waived` field pointing at
the waiver record that covers it. `status` and `blocking` are computed from **non-waived** findings
only — a waiver silences the exit code, not the visibility. This is the same rule SPEC-0003 already
states for a disabled check ("reported as skipped, never omitted"), applied to waived findings
specifically. A `urzua check` run that waives ten findings and reports clean must still show all ten
in its output; hiding them would recreate the exact invisibility an ignore list has.

### 3. An expired waiver is not a waiver

A waiver whose `expires` date has passed is treated as though it does not exist: the finding it
covered reverts to blocking, with no separate "expired waiver" state to configure. This is
deliberate — an expiring exception that quietly stops applying and requires no further action from
anyone is the only shape that can't be forgotten silently. Renewing an exception means editing the
waiver's `expires` field, which is itself a normal, visible, reviewable edit to a record.

## Open questions

- **Can a waiver cover more than one finding at once** (a whole rule, a whole file, a whole
  directory)? The profile above assumes one rule + one scope per waiver; whether a broader waiver
  needs its own shape or should just be several waiver records is undesigned.
- **Does waiving a finding require the same Decider role enforcement as an ADR?** Consistent with
  the project's role model would suggest yes — a waiver is a decision — but this needs the role
  model to exist first (deferred with RFC-0001 §1e).

## Non-goals

- **Auto-generating a waiver from a `check` run.** A waiver is authored deliberately; `check` never
  proposes exceptions to its own findings.
- **A UI or workflow for waiver review.** This RFC specifies the record shape and the reporting
  contract, not how a team decides to grant one.

## References

- RFC-0011 §5, RFC-0013 — the ignore-list anti-pattern this RFC's mechanism avoids.
- SPEC-0001 — the permanent content-scope ceiling, the precedent for a no-expiry structural waiver.
- SPEC-0003 — "a disabled rule is reported as skipped, never omitted," the principle §2 extends to
  waived findings.
- ADR-0010 — the core+profile schema a `waiver` record type extends.
