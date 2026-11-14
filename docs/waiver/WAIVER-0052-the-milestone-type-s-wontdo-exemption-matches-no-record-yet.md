---
Stable-Id: 01M6TXX9DZE12T6XFEGF2R1R95
Rule: type.dir-matches-nothing
Scope: .urzua/config.yaml
Reason: "No milestone has been cancelled yet, so no record holds WontDo and the exemption sets nothing aside. The exemption is kept on purpose: a WontDo milestone will never be done, so it is not asked for Done means (SPEC-6). The waiver lapses on its date, and waiver.matches-nothing reports it as soon as a WontDo milestone exists and the finding stops."
Expires: 2027-02-07
Variant: exempt-status-matches-nothing
Key-path: [record_types, milestone, sections, exempt_statuses]
---
# WAIVER-0052 — The milestone type's WontDo exemption matches no record yet

## What this covers

`type.dir-matches-nothing` reports (variant `exempt-status-matches-nothing`, at `record_types.milestone.sections.exempt_statuses`) that the `milestone` type declares `sections.exempt_statuses` entry `WontDo`, but no milestone has that status (`MILE-7`). The exemption is part of the milestone type (`SPEC-6`): a cancelled milestone will never be done, so it has no Done means to meet. No milestone has been cancelled yet, so the entry has nothing to set aside today.

The waiver names that one finding (`Variant` and `Key-path`), so it hides no other `type.dir-matches-nothing` finding in the configuration, such as a misdeclared directory. It has an `Expires` and lapses on that date. It is reported by `waiver.matches-nothing` as soon as a milestone is `WontDo` and the finding stops, which is when it should be deleted.

## References

- `.urzua/config.yaml` — the `milestone` type's `sections.exempt_statuses`, the entry this waives the finding for.
- `docs/specs/SPEC-6-the-milestone-record-type.md` — the milestone type and its `WontDo` status.
- `MILE-7` — the exemption, and the report of an exemption that matches no record.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Filed with the report of an exemption that matches no record (`MILE-7`). **Why:** this repository's `milestone` type declares `WontDo` as exempt and no milestone has that status yet. | **substantive** |
> | 2026-11-14 | Narrowed to the finding it was written for: `Variant: exempt-status-matches-nothing` and `Key-path: [record_types, milestone, sections, exempt_statuses]`. It had covered every `type.dir-matches-nothing` finding in the configuration, present and future, such as a `condition` directory declared as `docs/conditionz`. **Why:** the first-run review (finding 1.6), and the maintainer decision of 2026-11-14 that a waiver may name its finding (`SPEC-10`). | **substantive** |
