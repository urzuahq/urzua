---
Stable-Id: 01M6CXZET544KZ3KCXNVXP39XS
Status: Done
Realized-by: code:rust/crates/urzua-cli/src/commands/export.rs@sha256:951071426790c6cd0ad471c38e79791a9a9e93d77df4b789b11fa29346ba3620, test:rust/crates/urzua-cli/tests/export_integration.rs#export_json_emits_every_record_with_parsed_header_and_raw_text@sha256:c355ede73c9314da9a8dcaffdbfa6c64f7d4c12c88188f868c542ca47bbc8ab4, spec:docs/specs/SPEC-1-cli.md, code:scripts/generate-dashboard.py@sha256:1fcbc09112b61edca0f8c721fda79412ed2b1541a4e41e4ce777b18d1d3685ab, code:rust/crates/urzua-core/src/rules/section.rs#exported_sections@sha256:03709597e2a19d9b7d5f1c013f5dbd392b739c04d13560af1de74f049af981e0, test:rust/crates/urzua-cli/tests/export_integration.rs#export_json_emits_declared_sections_with_aliases_resolved_and_the_preamble@sha256:c684536e0e28b6b19c43b675d84cd118f3abf717a12e7c7d9bf36c742cecccfc, test:rust/crates/urzua-cli/tests/export_integration.rs#export_json_marks_a_type_with_no_sections_declaration_with_null@sha256:b4139ecac15d412ed01dc69fa5ac064165e0844005111d1ea1c77e1a6be9c8e8, test:rust/crates/urzua-cli/tests/export_integration.rs#export_json_gives_a_declared_type_with_no_headings_an_empty_list@sha256:e547d5d9063e04547afbbab981f24c86234dc0bfd753b09cd4ea148cae77972d
Phase: '0'
Track: roadmap-tracking
---
# 115 — urzua export emits every record as plain JSON data, so tooling reads records through urzua instead of a second parser

## What

`urzua export` is not implemented today: `--format agdr` is the only declared value and the command
exits `not implemented`. Add a plain-JSON export that emits every record of every configured type as
data: its identity (display id and `Stable-Id`), type, path, and header fields, plus its sections once
`MILE-98`'s document model declares where sections come from (no type declares sections today). The
exact JSON field names are part of this milestone's deliverable and belong in `SPEC-1`'s output
contract. AgDR export stays `MILE-21`'s scope. Then switch `scripts/generate-dashboard.py` to consume that output instead
of globbing `docs/` and parsing it with its own Python and YAML logic.

## Why

`scripts/generate-dashboard.py` is a second parser of the corpus, written in a different language with
different rules about what a header is, so it can disagree with `check` about what the corpus contains.
It has: the published dashboard drifted to showing 3 bugs while the corpus held roughly 145. Tooling
that reads records should read them through `urzua`, the one implementation `check` itself uses, so a
parsing fix lands everywhere at once. This is the same one-implementation argument `SPEC-1` makes
against every team hand-writing its own linter, applied to this repository's own tooling.

## Done means

- `urzua export --format json` emits every record of every configured type as plain JSON data: display id, `Stable-Id`, type, path, header fields and raw text. — evidence: code:rust/crates/urzua-cli/src/commands/export.rs@sha256:951071426790c6cd0ad471c38e79791a9a9e93d77df4b789b11fa29346ba3620, test:rust/crates/urzua-cli/tests/export_integration.rs#export_json_emits_every_record_with_parsed_header_and_raw_text@sha256:c355ede73c9314da9a8dcaffdbfa6c64f7d4c12c88188f868c542ca47bbc8ab4
- The field names are in `SPEC-1`'s output contract. — evidence: spec:docs/specs/SPEC-1-cli.md
- `scripts/generate-dashboard.py` reads records only through the export and no longer globs or parses `docs/`. — evidence: code:scripts/generate-dashboard.py@sha256:1fcbc09112b61edca0f8c721fda79412ed2b1541a4e41e4ce777b18d1d3685ab
- The record's sections are emitted for each type that declares where sections come from, resolving aliases to the declared name. — evidence: code:rust/crates/urzua-core/src/rules/section.rs#exported_sections@sha256:03709597e2a19d9b7d5f1c013f5dbd392b739c04d13560af1de74f049af981e0, test:rust/crates/urzua-cli/tests/export_integration.rs#export_json_emits_declared_sections_with_aliases_resolved_and_the_preamble@sha256:c684536e0e28b6b19c43b675d84cd118f3abf717a12e7c7d9bf36c742cecccfc
- Each record carries `sections` and `preamble`, `null` for a type that declares none. — evidence: test:rust/crates/urzua-cli/tests/export_integration.rs#export_json_marks_a_type_with_no_sections_declaration_with_null@sha256:b4139ecac15d412ed01dc69fa5ac064165e0844005111d1ea1c77e1a6be9c8e8, test:rust/crates/urzua-cli/tests/export_integration.rs#export_json_gives_a_declared_type_with_no_headings_an_empty_list@sha256:e547d5d9063e04547afbbab981f24c86234dc0bfd753b09cd4ea148cae77972d

## References

- `SPEC-1` -- the one-config-driven-implementation constraint a second parser violates.
- `MILE-102` -- the first-adopter replacement this Phase 0 gap work serves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `export` declared only `agdr` and was unimplemented, so the dashboard script grew a second parser that already produced a wrong published count; filed so tooling reads records through `urzua`. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #137: `urzua export --format json` emits every record of every configured type as plain JSON. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Reopened: `Status` Done -> InProgress; `Realized-by` removed, evidence moved here. **Why:** the What asks for the records' sections to be exported once the declared document model says where sections come from. That condition now holds (the ADR and bug types declare `sections` in `.urzua/config.yaml`), and `urzua export --format json` emits only `type`, `path`, `id`, `stable_id`, `header_readable`, `header` and `raw_text`, no sections, so the What is not fully met. The plain-JSON export and the dashboard switch are delivered. Evidence kept from the Done state, which was a partial delivery and not the milestone's outcome: `code:rust/crates/urzua-cli/src/commands/export.rs, test:rust/crates/urzua-cli/tests/export_integration.rs`. A `Realized-by` under a not-done `Status` is a disagreement `status.delivery-agreement` (`MILE-183`) reports, so it is recorded in this row, as in `MILE-128`. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those this record's What asks for, and they are not met (see the previous row); `Status` is InProgress. | **structural** |
> | 2026-11-04 | `Status` InProgress -> Done; `Realized-by` added. **Why:** `export` now emits `sections` (name, heading, line span, content) and `preamble` for each type that declares `sections`, through the parser `section.required` uses, with aliases resolved to the declared name (`MILE-165`); a type declaring none gets an explicit `null`, not an absent key, so no-declaration is distinct from declared-and-empty. Named in `SPEC-1`'s export contract; `scripts/generate-dashboard.py` reads the milestone What and Why from `sections` and its output is unchanged. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/export.rs@sha256:951071426790, scripts/generate-dashboard.py@sha256:1fcbc09112b6 Reason: Pinned the files as they are now after checking each against the record it evidences: report.rs still discloses every non-absent exclusion in a rule's population (MILE-111), catalog.rs still registers header.case-variant-key (MILE-113), export.rs still emits each record as plain JSON and generate-dashboard.py still reads it (MILE-115), the Makefile and checks.yml still run the zero-warning gate (MILE-116), AGENTS.md and config.rs still state and read the declared-adopter premise (MILE-118), diff_scope.rs still selects the records a --base run examines (MILE-123), boundary.rs, back_pointer.rs and the corpus-h expectation still implement and prove the declared published-path boundary (MILE-125), the claim_expiry.rs, reverify.rs and commands/reverify.rs files and the changeset still implement ADR-97 beside embodiment.rs, discovery.rs and fix.rs (MILE-129), discovery.rs, new.rs and the corpus-i files still give one identity namespace across directories (MILE-130), and the rules.rs module index and the per-family rule modules still exist as the split MILE-131 describes. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
