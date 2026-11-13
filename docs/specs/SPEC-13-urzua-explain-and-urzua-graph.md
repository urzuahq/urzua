---
Stable-Id: 01M6R3NK5DB4YPSWZ6MQV85PQ7
Version: '0.16'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/explain.rs@sha256:58f30c13d91ff0b4d4897789eb30192f843625c9f47ee1ac693c5fa491fb1f0f, code:rust/crates/urzua-cli/src/commands/graph.rs@sha256:379b00875a633e9d8e59f690e2a12ce3ed3b0d5635d533f0a8242580041bfd0a, code:rust/crates/urzua-core/src/graph.rs@sha256:03f7eed2679cc43971411a8874bb09e17c284a7f1b3a9c4accc15ace4cd518f6, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs@sha256:b1769908d1efac6b86ad75cdcd2646c5884f0570ee49ba13d58140dc68918be1
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

A path that names no file or directory in the working tree, matches no glob and that no record cites in `Realized-by` is `not-run` too (`urzua explain nosuchfile.rs`), and says so, because a typo otherwise reads as an ungoverned file with the whole corpus as its examined population. A path that is a rule id adds `use urzua explain --rule <id>`. A glob or directory argument that matches no tracked file and that no record cites (`urzua explain 'nonexist/**/*.rs'`) is `not-run` the same way, whether or not the config accepts its locator kind. A file that was deleted but is still cited by a record is a legitimate query and is explained as before.

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

## `urzua explain --rule <rule-id>`

What one rule is, as data (`ADR-96` amendment of 2026-11-07). The rule id is the value of `--rule`, and
only of `--rule`: a bare argument is always a path, even when it spells a rule id. A path and a rule id
cannot be told apart by their spelling (`status.delivery-agreement` is a legal file name, and a rule id
may be a path), so no heuristic is used and no argument is ever read as the other. Giving both a path and
`--rule`, or neither, is `not-run` with the usage message.

The report is `{"status": "ok", "rule": {...}}`. `rule` carries:

- `id`, `description`, `scope` (where the rule looks: `records`, `config`, `files`, `diff` or `run`) and
  `judges` (`SPEC-2`), exactly as `urzua rules` prints them: one rule table feeds both commands.
- `level`: the level this repository sets (`off`, `info`, `warn`, `error`); `off` when its configuration
  does not name the rule.
- `severity_floor`: the strictest floor among the rule's variants, the lowest severity a finding of that
  variant has whatever the level (`ADR-96`); omitted when no variant has one.
- `waivable`: `false` when at least one variant cannot be cleared by a rule-level waiver, `true`
  otherwise. The variants below say which.
- `options`: an object with every option the rule accepts, by name, and the value this repository sets, or
  `null` when it sets none. `{}` for a rule with no options.
- `actions`: the rule's `ADR-73` suggested-action variants, one entry per finding variant, as
  `urzua rules` prints them: `variant`, then `kind` and `op`, or `exempt` with the reason there is no
  action; `severity_floor` and `waivable: false` on a variant that has them.
- `population_unit`: what one candidate in the rule's population is (`record`, `field`, `path` and the
  other units `check` reports in `rules_executed`), and `companion_population_unit` for a rule that judges
  a second population under its id. A test compares each declared unit with the one `check` reports.
- `spec`: the spec that documents the rule. Every rule is documented by `SPEC-2`'s generated rule table.

An id that is not a rule this build ships is `not-run` with the id, the nearest valid id when one is close
(`did you mean 'status.delivery-agreement'?`) and a pointer to `urzua rules`. Nothing is read from the
corpus: the report is the build's rule table and the repository's configuration.

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
> | 2026-11-07 | Bumped to `0.11`. Added `urzua explain --rule <rule-id>`: the rule's scope, level here, severity floor and waivable flag, options with current values, `ADR-73` action variants, population unit and spec, from the same rule table `urzua rules` prints. An unknown id is `not-run` and names the nearest valid id. **Why:** maintainer decision of 2026-11-07, recorded in the `ADR-96` amendment of that date: `ADR-96` says `explain` prints the floor, and the maintainer chose to build the rule mode rather than redirect to `urzua rules`. It is a flag so a path and a rule id are never confused. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 4 locators pinned, 4 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/explain.rs@sha256:9cdffc6765ee, rust/crates/urzua-cli/src/commands/graph.rs@sha256:379b00875a63, rust/crates/urzua-core/src/graph.rs@sha256:7d43a868a856, rust/crates/urzua-cli/tests/record_types_own_config_integration.rs@sha256:7caa23975490 Reason: Pinned the files as they are now after checking each against the record it evidences: declaration.rs and config.yaml still declare pointer_fields and narrative_fields per type (MILE-90), config.yaml still requires Subject on specs with checks.yml gating it (MILE-91), document_model.rs, both section.rs files, schema_version.rs and config.yaml still read the declared identity, fields and sections (MILE-98), .git-hooks/pre-push, the Makefile and checks.yml still run urzua check (MILE-99), waiver.rs, rules/waiver.rs and waiver_stale_integration.rs still implement the waiver type SPEC-10 describes, audit.rs and the own-config and check integration tests still implement SPEC-11, new.rs, new_record.rs and new_integration.rs still implement SPEC-12, explain.rs, both graph.rs files and the own-config test still implement SPEC-13, both migrate.rs files and check_integration.rs still implement SPEC-14, and doctor.rs and its tests still implement SPEC-15. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Bumped to `0.12`. The `code:` and `test:` locators of `Realized-by` are now hash-pinned by `urzua reverify --pin-only` (the `Pinned` row above) and, where the record stated `Drift detected`, its `Embodiment` was recomputed (the tool-authored row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97` section 10: a claim is renewed only over pinned code and test evidence, so the corpus is pinned once before expiry is enabled. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` 7caa23975490 -> 44fea1626a54. Reason: Read the diff of each pinned file since the old pin and the claim still holds. record_types_own_config_integration.rs: the milestone fixture's Done means is now a list item instead of a prose sentence (the repository config checks Done means as items), and the helper that swaps in items replaces that item; no assertion changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.13`. The pin this spec's `Realized-by` holds on `record_types_own_config_integration.rs` was re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. **Why:** a revision-log entry that adds a structural change needs a `Version` bump (`revision-log.version-bump-required`). | **structural** |
> | 2026-11-08 | Bumped to `0.14`. `explain` on a path that is no file or directory, a glob or directory that matches no tracked file, and that no record cites, is `not-run` with the reason (a rule id given as a path also names `--rule`), where it was `ok` with an empty `governing_records` and the whole-corpus population. A deleted file a record still cites is explained as before. **Why:** the round-4 CLI review (S6): a typo read as "ungoverned" and claimed 586 records examined. | **substantive** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain.rs` 9cdffc6765ee -> 15d3bc36d944. Reason: explain.rs now errors (not-run, exit 2) on a path or glob that matches no tracked file and is cited by no record, which this spec now documents; the rest of explain and graph is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain.rs` 15d3bc36d944 -> 58f30c13d91f, `rust/crates/urzua-core/src/graph.rs` 7d43a868a856 -> 03f7eed2679c. Reason: Read the diff of each re-pinned file against main. cli/src/commands/explain.rs: Notice struct literals gained detail: None; no behaviour change; core/src/graph.rs: Notice struct literals gained detail: None; no behaviour change. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.15`. The pins this spec's `Realized-by` holds on files this change touched were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. | **structural** |
> | 2026-11-09 | Bumped to `0.16`. Re-pin rows in this revision log were reworded to name who re-verified, matching the tool's wording since 2026-11-09; hashes, reasons and dates are unchanged. **Why:** Maintainer decision, 2026-11-09. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` 44fea1626a54 -> b1769908d1ef. Reason: record_types_own_config_integration.rs: the fixture takes the repository's config through `common::own_config_for_fixture`, which drops the `drift_baseline` line a throwaway repository cannot resolve; the assertions are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
