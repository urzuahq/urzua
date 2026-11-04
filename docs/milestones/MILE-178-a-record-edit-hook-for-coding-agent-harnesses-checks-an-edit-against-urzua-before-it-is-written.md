---
Stable-Id: 01M6K4FKKANSVK5ZE3JZ0MCX9Y
Status: Planned
Phase: '0'
Track: governance-process
Implements: RFC-7, ADR-69
Blocked-on: MILE-12 (decide RFC-7)
---
# 178 — A record-edit hook for coding-agent harnesses checks an edit against urzua before it is written

## What

A hook for coding-agent harnesses that checks an edit to a record before it is written. It is matched on the record directories, hands the edit's prospective content to `urzua` (the path-scoped, stdin form `MILE-159` builds), and blocks an edit that sets a field `urzua` computes, today `Embodiment`, to a value that disagrees with the computed one, whether it claims more or less than the evidence supports. The message names the computed value and the evidence it came from. The hook is a thin adapter: it holds no derivation of its own (`ADR-69`).

## Why

A real adopter environment runs exactly this hook, so that an agent cannot set a computed field from its own reading of the prose without a logged override. Here the same rule is an instruction: `AGENTS.md` says to run `urzua fix --apply` rather than hand-assert `Verified`, and `RFC-7` opens with the scenario of an agent editing `Embodiment` without running the tool. `RFC-27` section 5 names the same shape for acceptance: a convention that an instruction asks agents to follow, which is the argument of `RFC-7` turned on this project. `urzua fix` is whole-corpus today and parses its whole repair list to find one file (`MILE-159`), so an editor hook has no cheap, stable thing to call yet. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

In this repository a harness hook is configured for edits under `docs/`: a planted edit setting `Embodiment: Verified` on a record with no evidence is blocked with the computed value and the evidence named, the correct value passes, and an edit to any other field of the same record passes; when `urzua` is missing, returns `not-run` or cannot read history, the hook allows the edit and says plainly that it did not judge, in output the agent sees (never a silent pass); an override is explicit, carries a name and is recorded in the hook's output; a single-file judgement completes in well under a second on this corpus; the hook is tested with recorded harness payloads for Edit, Write and a multi-edit; and the configuration and records in this repository are what it runs against, not only the tests.

This milestone builds the parts of `RFC-7` section 2 and section 3 (first bullet) that `ADR-69` drafts. It does not decide `RFC-7`'s open questions, which `MILE-12` owns.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed that environment's pre-write hook has no counterpart here; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | `Blocked-on` now reads `MILE-12` only. **Why:** `MILE-159` is `Done` (its path-scoped run and prospective-content judgment are delivered), and `narrative-field.stale` flagged the reference; set by the delivery backfill of `MILE-183`. `Status` unchanged. | **substantive** |
