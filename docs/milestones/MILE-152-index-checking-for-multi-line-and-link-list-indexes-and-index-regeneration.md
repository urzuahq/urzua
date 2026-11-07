---
Stable-Id: 01M6JKNRESRV32NBGDRTQD3M1A
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: test:rust/crates/urzua-core/src/rules/index.rs#a_multi_line_entry_is_checked_as_one_entry_observed_failing@sha256:c95c72d6df66ac86dbb33841eeec1c2411e46966bf5e47e0985a0a1f70ca845d, test:rust/crates/urzua-core/src/rules/index.rs#a_title_excludes_the_records_own_id_whatever_its_form@sha256:959ac63a3d73bf771505fb473e27578b88d885a7188ed639a1a7eb54532fb775, test:rust/crates/urzua-core/src/rules/index.rs#a_link_list_needs_no_id_token_and_identifies_by_file@sha256:16defb95e5d13367ae53457d37f8dc95d431bb005f111c28000834db248c2135, test:rust/crates/urzua-cli/tests/index_freshness_integration.rs#index_regenerates_a_declared_index_and_a_second_run_is_a_no_op_end_to_end@sha256:7046d1eaf20f875864e3d251792ff2156105132c5865fe4e2d7ff1ba511f003e, code:rust/crates/urzua-cli/src/commands/index.rs, code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-index-rules-run.yaml
---
# 152 — Index checking for multi-line and link-list indexes, and index regeneration

## What

Extend index checking to accept entries spread over several lines (a YAML index), a `{title}` that excludes the id, and an `{id}` that need not be a visible token; and add a way to regenerate an index from the records.

## Why

Index checking today reads one line per entry, treats `{title}` as the whole H1 including the id, and requires `{id}`, so a list of linked entries needs a hidden comment to satisfy it. Nothing regenerates an index, which the adopter's old tool did on every audit; hundreds of index lines would otherwise be maintained by hand or by a side script, the duplication urzua is meant to remove. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- A multi-line index entry shape is declared and checked. — evidence: test:rust/crates/urzua-core/src/rules/index.rs#a_multi_line_entry_is_checked_as_one_entry_observed_failing@sha256:c95c72d6df66ac86dbb33841eeec1c2411e46966bf5e47e0985a0a1f70ca845d
- `{title}` can mean the H1 without its id. — evidence: test:rust/crates/urzua-core/src/rules/index.rs#a_title_excludes_the_records_own_id_whatever_its_form@sha256:959ac63a3d73bf771505fb473e27578b88d885a7188ed639a1a7eb54532fb775
- An index entry needs no hidden id token. — evidence: test:rust/crates/urzua-core/src/rules/index.rs#a_link_list_needs_no_id_token_and_identifies_by_file@sha256:16defb95e5d13367ae53457d37f8dc95d431bb005f111c28000834db248c2135
- A command regenerates a declared index from the records and a second run is a no-op. — evidence: test:rust/crates/urzua-cli/tests/index_freshness_integration.rs#index_regenerates_a_declared_index_and_a_second_run_is_a_no_op_end_to_end@sha256:7046d1eaf20f875864e3d251792ff2156105132c5865fe4e2d7ff1ba511f003e, code:rust/crates/urzua-cli/src/commands/index.rs
- The capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-index-rules-run.yaml

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Built: `index.freshness` reads multi-line entry formats (lines separated by `\n`) and formats with `{file}` and no `{id}`; `{title}` excludes the record's own id as well as a leading number; and `urzua index [--apply]` regenerates the declared indexes deterministically (a second run reports every artifact `current`). Exercised in this repository's own `.urzua/config.yaml`: `docs/adr/README.md` (an id-less link list) and `docs/specs/README.md` (a two-line entry) are generated and checked. Documented in `SPEC-2` and `SPEC-3`. The command is a new subcommand rather than a `urzua fix` action because fix repairs a record's own fields. `Status` is untouched. **Why:** the Done-when asks for a declared multi-line shape, an id-less entry, a regeneration command that is a no-op on a second run, and own-repo exercise. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #203: multi-line and link-list index formats, and `urzua index [--apply]` regeneration. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
