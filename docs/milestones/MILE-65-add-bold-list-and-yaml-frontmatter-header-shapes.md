---
Status: Done
Stable-Id: 01M1YNNE70GCPJRXKCKCFTJ8Q9
Phase: '0'
Track: header-format
Implements: ADR-16, ADR-17
Blocked-on: —
Realized-by: test:rust/crates/urzua-core/src/header.rs#parses_yaml_frontmatter@sha256:d4c5618b31c831da8d49f6a85d3fbdc69ef73d3c9985b5d411d65adcfed1bec5, code:rust/crates/urzua-core/src/header.rs#parse_yaml_frontmatter@sha256:7896a93f5036961d63b5dac25fc2331954a67732c15d8306b683d98e8e9c40ea, code:rust/crates/urzua-core/src/header.rs#parse_with_shape@sha256:0684a0aa75d67805845cf344098f4076eb775d807219f0a18a524a5e8b40ecde, test:rust/crates/urzua-core/src/config.rs#header_shape_survives_a_serialize_deserialize_round_trip@sha256:c681fafff173c7d04c8aad049b2764f2a01be41c135a3f4c45369afae525a229, test:rust/crates/urzua-core/src/config.rs#a_removed_header_shape_is_a_clear_error_naming_the_form@sha256:3704ea8e26e1ead03470812212cd819ec5538a8d7452bbf58096c48dff815e80, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-a/expectations/check-removed-bold-list-config-is-rejected.yaml@sha256:f2b924fab8a7b51fa6ab4cf6b27d9e88574d33d1dffe588c19b0714d36a4ad3a
---
# 65 — Add bold-list and yaml-frontmatter header shapes

## What

Two more `header_shape` values beyond the default `blockquote`: `bold-list` (a bold markdown list with no blockquote) and `yaml-frontmatter` (real, `serde`-deserialized YAML). Declared per record type in config, never sniffed from content.

## Why

Forcing every adopted corpus to rewrite its header convention to match Urzua's own default would violate the adopt-without-a-rewrite principle `init` exists to serve -- these are the two other real shapes this project actually encountered.

## Done means

- `yaml-frontmatter`, deserialized as real YAML, is a `header_shape` value. — evidence: test:rust/crates/urzua-core/src/header.rs#parses_yaml_frontmatter@sha256:d4c5618b31c831da8d49f6a85d3fbdc69ef73d3c9985b5d411d65adcfed1bec5, code:rust/crates/urzua-core/src/header.rs#parse_yaml_frontmatter@sha256:7896a93f5036961d63b5dac25fc2331954a67732c15d8306b683d98e8e9c40ea
- `header_shape` is declared per record type in config and never sniffed from content. — evidence: code:rust/crates/urzua-core/src/header.rs#parse_with_shape@sha256:0684a0aa75d67805845cf344098f4076eb775d807219f0a18a524a5e8b40ecde, test:rust/crates/urzua-core/src/config.rs#header_shape_survives_a_serialize_deserialize_round_trip@sha256:c681fafff173c7d04c8aad049b2764f2a01be41c135a3f4c45369afae525a229
- `bold-list` was added as a `header_shape` value and, with `blockquote`, was later removed by `MILE-85`; `yaml-frontmatter` is the only shape `header.rs` reads. — evidence: code:rust/crates/urzua-core/src/header.rs#parse_with_shape@sha256:0684a0aa75d67805845cf344098f4076eb775d807219f0a18a524a5e8b40ecde
- A config naming a removed shape is reported. — evidence: test:rust/crates/urzua-core/src/config.rs#a_removed_header_shape_is_a_clear_error_naming_the_form@sha256:3704ea8e26e1ead03470812212cd819ec5538a8d7452bbf58096c48dff815e80, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-a/expectations/check-removed-bold-list-config-is-rejected.yaml@sha256:f2b924fab8a7b51fa6ab4cf6b27d9e88574d33d1dffe588c19b0714d36a4ad3a

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Partly superseded by `MILE-85`; `Status` stays Done. **Why:** the closure audit of #234 found the What stale. The `bold-list` header shape this milestone added was removed by `MILE-85`, and `blockquote` went with it; `yaml-frontmatter` is now the only shape `header.rs` reads, and a config naming the removed shapes fails to load. The milestone delivered both shapes when it shipped, and the `yaml-frontmatter` half remains in force. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/tests/acceptance/corpora/corpus-a/expectations/check-removed-bold-list-config-is-rejected.yaml@sha256:f2b924fab8a7 Reason: Pinned the files as they are now after checking each against the record it evidences: report.rs, config.rs and runner.rs still carry the info level (MILE-46), init.rs still proposes a config in adopt mode (MILE-57), doctor.rs, the Makefile, both git hooks and checks.yml still report and wire the gate (MILE-58), waiver.rs, the waiver template and the WAIVER-0035 record still define and hold a reviewed exception (MILE-59), new.rs and new_record.rs still create the next record (MILE-60), both fix files and fix_drift_integration.rs still detect and apply (MILE-61), pointer.rs, audit.rs and the corpus-b expectation still run the audit that reports one-sided supersession (MILE-62), explain.rs and graph.rs still print the governing records and the edges (MILE-63), the corpus-a expectation still shows a config naming the removed bold-list shape rejected, which is what the milestone's item says (MILE-65), and migrate.rs still reports what a newly required field would break without writing (MILE-68). MILE-46's revision table had a blank line between its header and its rows, which hid every row from the parser, so that line was removed before pinning. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/header.rs#parse_yaml_frontmatter` (lines 255-322) b9f96f723206 -> 7896a93f5036. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: header.rs: split parse_yaml_frontmatter into header_without_region and frontmatter_fields; the opening-line, closing-line and YAML-error checks stay in parse_yaml_frontmatter in the same order with the same parse_error text. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline and reverify JSON compared before and after, with and without --as-of). **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
