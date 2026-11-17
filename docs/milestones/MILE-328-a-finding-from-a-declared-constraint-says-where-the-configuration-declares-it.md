---
Stable-Id: 01M7KM467S73EZYC0T1H41NG46
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 328 — A finding from a declared constraint says where the configuration declares it

## What

Have every finding that enforces something a configuration declared name the configuration key and line that declares it, and have the reference say for each rule whether it acts only on a declaration.

## Why

On 2026-11-17 the maintainer read two findings from an adopting repository (a field value outside a declared list; a field required when another field holds a given value) as rules built into the engine, and objected that the engine should not force such things. It does not: both came from that repository's own configuration. But nothing in the finding says so. "field 'Kind' holds 'defect', which is not declared for type 'task' (allowed: ...)" reads as the tool's opinion to anyone who has not read that configuration, and the reference pages describe what a rule reports without saying that it reports nothing unless a declaration asks. A product whose rule is that configuration owns behaviour should show, at the point of every refusal, the line of configuration that asked for it.

## Done means

- A finding produced because of a declaration carries the configuration file, key path and line of that declaration as a field of the finding, and its message says the constraint is declared there, shown by a test for a declared value list, a declared requirement, a declared target type and a gated status.
- The suggested action of such a finding offers both ways forward: change the record, or change the declaration, naming the key.
- `urzua rules` and the rule table in the specification mark each rule as acting only on a declaration, or as a shape check that needs none, and the reference page for configuration opens with the statement that the engine enforces nothing about field names, values or record types that the configuration does not declare.
- `urzua explain <record>`, a new form of that command, lists the declarations that apply to it and where each is declared.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** The maintainer said on 2026-11-17, on reading an adopting repository's findings as rules the engine forces, "everything is in config, not in the code" and "if it's getting confused, this is a usability issue where the documentation needs improving". | **structural** |
