---
Stable-Id: 01M7KKP3HF1VGWMZS87ZAE592Z
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 327 — A finding about an empty or absent field says which it is and what may go there

## What

Make the findings about a required field with no value distinguish a field that is absent from one that is present and empty, and name the values the field may hold when the type declares them.

## Why

On release 0.4.7, straight after `urzua new`, the finding on a required field with a declared value list reads "field 'Kind' is Blank" and does not say which values are allowed, so the author opens the configuration to find out; `field.value-not-declared` already names the list. And with a field line absent entirely, the conditional requirement's message reads "Kind is bug, which requires 'Severity', but it is Blank -- not a real, present value": the field is not blank, it is not in the file.

## Done means

- A finding that a field is empty names the declared values for that field when the type declares them, in the same form `field.value-not-declared` uses, shown by a test.
- A finding about a required field that is absent from the header says it is missing, and one about a field present with no value says it is empty, in `field.quality` and the status requirement rules, as `header.required-fields` already does, shown by a test of each.
- The suggested action for such a finding carries the declared values as its inputs where a value list exists, shown by a test.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** Reported on 2026-11-17 by an agent in an adopting repository after moving to release 0.4.7, relayed at the maintainer's request. | **structural** |
