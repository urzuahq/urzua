---
Status: Done
Stable-Id: 01M1Y5JYB0J2X7P0AVRJKWJAJV
Phase: '0'
Track: schema-governance
Implements: RFC-6
Depends-on: MILE-171
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/schema_version.rs, test:rust/crates/urzua-cli/tests/schema_version_integration.rs#the_fixture_at_each_supported_version_loads_to_the_same_effective_config@sha256:f4edfba5a5061aa858ca7cb8bc9a7617ad9f105ca780fcb5d17930aec62f2cf8, code:rust/crates/urzua-cli/src/commands/migrate.rs, spec:docs/specs/SPEC-3-configuration.md, test:rust/crates/urzua-cli/tests/schema_version_integration.rs#a_previous_version_config_gives_the_findings_the_current_one_does_plus_one_warning@sha256:f8359c0933c1d1b4a7703dec5986e5a168cce460c6c58e34aff42f254caf5e54, test:rust/crates/urzua-cli/tests/schema_version_integration.rs#migrate_config_never_reads_a_version_outside_the_window@sha256:f445dfca8f0269609259cf2d1c8a6903eb4e37b104d3af648b0b7178d14396cd, code:rust/crates/urzua-core/src/rules/table.rs, code:.urzua/config.yaml
---
# 34 — Profile schema versioning and generalized cross-record validation

## What

Version the profile schema itself (not just individual records), and generalize the ad hoc validation check/fix already does (reciprocity, dangling references) into a stated, pluggable model.

## Why

RFC-6 versions records; nothing versions the schema those records are validated against -- an explicitly open question in research with no answer yet.

## Done means

- The profile schema carries a version that `check` reads. — evidence: code:rust/crates/urzua-core/src/schema_version.rs, test:rust/crates/urzua-cli/tests/schema_version_integration.rs#the_fixture_at_each_supported_version_loads_to_the_same_effective_config@sha256:f4edfba5a5061aa858ca7cb8bc9a7617ad9f105ca780fcb5d17930aec62f2cf8
- A mismatching version is reported with a migration path rather than a bare equality reject. — evidence: code:rust/crates/urzua-cli/src/commands/migrate.rs, spec:docs/specs/SPEC-3-configuration.md, test:rust/crates/urzua-cli/tests/schema_version_integration.rs#a_previous_version_config_gives_the_findings_the_current_one_does_plus_one_warning@sha256:f8359c0933c1d1b4a7703dec5986e5a168cce460c6c58e34aff42f254caf5e54, test:rust/crates/urzua-cli/tests/schema_version_integration.rs#migrate_config_never_reads_a_version_outside_the_window@sha256:f445dfca8f0269609259cf2d1c8a6903eb4e37b104d3af648b0b7178d14396cd
- The ad hoc cross-record validation (reciprocity, dangling references) is stated as a declared model that the rule table (`MILE-171`) runs. — evidence: code:rust/crates/urzua-core/src/rules/table.rs, spec:docs/specs/SPEC-3-configuration.md
- A fixture on an older schema version is reported. — evidence: test:rust/crates/urzua-cli/tests/schema_version_integration.rs#a_previous_version_config_gives_the_findings_the_current_one_does_plus_one_warning@sha256:f8359c0933c1d1b4a7703dec5986e5a168cce460c6c58e34aff42f254caf5e54
- The versioning is exercised in urzua's own `.urzua/config.yaml`. — evidence: code:.urzua/config.yaml

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Re-phased from Phase 1 to Phase 0 (pulled into Phase 0 as foundation work); `Status` unchanged. **Why:** Maintainer decision of 2026-11-04 defining Phase 0 as parity on a solid codebase (`SPEC-1`'s Phase 0 exit): `schema_version` is a hard equality reject in `config.rs` with no migration path, and the pluggable cross-record validation model is what the rule-table work (`MILE-171`) builds on. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Depends-on: MILE-171` added. **Why:** Done means requires the cross-record validation model to be run by the rule table `MILE-171` built (Done). Proposed for maintainer review; `Status` unchanged. | **structural** |
> | 2026-11-05 | Delivered, as `ADR-77` (Accepted) decides: a window of `N` and `N-1`, an in-memory upgrade from version 1 to version 2 with the `config.schema-version-outdated` warning and its `run-command` action, the structured out-of-window error with exit 2, `urzua migrate config`, and `SPEC-3`'s "Supported schema versions" table and bump rule; a fixture at each supported version (`corpus-k`) loads to the same effective config; the rule is enabled in this repository's own `.urzua/config.yaml`. The declared cross-record model was already in place: reciprocity is `relation_pairs` and `relation_fields` (`relation.pair-reciprocity`, `relation.supersession-reciprocity`) and dangling references are `pointer_fields` (`pointer.resolution`), each a rule in `RULE_TABLE`. `Realized-by` and `Status: Done` set by the delivering change. **Why:** every clause of the Done means is met; the maintainer reviewing the PR is the human decision. | **substantive** |
> | 2026-11-05 | The version-1 upgrade refuses a record type that omits `header_shape` (version 1 read it as `blockquote`, which `MILE-85` removed) instead of silently changing it to `yaml-frontmatter`; `SPEC-3` says so and a test covers it. **Why:** review of the pull request that delivered this record; `ADR-54` (a break is a break) and `ADR-77` (an upgrade keeps behaviour or says it cannot). | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
