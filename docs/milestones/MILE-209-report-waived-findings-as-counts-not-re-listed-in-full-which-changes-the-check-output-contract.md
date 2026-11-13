---
Stable-Id: 01M6SSWGY7C4TS3EN6EVWFWGHG
Status: Planned
Phase: '1'
Track: scale-and-analytics
Blocked-on: null
---
# 209 — Report waived findings as counts, not re-listed in full, which changes the check output contract

## What

Report waived findings in `urzua check` output as counts per rule, with the findings listed only when asked, so a waived finding is not repeated in full on every run. This changes the output contract.

## Why

On an adopter with several hundred ADRs the report is very large for a handful of live findings, because every waived finding is re-listed in full (adopter end-to-end second pass, L3). The size costs every consumer that parses the report. Consumers read the waived list today, so the change needs a decision on the contract, which is the milestone's first step.

## Done means

- By default the report carries waived findings as a count per rule and the waiver that covers them; a documented option lists them in full.
- The unwaived findings, the populations, the exit code and `status` are unchanged, and a test pins both shapes.
- The gate action, the editor hook and `harness` consume the new default and still report every live finding.
- A decision on the contract change is recorded before the build (an ADR needing the maintainer's acceptance).
- `SPEC-2` and `SPEC-8` state the output with revision rows.
- A changeset names the change.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Initial record. **Why:** deferred during Phase 0 hardening and tracked by no record. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-10 | Why reworded to name no adopter count; the Done means are unchanged. **Why:** a distinctive count of one private corpus does not belong in the public tree. | **structural** |
