---
Stable-Id: 01M6R3NK5DB4YPSWZ6MQV85PQ7
Version: '0.33'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/explain.rs@sha256:e4615ab46a7b6b0a219a9a88a92a7f2141ead75ec1c7d567a18280e2a808745d, code:rust/crates/urzua-cli/src/commands/graph.rs@sha256:23320e0f0346cec76142453f0f085dacd5a2df4440a587fa829a3c1f81ddd2e5, code:rust/crates/urzua-core/src/graph.rs@sha256:c950cb3add43fd918039433c90b10764914c3724658e2dcad3eec08e8fb578a4, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs@sha256:c54ac806e2c9b2f02f51bb222849fabc30ad3f80affb2d15bfd653064798688c, code:rust/crates/urzua-cli/src/commands/explain_types.rs@sha256:3454d195724916a3beb4efbfe748b7c99fc982258a3244f2234c81cf94641228, code:rust/crates/urzua-cli/src/commands/config_lines.rs@sha256:e5886132ed49cda81d59ff563f254557b246a58e7b7933ee35b93817ed64eb0b, test:rust/crates/urzua-cli/tests/explain_types_integration.rs@sha256:19478698a5b3b05af76840dcffea819554ef01e17397c62174c0e1f85bcf6c47
Author: beauwilliams
Subject: '`urzua explain`/`urzua graph` -- read-only relationship queries over already-parsed data.'
Implements: ADR-24, ADR-44
Parent: SPEC-1
---
# SPEC-13 — `urzua explain` and `urzua graph`

## Purpose

Two read-only relationship queries over data every other rule already parses — neither needs a new
schema field. `explain` needs no config change either; `graph` reads `pointer_fields`/
`narrative_fields` and the declared `relation_pairs` from existing config (MILE-90/ADR-44, MILE-259), not a new key. Bundled into one spec
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

## `urzua explain --types` and `urzua explain --type <name>`

What the configuration declares for its record types, read from the configuration file and nothing
else (`MILE-329`). The report shows declarations; where it shows a value that is not a copy of one,
the value comes from the function the engine itself uses for that decision. Types are listed in name
order, not declaration order; a configuration with no record type prints `types` as an empty list and
no `next_action`. A name the configuration does not declare exits 2 with a `not-run` report naming
the declared types, or saying that none is declared. `--full` goes with `--types` only:
`--type <name> --full` exits 2 with `urzua explain --type <name>` as its `next_action`, and `--full`
alone exits 2 with `urzua explain --types --full`.

`urzua explain --types` prints a summary: per type `name`, `at`, `prefix`, `dirs`, `cannot_create`
when `urzua new` refuses the type, `required` and `optional` field names, `states` (`values` and
`initial`) and `references` (each reference field with the types it may name, `"any"`, or `none`);
then, only for a type that declares each, `conditions` (the `declared: when ...` sentence of each
`statuses.require` and `statuses.forbid` entry, the same sentences `--type` prints), `gates` (each
`authority.gates` entry naming the type as its own entry, `to` and `by` as declared, in declaration
order, no logins, no combination rule), `field_values` (each field that
declares values, with them), `revision_log_classes` (`revision_log.classes`) and `checklist` (the
`statuses.delivery.items` section; `categories`, the evidence categories the engine accepts before a
locator's path; `tiers`, for each category a tier under `embodiment.tiers` declares, that tier's state,
a category absent from it having no tier declared and nothing said of whether it may be cited; and
`resolving_tiers`); then `path_gates`, the path gates as declared, without the note. A declared value
is printed bare; a default keeps its marker object. Each value is the one `--type` prints for the same
declaration, not a second reading of the configuration. The summary prints each array of scalars and
each object whose values are all scalars or arrays of scalars on one line, a layout of the same JSON;
`--full`, `--type` and every other command print one element per line. Its `next_action` is
`urzua explain --type <name>`.

The summary carries one fixed `note` (when a type is declared): these are declarations, whether a gate
or a condition is acted on depends on what the configuration enables, which `--types --full` and
`urzua check` show, and `--type <name>` says what counts as a missing value for a condition.

`urzua explain --types --full` prints every declaration of every type, `rules_set_in_configuration`
(each rule as written under `rules:`, in written order, with its level and line, and a fixed note),
`path_gates`, and `repository`: every declared top-level key other than `record_types` and `rules`,
each as `{value, at}`. `urzua explain --type <name>` prints the same detail for one type and the
note's sentence on enforcement.

Per type in detail: `name`, `at`, `create_command` or `cannot_create`, `prefix`, `dirs`, `template`
(the conventional path, marked as a default, with `readable_when_run`: whether `urzua new`'s own
read of that file, `new::read_template`, returned text when the command ran), `header_fields` (each with `required`, `values` from `field_values`, `pointer`, `narrative`, `roles`
from `relation_fields`), `states`, `conditions`, `checklist`, `gates`, `aliases`. `create_command` (shell-quoted) is
printed when `urzua new`'s own check of the configuration accepts the type (`header_shape`, and an
identity pattern that compiles); the command can still be refused for the given title, because the
identity pattern does not read back the name built from the real number and title, or by a path gate,
and `urzua new` says why. `cannot_create` carries that check's reason. Each header field that a
relation pair declares as its forward or inverse side, where the pair's `types` (if any) name the
type, lists that pair under `relation_pairs` as declared (`name`, `forward`, `inverse`, `types`,
`target_types`, `at`). A reference field is a `pointer_fields` entry
or a field the engine's target constraints name for the type (a relation pair's forward field); its
`pointer` gives the field's own `targets` with their line when it declares any, each relation pair
whose forward side it is and that names `target_types`, with that line and the pair's `name`, under
`pair_target_types`,
and `effective_targets` from the engine's target constraints: the types the field may name, `"any"`
when nothing constrains it, or `{"value": "none", "note": ...}` when no type satisfies every
declaration. `states` holds the status lists, `initial`, `new_writes` (the status `urzua new`
writes, from the function `urzua new` uses; absent when it writes none or refuses the type),
`transitions` in declaration order, every other `statuses` key raw, and `expiry`: the claim periods
the claim-expiry rule's setting declares for the type, with that setting's level as written.
`conditions` gives each `statuses.require` and `statuses.forbid` entry with its `when`,
`when_field` and `fields` keys, a `sentence` worded as a declaration (`declared: when ...`), and for a
requirement `counts_as_missing`: the type's `pending` and `placeholders` with their lines, or, where
the type declares none, a marker that the engine's built-in list applies. `gates` lists only the
`authority` gates declared for the type by name (`type: <name>`). `checklist` is printed for a type
that declares `statuses.delivery.items`: the items `section` and `evidence_marker` with their lines;
`evidence_categories`, the categories the engine accepts before a locator's path
(`locator::CATEGORIES`); `evidence_tiers`, for each category the first `embodiment.tiers` entry whose
`evidence` names it, with its state and its line, beside `resolving_tiers` and not joined to it (a configuration that declares no `embodiment` shows the engine's built-in tiers marked
as a default, and a category no tier declares carries `tier: null` and a `note` saying that no entry of
`embodiment.tiers` declares it); a `note` that `urzua check` says which items of a record resolve
(the engine also weighs `kind.cited_records` and whether a locator exists, which no join of tiers
states); `resolving_tiers`, as
declared under `items`, else the type's `delivered_tiers` marked as a default, the value the engine's
own `DeliveryDecl::resolving_tiers` returns. It states what is declared and describes no resolution
rule and no form of a locator: which files an item's locators must name, and when a tier counts, are
decided by the rule that reads the clause, and the engine states the `<category>:` form in no single
place a report could copy it from. `aliases` lists each YAML alias
under the type with the line of its anchor (`defined_at`), once per alias. Every other declared key
of the type is printed raw under its own name as `{value, at}`; relation pairs appear as declared
under `repository`.

`path_gates` lists every `authority` gate declared by path, once, as declared (`paths`, `by`, `at`);
in `--full` it carries a fixed note: a path gate declares who may change paths matching its patterns,
whatever the type, and `urzua new`, the edit hook and the pull request check act on it where they are
enabled. Path gates are not attributed to types.

The report does not say which rules apply to a type, nor whether a declared condition is enforced.
Which records a rule judges is decided inside each rule's code, and a mapping kept outside the rules
drifts from it. `rules_set_in_configuration` lists what `rules:` says, nothing more: rules on by
default or by a declaration do not appear there; `urzua check` reports every rule that ran and how
many records each examined, and with this section answers whether a declaration is enforced.

Every entry read from the configuration carries `at`, its key path and line (`path:line`). Lines come
from a second pass over the YAML text: block keys and items, the first level of a single-line flow
mapping, and each item of a flow list (also over several lines) are placed exactly; a key inside a
multi-line flow mapping or an aliased value is given its nearest enclosing key's line. Nothing the
configuration does not declare is printed, with these exceptions, each marked `"default": true` with
a `source` and no `at`: `new_writes`, printed beside a declared `initial` so a mismatch between them
shows
(`urzua_new_scaffold`); the identifier prefix when `prefix` is not declared, from `urzua new`'s own
prefix function (`type_name_uppercased`); the template path `urzua new` reads, with whether that file
is read by `urzua new` at the time of the run (`conventional_template_path`, `readable_when_run`); the evidence tiers and
the resolving tiers of a checklist when the configuration declares none (`built_in_embodiment_tiers`,
`delivered_tiers`); and the built-in pending and placeholder lists in `counts_as_missing`
(`built_in_list`). Empty lists, empty objects and absent settings are omitted. The only prose is a
condition's `sentence`, the reason in `cannot_create` and the fixed notes.

## `urzua graph`

Every edge named by a type's config-declared `pointer_fields` ∪ `narrative_fields` (MILE-90/ADR-44),
plus the forward and inverse fields of every declared `relation_pairs` entry the record's type carries
and declares (MILE-259; tagged `pointer`, and listed once when a pair field is also a pointer field),
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
`narrative_fields` and `relation_pairs` from config (MILE-90/ADR-44, MILE-259); `explain` still requires no config change.

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
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` 44fea1626a54 -> dc29b51bde5a. Reason: Read the diff of each pinned test file since the previous pin. The repo() fixture helper now also copies the repository's .urzua/templates into the fixture, because the own config declares sections.order from templates; no assertion or test body changed; the claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Bumped to `0.17`. Evidence pins re-baselined for this change (the `Re-baselined` row above); the specification text is unchanged. **Why:** a revision-log entry that adds a structural change bumps `Version`. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain.rs` 58f30c13d91f -> b9d38e5af3c0, `rust/crates/urzua-cli/src/commands/graph.rs` 379b00875a63 -> 1f510a65fa59. Reason: Read the diff of explain.rs and graph.rs against main. Each run_inner became a pub(crate) build() that returns the report, and run() prints it with the same emit(); the report, its fields and the output are unchanged, and the MCP server calls build(), so the claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Bumped to `0.18`. `explain` and `graph` each build their report in a `build()` function that `run()` prints and the MCP server (`SPEC-26`) also calls; the report and its output are unchanged; the evidence pin is re-baselined in the row above. **Why:** `MILE-29`. | **substantive** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain.rs` b9d38e5af3c0 -> 418e543d4466. Reason: Pinned file(s) changed by the symlinked-record refusal: explain.rs: `explain --rule` reports the default level of a rule that is on by default and undeclared. The record's claim is unaffected by these changes. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Bumped to `0.19`. A pinned evidence file was re-baselined for the symlinked-record refusal (`ADR-89` amendment of 2026-11-14); this spec's text is unchanged. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain.rs` 418e543d4466 -> a199dc863fdb. Reason: explain.rs makes a mistyped --rule id name urzua explain --rule <nearest> as its next action; the record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Bumped to `0.20`. The pins re-baselined above follow code changed for `MILE-266` and `MILE-267` (printed next actions that run as printed, a revision log in every built-in template); nothing this spec states changes. **Why:** the re-pin rows are structural and `revision-log.version-bump-required` asks for a bump. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/graph.rs` 03f7eed2679c -> 509c4a10a5da. Reason: graph.rs only gains rejected_values and waiver_fields in a test fixture's RecordTypeConfig literal. The claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Bumped to `0.21`. Re-pinned the evidence `MILE-233` changed (the rule vocabulary read from configuration); what this spec states is unchanged. **Why:** a re-pinned versioned spec carries a version above main's. | **structural** |
> | 2026-11-16 | Bumped to `0.22`. Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/graph.rs` 509c4a10a5da -> 02ace1051d4b. Reason: graph.rs: the only change on this branch is one added `hooks: Default::default()` field in a Config struct literal, needed by the new optional hooks key, merged with main's own change to the file; behaviour this record cites is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` dc29b51bde5a -> 95790a97cb28. Reason: the doctor test in record_types_own_config_integration.rs expects the message for a declared hook that is missing, since this repository's config declares its hooks; no other test in the file changed **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` 95790a97cb28 -> a9004c5b0c23. Reason: the doctor test in record_types_own_config_integration.rs expects not-pointed at .git-hooks with the fix urzua hooks install, since this repository's config names hooks.path; no other test in the file changed **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` a9004c5b0c23 -> 459b7bc6fe09. Reason: the doctor test in record_types_own_config_integration.rs expects not-pointed at .git-hooks with the fix urzua hooks install, since this repository's config names hooks.path; no other test in the file changed **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` 459b7bc6fe09 -> 95790a97cb28. Reason: record_types_own_config_integration.rs is back to its content on main: the doctor test again expects declared-missing, since this repository's config does not name hooks.path until a release reads it **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/graph.rs` 02ace1051d4b -> 9b36fa2ee136. Reason: Adds declared target types for reference fields (MILE-259). config.rs: a pointer_fields entry may be {name, targets} (the typed parse keeps only names), Config gains a skipped pointer_targets map, RelationPair gains optional target_types, a new load error InvalidPointerTargets; every other key unchanged. rules: new rule pointer.target-type (catalog.rs, pointer.rs, table.rs, default-on at error, not-enabled when nothing is declared); relation.rs cross-type-disallowed defers to it for a claim outside target_types. graph.rs lists relation-pair edges. fix.rs, migrate.rs, property.rs, type_scope.rs, sequence.rs: test fixtures add the new fields only. check_integration.rs: the header-none fixture declares a target list so the new rule is exercised. SPEC-3 documents the keys. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Bumped to `0.23`. The `urzua graph` section is corrected to include relation-pair edges: the forward and inverse fields of every declared `relation_pairs` entry a record's type carries are edges, tagged `pointer`. **Why:** `MILE-259` added these edges in `graph.rs`, and the re-pin rows of 2026-11-16 and 2026-11-17 on this spec said the claim still held, which was untrue for this section until this correction. | **substantive** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain.rs` a199dc863fdb -> d87eefb9ff72, `rust/crates/urzua-core/src/graph.rs` 02ace1051d4b -> 46d69af1dd1c. Reason: explain.rs: explain --rule reports the rule's effective level, which equals the old reading for every rule but a default-on rule whose default depends on the configuration; graph.rs: a test fixture's RecordTypeConfig literal gains field_values: None; no behaviour changed **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Pin recomputed on merge (attested by beauwilliams, identity: gh-verified, actor: agent): both merged sides re-pinned, `rust/crates/urzua-core/src/graph.rs` (Realized-by) 46d69af1dd1c and 9b36fa2ee136 -> 16a38ffef9e7; the new hash is the merged file's, which is git's automatic merge of the two re-pinned commits. No person wrote a reason for the merged state. **Why:** `urzua fix --conflicts` resolved a conflict that lay only in re-pin lines; a reviewer of the pull request should confirm the merged pin. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` 95790a97cb28 -> c54ac806e2c9. Reason: record_types_own_config_integration.rs: the doctor test expects the not-pointed message now that this repository's config names its hook directory; no other test changed. The claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/graph.rs` 16a38ffef9e7 -> 36dba22b9492. Reason: rust/crates/urzua-core/src/graph.rs: test fixtures that build a Config gain the new declared_status_entries field; no production code changed **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Bumped to `0.24`. Adds the section `urzua explain --types` and `urzua explain --type <name>`: what the configuration declares per record type, each entry with its key path and line, the two marked defaults, and the three rule lists. **Why:** an agent new to a repository could not learn its record model from the tool without reading the configuration file. | **substantive** |
> | 2026-11-17 | Bumped to `0.25`. `urzua explain --types` prints a summary with `--full` for every declaration and a `repository` section; every undeclared-rendering key is printed raw; a requirement carries `counts_as_missing`; four marked non-declared entries replace two; transitions keep declaration order; the per-type field list is `header_fields`; the initial-state text says what is declared. **Why:** review of the first version found declarations left out and a report too long for a session start; the summary default, the four marked entries and the agent-only session-start line were decided by the orchestrator and accepted by the maintainer on 2026-11-17: "The three decisions you made make sense to me. I'm happy with your decisions". | **substantive** |
> | 2026-11-17 | Bumped to `0.26`. The `urzua explain --types` section: a type's rules are read from the rule table and include rules declared outside the type's keys; expiry is shown only while its rule is in effect; a reference field reports its own targets, each pair's `target_types` and the effective result, with `"any"` for none; an undeclared initial state says no start status is required and names what `urzua new` writes; the summary keeps default markers and counts conditions by whether their rule is on; `--full` goes with `--types` only; types are in name order; aliases name their anchor's line; the built-in pending and placeholder lists are marked; `Realized-by` cites `explain_types.rs`, `config_lines.rs` and `explain_types_integration.rs`. **Why:** review of 2026-11-17 found the command stated six things untruly; each is corrected. | **substantive** |
> | 2026-11-17 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/explain_types.rs@sha256:08015e93ad9d, rust/crates/urzua-cli/src/commands/config_lines.rs@sha256:e73c38775722, rust/crates/urzua-cli/tests/explain_types_integration.rs@sha256:6ceceef0feb2 Reason: explain_types.rs builds the explain --types and --type reports this spec's section describes; config_lines.rs gives each entry its key path and line; explain_types_integration.rs tests each stated behaviour **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-17 | Bumped to `0.27`. One type's rules are now `rules.from_declarations`: only rules tied to a declaration of the type or to an entry of a rule's setting naming it, each with `declared_by`, base-only rules marked; repository-wide rules move to `repository.rules` under `--types --full`, and each type points there in a fixed sentence. `new_writes` is read from the function `urzua new` uses and is absent when it writes no status; an empty target intersection reads `none`; gate paths compare with case folded; `--type --full` names the command to run. **Why:** review of 2026-11-17 showed that which rules examine a type cannot be derived from the rule table; the narrower claim was decided by the orchestrator on 2026-11-17, pending maintainer review. | **substantive** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain_types.rs` 08015e93ad9d -> 6be486c4a397, `rust/crates/urzua-cli/tests/explain_types_integration.rs` 6ceceef0feb2 -> e3ff880de76a. Reason: explain_types.rs: one type's rules are those tied to its declarations or to a rule setting naming it, repository-wide rules move to repository.rules, new_writes comes from scaffold_status, an empty target intersection reads none, gate paths fold case; explain_types_integration.rs: tests for each **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Bumped to `0.28`. The report no longer says which rules apply to a type: per-type rule lists, rule ties and the repository-wide rule list are removed, and a paragraph says why (each rule decides its own population). `--types --full` lists `rules_set_in_configuration` as written, with a note that rules on by default or by declaration do not appear and `urzua check` reports what ran. Expiry shows the periods and the setting's level as written. A type `urzua new` refuses shows `cannot_create` with its reason instead of `create` and `new_writes`. Path gates use the engine's matcher, with `may_cover` where a glob cannot be decided. A pair's forward field outside `pointer_fields` is shown as a reference. **Why:** review of 2026-11-17 found the rule ties false three times; the cut was decided by the orchestrator on 2026-11-17, pending maintainer review. | **substantive** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain_types.rs` 6be486c4a397 -> 83467de65c2d, `rust/crates/urzua-cli/src/commands/config_lines.rs` e73c38775722 -> 7e2b16324972, `rust/crates/urzua-cli/tests/explain_types_integration.rs` e3ff880de76a -> 0addf2725ab1. Reason: explain_types.rs: per-type rule ties are removed, --types --full lists the rules set as written, expiry shows periods and the level as written, a type urzua new refuses shows cannot_create, path gates use the engine's matcher, a pair's forward field is a reference; config_lines.rs: an index under a quoted key falls back to the right parent; explain_types_integration.rs: tests follow **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Bumped to `0.29`. Path gates are no longer attributed to types: per type `gates` lists only gates declared by type name, and `path_gates` lists every path gate once, as declared, with a fixed note; `create` becomes `create_command`, printed when `urzua new`'s own configuration check (header shape and identity pattern) accepts the type, with a sentence that the title or a path gate can still refuse it; conditions carry no rule id and their sentence reads `declared: ...`; `--type` output repeats the enforcement sentence; `rules_set_in_configuration` keeps the written order; each flow-list item has its own line; relation pairs appear only under `repository`; reference fields and every default come from the engine's own functions. **Why:** review of 2026-11-17 found the three computed parts still contradicting the engine; showing declarations only was decided by the orchestrator on 2026-11-17, pending maintainer review. | **substantive** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain_types.rs` 83467de65c2d -> f2c2e4ac96a5, `rust/crates/urzua-cli/src/commands/config_lines.rs` 7e2b16324972 -> e5886132ed49, `rust/crates/urzua-cli/tests/explain_types_integration.rs` 0addf2725ab1 -> 6fc69e681902. Reason: explain_types.rs: per type only gates declared by type name, path gates once as declared, create_command from urzua new's own check, conditions worded as declarations without a rule id, rules set in written order, reference fields from the engine's target constraints, required_of_records from has_transitions; config_lines.rs: each flow-list item gets its own line; explain_types_integration.rs: tests follow **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Bumped to `0.30`. An undeclared `initial` is no longer printed (its `required_of_records` could only be false); `create_command` is shell-quoted and can be refused for the title because the identity pattern is fitted to the real number and title by `urzua new`; each header field lists the relation pairs that declare it; `pair_target_types` names its pair; the summary's `path_gates` has no note, and the note in `--full` is worded as a declaration. **Why:** review of 2026-11-17; the identity-pattern fit returns to `urzua new` after numbering, as before this change. | **substantive** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain_types.rs` f2c2e4ac96a5 -> 8d269478bbb2, `rust/crates/urzua-cli/tests/explain_types_integration.rs` 6fc69e681902 -> 22004fd62300. Reason: explain_types.rs: create_command is shell-quoted, an undeclared initial is not printed, each header field lists the relation pairs declaring it, pair_target_types names its pair, the summary's path_gates drops the note, the note reads as a declaration; explain_types_integration.rs: tests follow, including six identity-pattern rows run through urzua new **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Bumped to `0.31`. The `urzua explain --types` summary carries, for a type that declares each, `gated` (group per status, from the gates naming the type), `field_values`, `conditions` as the declared sentences in place of a count, `revision_log_classes` and a `checklist` marker; `--type` carries `checklist` with each evidence category, the tier a configuration declares for it, and whether that tier is listed in `resolving_tiers`, from the engine's category list and locator grammar; the summary prints scalar arrays on one line; `template` carries `exists_when_run` and is printed whether or not the file exists. **Why:** `MILE-340`; an adopting repository reported that the summary omitted the declarations it had got wrong; decided by the builder on 2026-11-17, pending maintainer review. | **substantive** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain_types.rs` 8d269478bbb2 -> f74589ee7177, `rust/crates/urzua-cli/tests/explain_types_integration.rs` 22004fd62300 -> 0da9da3db5c7. Reason: explain_types.rs: the summary gains gated, field_values, revision_log_classes and a checklist marker and prints condition sentences in place of their count, each from a helper shared with the --type detail (condition_rules, gated, checklist, evidence_tiers, resolving_tiers); --type gains checklist with the engine's evidence categories and locator grammar; template is printed with exists_when_run whether or not the file exists; run() prints the summary through emit_compact. explain_types_integration.rs: updated for the condition sentences and the template entry, and new tests for each addition present and absent, summary equal to detail, line numbers, defaults, formatting and sizes (plus the checklist.yaml fixture). The claims this spec states about explain --types and --type still hold and the spec text is updated in this change. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Bumped to `0.32`. The summary prints each object whose values are all scalars or arrays of scalars on one line, beside the arrays of scalars, and its `checklist` carries `categories` and `tiers` in place of `evidence` and `no_tier_declares`; `--type` prints a category no tier declares as `tier: null` with a `note`, and no longer prints a sentence on how a locator is written, because the engine's grammar does not contain the `<category>:` form. **Why:** `MILE-340`; the maintainer chose the layout of the summary and asked that nothing retype engine behaviour, 2026-11-17; the choice of key names is decided by the builder on 2026-11-17, pending maintainer review. | **substantive** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain_types.rs` f74589ee7177 -> d2662dabb8c4, `rust/crates/urzua-cli/tests/explain_types_integration.rs` 0da9da3db5c7 -> 4995c87b85e8. Reason: explain_types.rs: the summary's checklist carries categories and tiers (no evidence or no_tier_declares), --type prints a category no tier declares as tier null with a note and no locator_form sentence; the compact summary printing and its tests are in compact_json.rs and main.rs. explain_types_integration.rs: tests for the summary equal to its pretty form, flat objects on one line, --full, --type and graph as before but for exists_when_run, the checklist keys; new golden fixtures. The claims this spec states about explain --types and --type still hold; the spec text is updated in this change. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/explain_types_integration.rs` 4995c87b85e8 -> 26299019603c. Reason: explain_types_integration.rs: a test that the status a real urzua new writes equals new_writes and the first of the summary's declared initial (or the first of the values when none is declared), run against the starting-status rule of the merged main; nothing else in the file changed. The claims this spec states still hold. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Bumped to `0.33`. The summary's `gated` becomes `gates`: each `authority.gates` entry naming the type as its own `{to, by}` entry in declaration order, so nothing reads as alternatives; the summary carries one fixed `note` that these are declarations; `--type` no longer prints `in_resolving_tiers` and its `checklist` carries a `note` that `urzua check` says which items resolve; `template` carries `readable_when_run` from `urzua new`'s own template read in place of `exists_when_run`. **Why:** `MILE-340`; review of 2026-11-17: the engine judges each gate on its own, a join of tiers and resolving tiers reads as "this category counts" while the engine also weighs the item kind and the locator, and a template `urzua new` cannot read is not one it uses; decided by the builder on 2026-11-17, pending maintainer review. | **substantive** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain_types.rs` d2662dabb8c4 -> e775f8637d91, `rust/crates/urzua-cli/tests/explain_types_integration.rs` 26299019603c -> 6741d0cd3af0. Reason: explain_types.rs: the summary's gates are one entry per gate naming the type (replacing gated), the summary carries one fixed note, --type's checklist drops in_resolving_tiers and carries a note, template reports readable_when_run from new::read_template; explain_types_integration.rs and the golden fixtures follow, with tests for each and for an unreadable template. The claims this spec states still hold and the spec text is updated in this change (0.33). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-18 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/graph.rs` 1f510a65fa59 -> 23320e0f0346, `rust/crates/urzua-core/src/graph.rs` 36dba22b9492 -> c950cb3add43, `rust/crates/urzua-cli/tests/explain_types_integration.rs` 22004fd62300 -> 7b284f368f90. Reason: commands/graph.rs: the edge computation moves into edges_of, shared with export; the graph report is unchanged; core graph.rs: test and fixture Config literals gain the part_of and export fields; no production behavior changed; explain_types_integration.rs: the test that every top-level key is reported under repository leaves out the optional export key, which this repository does not declare; no other test changed **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-18 | Pin recomputed on merge (attested by beauwilliams, identity: gh-verified, actor: agent): both merged sides re-pinned, `rust/crates/urzua-cli/tests/explain_types_integration.rs` (Realized-by) 6741d0cd3af0 and 7b284f368f90 -> 19478698a5b3; the new hash is the merged file's, which is git's automatic merge of the two re-pinned commits. No person wrote a reason for the merged state. **Why:** `urzua fix --conflicts` resolved a conflict that lay only in re-pin lines; a reviewer of the pull request should confirm the merged pin. | **evidence** |
> | 2026-11-19 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain.rs` d87eefb9ff72 -> b1e7860f65e2. Reason: explain.rs adds the declared types list to explain --rule's output, omitted when unset **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-22 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain.rs` b1e7860f65e2 -> 722b89bf4452. Reason: explain.rs: the scope-resolver match arm reads Scope::Symbol as a struct variant ({ .. }) now that a symbol scope carries an optional member kind; what explain prints and how it resolves are unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-24 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/explain.rs` 722b89bf4452 -> e4615ab46a7b, `rust/crates/urzua-cli/src/commands/explain_types.rs` e775f8637d91 -> 3454d1957249. Reason: Read the diff of rust/crates/urzua-cli/src/commands/explain.rs, rust/crates/urzua-cli/src/commands/explain_types.rs against main. rust/crates/urzua-cli/src/commands/explain.rs: Failure now carries a non-optional kind and has no conversion from a bare message, so a refusal states its kind where it is made; an unknown rule id, a path that is not in the repository and a malformed locator are tagged usage; also rust/crates/urzua-cli/src/commands/explain_types.rs: Failure now carries a non-optional kind and has no conversion from a bare message, so a refusal states its kind where it is made; an unknown type and --full without --types are usage, an unreadable config is config. The claims each record cites are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
