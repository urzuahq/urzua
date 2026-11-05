---
Stable-Id: 01M6NNKBVYT98MBB7B1QBRY7C9
Status: Open
---
# COND-0001 — RFC-9 question 2 is resolved and recorded

## Signal

Met when the maintainer resolves question 2 of `RFC-9` (whether inferred, LLM-authored record content is trusted at all, the project's largest product risk, under "Two" in the RFC's Motivation) and the resolution is written down in `RFC-9` itself or in an ADR that derives from it. A conversation, a leaning or a re-worded open question does not meet it: the answer has to be in a record.

`Met-by` names the record that holds the answer. `MILE-13` waits on this condition, because it cannot decide the agentic repair tier before the question it rests on is settled.

## Why this is a condition

`MILE-13` waited on this in prose (`Blocked-on: RFC-9's own Q2`). `RFC-48` would point such a wait at the RFC itself; the maintainer's decision of 2026-11-05 names `MILE-13` for migration, so the wait is a condition whose `Met-by` is that record.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed `Open`. **Why:** `MILE-187`, the first records of the `condition` type (`RFC-48`). A person, not the PR that files it, moves it to `Met` or `Dropped`. | **structural** |
> | 2026-11-05 | `Met-by` removed while `Open`. **Why:** `Met-by` records the fact; it is set when a person moves the condition to `Met`, and names the `RFC-9` text or ADR that holds the answer then. | **structural** |
