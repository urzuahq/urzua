---
Stable-Id: 01M7N42DTHNRWZWQ594JPJJJN1
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on:
---
# 341 — Each rule reports which record types it judges under a configuration, so a report can say it without guessing

## What

Each rule exposes, from its own code, which record types it would judge under a given configuration and which declaration makes it so. Commands that describe a configuration read that, and no command works it out a second way.

## Why

`urzua explain --types` was built to say which rules apply to each type. Five reviews on 2026-11-17 found that statement false in a new way each round: a rule listed for a type whose records it never reads, a rule that does read them left out, a type named like a setting key tied to a rule through that key. The cause was the same every time: which records a rule judges is decided inside the rule, and any table or walk kept outside the rules drifts from it. As released in 0.4.9 the command says nothing about which rules apply and points at `urzua check`. An adopting repository's agent reported that this does not hurt in practice, so this is not urgent; it is the way to bring the statement back that the reviews did not rule out.

## Done means

- Every rule in the rule table answers, through one function the rule owns, which record types it judges under a loaded configuration and the key path of each declaration that brings a type in; the rule's own evaluation uses the same function to select records, shown by a test per rule that the types it reports are exactly the types whose records it examines on a corpus built for that purpose.
- `urzua explain --type <name>` lists the rules that judge the type, each with its level as set or defaulted and the declaring key path and line, taken only from that function; a rule that judges only under `check --base` says so.
- No command holds its own table or walk of which rule reads which declaration; a test fails when one is added.
- The capability is exercised on urzua's own `.urzua/config.yaml`: for each of its types, the rules listed equal the rules that examine at least one record of that type in a `check` run.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-18 | Initial record, `Status: Planned`. **Why:** five reviews of `urzua explain --types` on 2026-11-17 showed that which records a rule judges cannot be stated from outside the rule. | **structural** |
