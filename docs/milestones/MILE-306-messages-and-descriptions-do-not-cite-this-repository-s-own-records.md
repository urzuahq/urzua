---
Stable-Id: 01M7HY9EXFVZREM0ZN7MK05NQ4
Status: Planned
Phase: '1'
Track: user-docs
Blocked-on: null
---
# 306 — Messages and descriptions do not cite this repository's own records

## What

Remove this repository's record ids and its own type and status names from the engine's messages, rule descriptions and tool descriptions, so the text is true in any repository.

## Why

An audit on 2026-11-16 read all non-test engine code for places that name a particular record type, status, field or workflow. It found about twenty messages citing this repository's own records by id, two tool descriptions that state this repository's types and statuses as fact, a rule listing that prints this repository's spec id, and many rule descriptions using this repository's ids as examples.

## Done means

- No message, rule description or tool description in non-test engine code names a record id of this repository or states a type or status name as fact, enforced by a test over the rule table and the tool descriptions.
- Where an example helps, it is built from the repository's declared types.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from the audit of 2026-11-16, asked for after the maintainer said the tooling has no opinion about which record types a repository defines and that this is driven by configuration, not by the code. | **structural** |
