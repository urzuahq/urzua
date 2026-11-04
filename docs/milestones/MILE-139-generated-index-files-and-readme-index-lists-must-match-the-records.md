---
Stable-Id: 01M6HZ0N6A83BE6C89RP8F46S9
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
---
# 139 — Generated index files and README index lists must match the records

## What

A generated artifact must be current: where a repository keeps generated index files (a structured index file in a record directory) or a list of records between markers in a README, `check` regenerates the expected content from the records and fails when the committed copy differs. A missing file, a stale entry and a removed marker pair are each an error.

## Why

The adopter regenerates two indexes per record directory in its audit and its linter then verifies they are fresh, and for its proposals that freshness check is the only structural check applied. `MILE-42` (an `INDEX.json`) and `MILE-115` (`export`) serve discovery by tools, not a freshness gate, so a repository that commits an index has nothing here that stops it going stale. The alternative, deleting the committed indexes and deriving them with `export`, is available to an adopter but is its choice; the engine must support the checked-in case.

## Done means

A record added, removed or retitled without regenerating the index fails `check` with an error naming the stale artifact and the differing entry; a missing artifact and a removed marker pair each fail; regenerating clears the findings; the rule reports how many generated artifacts it compared.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter treats a missing or stale generated index, or a removed README marker pair, as an error, and urzua has no equivalent gate. | **substantive** |
