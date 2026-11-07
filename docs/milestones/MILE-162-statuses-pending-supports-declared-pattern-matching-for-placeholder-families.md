---
Stable-Id: 01M6JW44TCEWCNANCN4X8ZZ8K0
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: test:rust/crates/urzua-core/src/status_model.rs#a_pending_pattern_matches_a_placeholder_family_beside_whole_values_observed_failing@sha256:5d3eb726625276b2fb3c2b21352b6d20f273fec9891591aeaf684fb2dd7ebf85, test:rust/crates/urzua-cli/tests/placeholder_vocabulary_integration.rs#a_declared_pending_pattern_reads_a_placeholder_family_observed_failing@sha256:ace1c1a270dfd8dfb4eba24571743e4412050df1e3ad3364c3f813e2c0166632, code:.urzua/config.yaml
---
# 162 — `statuses.pending` supports declared pattern matching for placeholder families

## What

Let `statuses.pending` (`MILE-153`) list patterns as well as whole values, so a family of placeholder statuses such as a prefix followed by a variable tail is declared once.

## Why

A real adopter corpus uses placeholder values that share a stem and differ in a tail, so an exact-value list needs an entry per variant and misses the next one. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- A declared pattern in `statuses.pending` makes every matching value read as pending rather than as a status. — evidence: test:rust/crates/urzua-core/src/status_model.rs#a_pending_pattern_matches_a_placeholder_family_beside_whole_values_observed_failing@sha256:5d3eb726625276b2fb3c2b21352b6d20f273fec9891591aeaf684fb2dd7ebf85, test:rust/crates/urzua-cli/tests/placeholder_vocabulary_integration.rs#a_declared_pending_pattern_reads_a_placeholder_family_observed_failing@sha256:ace1c1a270dfd8dfb4eba24571743e4412050df1e3ad3364c3f813e2c0166632
- A value matching no entry and no pattern is still read as a real value, not as pending. — evidence: test:rust/crates/urzua-core/src/status_model.rs#a_pending_pattern_matches_a_placeholder_family_beside_whole_values_observed_failing@sha256:5d3eb726625276b2fb3c2b21352b6d20f273fec9891591aeaf684fb2dd7ebf85, test:rust/crates/urzua-cli/tests/placeholder_vocabulary_integration.rs#a_declared_pending_pattern_reads_a_placeholder_family_observed_failing@sha256:ace1c1a270dfd8dfb4eba24571743e4412050df1e3ad3364c3f813e2c0166632
- A fixture covers a pattern and a whole value together. — evidence: test:rust/crates/urzua-cli/tests/placeholder_vocabulary_integration.rs#a_declared_pending_pattern_reads_a_placeholder_family_observed_failing@sha256:ace1c1a270dfd8dfb4eba24571743e4412050df1e3ad3364c3f813e2c0166632
- The capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/placeholder_vocabulary_integration.rs#a_declared_pending_pattern_reads_a_placeholder_family_observed_failing@sha256:ace1c1a270dfd8dfb4eba24571743e4412050df1e3ad3364c3f813e2c0166632

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Built: `statuses.pending` accepts the structured entry `{pattern: "..."}` beside whole values (`SPEC-3` 0.39, `SPEC-2` 0.52); an invalid, blank or repeated pattern is a load error. Exercised by this repository's ADR type (a `TBD` / `to be decided` family) and `placeholder_vocabulary_integration.rs`. **Why:** the milestone's Done means. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged on `main`: `statuses.pending` accepts `{pattern: ...}` entries. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
