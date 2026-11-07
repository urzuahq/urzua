---
Stable-Id: 01M6R3NK5DB4YPSWZ6MQV85PQ7
Version: '0.10'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/explain.rs, code:rust/crates/urzua-cli/src/commands/graph.rs, code:rust/crates/urzua-core/src/graph.rs, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs
Author: beauwilliams
Subject: '`urzua explain`/`urzua graph` -- read-only relationship queries over already-parsed data.'
Implements: ADR-24, ADR-44
Parent: SPEC-1
---
# SPEC-13 — `urzua explain` and `urzua graph`

## Purpose

Two read-only relationship queries over data every other rule already parses — neither needs a new
schema field. `explain` needs no config change either; `graph` reads `pointer_fields`/
`narrative_fields` from existing config (MILE-90/ADR-44), not a new key. Bundled into one spec
because they're the same feature area (queryable relationship data) answering two directions of the
same question: "which decisions govern this file" (`explain`) and "what does the whole graph look
like" (`graph`).

## `urzua explain <path>`

Every record whose `Realized-by` names `path` as evidence, with which category (`spec:`/`code:`/
`test:`) matched. Reuses ADR-18's Embodiment MVP's categorized-locator data directly — no new field.

`governing_records` entries carry `scope` when the record pins a symbol or line range within the
file (`ADR-74`). Given `urzua explain <path>#<scope>` the report also carries `scope`: `scope`,
`resolver` (for a symbol), `start` and `end` (1-based, inclusive), `text` (the pinned bytes),
`sha256` and `pin`, the locator to paste after `code:` in `Realized-by`. This is the authoring
route for a scoped pin, and it shows the resolved span because an early end (a `}` in a string)
cannot be detected by the engine. A scope that does not resolve, a file that is not UTF-8 and a
locator carrying an anchor are `not-run` with the reason.

A path that is absolute, climbs out of the repository with `..`, or resolves through a symlink to a file outside it is `not-run`, whether or not it exists, and nothing is read from it. The same confinement applies to every path the tool reads or writes on a name from the arguments or the configuration (`index`, `fix`, `migrate ids`, `refs refresh`, `init`).

Given a directory or glob (`urzua explain src` or `urzua explain 'src/**/*.rs'`, `BUG-176`) the
report carries `coverage`: `kind`, `files` (the tracked files it covers, sorted), `sha256` (the hash
of the sorted path and file-hash list, `SPEC-3`) and `pin`, the locator to paste after `code:`,
present only when `pinnable` lets that kind carry a pin. When `embodiment.locator-exists` does not
declare the kind in `locator_kinds` there is no `coverage` and a notice names the option; a glob
that matches no tracked file is `not-run` with the reason.

Exact-match only for `governing_records`: no glob, no directory prefix, no fuzzy match, so a record
citing `src` is found by querying `src` and not by querying a file below it. A corpus citing a file by a
different relative path than the one queried finds nothing. Coverage is only as complete as
`Realized-by` adoption — a record that genuinely governs a file but never had a `Realized-by` field
added is invisible to `explain`, the same limitation `embodiment.consistency` already has.

## `urzua graph`

Every edge named by a type's config-declared `pointer_fields` ∪ `narrative_fields` (MILE-90/ADR-44),
plus the still-hardcoded `Supersedes`/`Superseded-by` (its own mechanism, outside that axis), across
every record with a resolvable identity (`record_id`) — a record whose filename doesn't match its
type's configured prefix/number shape contributes no outgoing edges, the same precondition
`pointer_resolution`'s index-building already has. Config-driven, not a hardcoded field list — a
type's own `Parent`, or any other declared relationship field, appears for free the moment it's
declared, with no code change here.

Each edge is tagged:

- `kind: "pointer" | "narrative"` — the field's declared kind (MILE-90/ADR-44). Describes the edge's
  value shape (clean reference vs. prose-tolerant), not which rule resolved it: `Supersedes`/
  `Superseded-by` reports `pointer`, since its values are always clean references, even though
  `relation.supersession-reciprocity` (a separate mechanism) is what actually checks it.
- `dangling: bool` — the same condition `pointer.resolution` computes, exposed as an edge property
  instead of requiring a separate `check` run to notice.

No filtering (`--type`, `--format dot`) — dumps every edge.

## Population and notices

Both reports carry the population block `check` reports for a rule (`BUG-169`, `SPEC-1`'s "a check that
finds nothing must be distinguishable from one that ran on nothing"): `population` with `unit: "record"`,
`eligible` (every loaded record), `examined` (records whose header was read), `out_of_scope` (records of a
`header_shape: none` type, which have no header) and `unreadable` (records whose header could not be read).
When any record is unreadable a `warning` notice lists the records left out, since their relationships are
missing from the output. `status` stays `ok` and the exit code `0`: the query ran, and the population says
what it ran on. An empty `governing_records` or `edges` beside `eligible: 0` is an empty corpus, and beside
`unreadable: 1` is a corpus that could not be fully read.

## Shared implementation

Both reuse `record_id`/`extract_references`/`parse_realized_by` (`rules.rs`) rather than
reimplementing reference-parsing a third time; `graph` additionally reuses `build_normalized_index`,
the same normalized-lookup helper `pointer_resolution`/`narrative_field_stale` use (MILE-90 closed
BUG-11 here: this function's own index used to skip normalization, unlike `pointer_resolution`'s).
Both are stdout-JSON-always (ADR-23) and never write. `graph` reads `pointer_fields`/
`narrative_fields` from config (MILE-90/ADR-44); `explain` still requires no config change.

## What's deliberately not built

- **`explain` fuzzy/prefix matching** on `Realized-by` locators — real follow-up, not attempted.
- **`graph --type`/`--format dot` filtering** — additive later work once a real case needs a
  narrower or differently-shaped view than "every edge."

## References

- ADR-24 — the decision this spec details: both commands, the data they read, why neither needs a
  schema change.
- ADR-44 — `pointer_fields`/`narrative_fields` as config-declared per type (MILE-90); `graph`'s
  edge set and `kind` field are this decision's realization.
- ADR-18 — the categorized-locator model `explain` reads.
- ADR-23 — the stdout-JSON-always contract both commands follow.
- `rust/crates/urzua-core/src/rules.rs` — `record_id`, `extract_references`, `build_normalized_index`,
  reused by both/`graph`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec, bundling `explain` and `graph` since they're one feature area (queryable relationship data) answering two directions of the same question. **Why:** MILE-77 found both documented only as a single ADR while comparable-complexity command areas (`check`, `init`) had specs. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-09-09 | `urzua graph` is now config-driven (MILE-90/ADR-44): edges come from each type's declared `pointer_fields`/`narrative_fields`, not a hardcoded field list, and every edge gains a `kind` field. Fixed BUG-11 in the same pass (`graph`'s own index never normalized reference padding, unlike `pointer_resolution`'s). Added `Implements: ADR-44`. **Why:** this spec's own text previously claimed `graph` "requires no config change," which stopped being true the moment this shipped -- leaving it unstated would have been the exact "spec goes factually stale the moment a decision lands" gap this project's own history keeps flagging. | **substantive** |
> | 2026-11-05 | Bumped to `0.5`. `explain <path>#<scope>` resolves a scope and prints the span and its pin; `governing_records` entries name the scope a record pins. **Why:** `MILE-185` / `ADR-74` -- a person cannot reproduce a resolver by hand, so `explain` is the authoring route for a scoped pin. | **substantive** |
> | 2026-11-05 | Bumped to `0.6`. Added `Realized-by` naming the implementing code and the tests that exercise this spec: `explain_names_the_governing_record_and_graph_reports_kind_and_dangling_per_edge`. **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`; the tests run this repository's own `.urzua/config.yaml`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/commands/explain.rs, code:rust/crates/urzua-cli/src/commands/graph.rs, code:rust/crates/urzua-core/src/graph.rs, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Bumped to `0.7`. `graph` and `explain` report the `check` population block (`eligible`, `examined`, `out_of_scope`, `unreadable`) and a warning notice listing the records left out for an unreadable header; status stays `ok`. **Why:** `BUG-169`, maintainer decision 2026-11-05; an unreadable header used to drop its edges with no disclosure. | **substantive** |
> | 2026-11-05 | Status changed from Accepted to Shipped and bumped to `0.8`. **Why:** maintainer decision of 2026-11-05 to ship the specs whose implementation is verified; `Embodiment: Verified` with `Realized-by` naming the implementing code and tests. | **structural** |
> | 2026-11-05 | Bumped to `0.9`. `explain <directory-or-glob>` prints `coverage`: the kind, the covered files, the hash and the pin to paste, when the repository declares that locator kind; otherwise a notice names `locator_kinds`. **Why:** `BUG-176` -- a pin on a directory or glob needs the reviewer to see which files it covers. | **substantive** |
> | 2026-11-07 | Bumped to `0.10`. `explain` refuses a path that resolves outside the repository (absolute, `..`, or a symlink out), and the other file readers and writers share the check. **Why:** a Phase 0 hardening review found `explain '/etc/hostname#L1'` printing the file and a ready-to-paste pin. | **substantive** |
