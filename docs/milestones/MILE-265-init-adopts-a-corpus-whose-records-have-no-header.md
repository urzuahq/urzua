---
Stable-Id: 01M7FW5MSSZVTYPFH2JT42FN3D
Status: Done
Phase: '1'
Track: header-format
Blocked-on: null
---
# 265 — init adopts a corpus whose records have no header

## What

Make `urzua init` propose a working configuration for a corpus in the common ADR style (numbered files, a title heading, status and context sections, no header block), and stop `doctor` sending the user back to `init`.

## Why

A walkthrough of 2026-11-15 followed only the user documentation with the released binary. `init` refused a fifteen-record corpus in the most common ADR style because no file has a header, and `doctor` then advised running `init`, a loop. The whole configuration had to be written by hand.

## Done means

- `urzua init` on such a corpus proposes a type with the sections it found as required sections, and `urzua check` then runs. — evidence: test:rust/crates/urzua-cli/tests/init_adopt_integration.rs#init_proposes_a_headerless_type_for_the_common_adr_style_and_check_runs_observed_failing@sha256:191ce20a81813dbf835af5f451f0a7188bc951c9c0cc13fc1472a5254079559a
- `urzua doctor` after a refused `init` names the next step that works. — evidence: test:rust/crates/urzua-cli/tests/init_integration.rs#doctor_after_a_refused_init_names_a_step_that_works_observed_failing@sha256:c1cf2e52b0404d726b743331a5b069cd2d83b9725b171037e2ac551eb6479156
- `urzua init` in an empty repository names `--types`. — evidence: test:rust/crates/urzua-cli/tests/init_integration.rs#init_in_an_empty_repository_names_the_greenfield_command_observed_failing@sha256:99736771412a2f9e44184b93bb72ee7871b0499f6eecc73a28ca03d253b7ba6a
- An index page in a record directory is not proposed or read as a record. — evidence: test:rust/crates/urzua-cli/tests/init_adopt_integration.rs#index_pages_and_numberless_files_beside_numbered_records_are_excluded@sha256:6a2f768518f55cfd88678c9324e63e233ff3b4b2cca52bedd4ae7d0b5158266f

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (4 of 4 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
