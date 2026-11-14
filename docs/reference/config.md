# The config file

Urzua reads one file, `.urzua/config.yaml`, found by walking up from the current directory. Pass
`--config <path>` to any command to use another. The file is YAML.

An unknown key in the keys listed here is a load error: the command exits 2 with a `not-run` report
naming it. A repository's config is therefore either understood in full or refused, never partly
ignored. A key written twice under `rules:` or `record_types:` is refused the same way, naming the key.
A leading byte order mark is skipped. `urzua doctor` reports the same problems on demand.

This page lists every top-level key and every key of a record type. For the full semantics of a key,
the build-level specification is [`SPEC-3`](../specs/SPEC-3-configuration.md).

## A minimal config

```yaml
schema_version: 2
record_types:
  adr:
    dir: docs/adr
    prefix: ADR
    required_fields: [Status, Date, Author]
    known_fields: [Deciders, Stable-Id]
    header_shape: yaml-frontmatter
rules:
  header.required-fields: error
  filename.identity-pattern: warn
```

## Top-level keys

| Key | Type | What it does |
|---|---|---|
| `schema_version` | number | Required. `2` is the version this build writes. A config at the previous version is read and upgraded in memory (`check` reports `config.schema-version-outdated` if you enable it); `urzua migrate config` rewrites it in place. A version outside what this build reads is refused. |
| `record_types` | map | Required. Record types by name. See below. |
| `rules` | map | Which rules run, and at what level. See below. Absent or empty means no rule runs. |
| `fail_on` | `error` or `warning` | The lowest finding severity that makes `check` exit 1. Default `error`. The `--fail-on` flag overrides it for one run. |
| `embodiment` | map | The vocabulary of evidence states. See [waivers and drift](waivers-and-drift.md). |
| `locator_resolvers` | list | How a `path#Symbol` evidence pin finds the item it pins, per file type. A line-range pin (`#L10-L40`) needs none. |
| `relation_pairs` | list | Pairs of fields declared as inverses of each other, optionally a sequencing relation. |
| `roadmap` | map | `pairs` (the sequencing pairs `urzua roadmap` orders by), `group_by` (fields to group by) and `work_types` (types the roadmap orders). |
| `new` | map | How `urzua new` numbers records. `check_open_prs` (default `false`) fetches pull request head refs to skip numbers open pull requests took; `pull_ref_patterns` (default `refs/pull/*/head`), `remote` (default `origin`), `fetch_timeout_seconds` (30), `lock_timeout_seconds` (10). |
| `discovery` | map | `include_untracked` (default `false`): also read untracked, non-ignored files directly under a record directory, so `check` sees a draft before `git add`. |
| `attestation` | map | Who a tool-written revision row names. `identity` is `gh-verified` (default) or `asserted`; `require_verified` (default `false`) refuses a run with no verified identity. |
| `harness` | map | The agent harness. See [the agent harness](agent-harness.md). |
| `authority` | map | Who may accept a record or change a protected path. Read by the harness guardrails and by the `diff.decider-required` rule. See `SPEC-3`, "Who may accept a record". |
| `override_log` | path | A committed log of editor-hook overrides, read by `override.unreviewed`. |
| `block_on` | `introduced` or `all` | Which errors the editor hook blocks on. Default `introduced`. |

## Rules

```yaml
rules:
  header.required-fields: error          # a bare level
  embodiment.consistency:                # a table, when the rule takes options
    level: warn
    drift_grace_days: 7
```

A level is `off`, `info`, `warn` or `error`. `off` is permitted and is reported in `check`'s
`rules_executed` as not enabled, never omitted. A rule that is absent does not run.

Options belong to one rule each, and an option on a rule that does not take it is a load error, so a
setting is never silently ignored. `level` is the one option every rule takes: it is the rule's
severity ceiling.

`scope: changed` is not general. It makes a rule judge only the records a `check --base` diff adds or
changes, and four of the 74 rules accept it:

- `embodiment.locator-promotion-candidate`
- `embodiment.locator-on-back-pointer-path`
- `embodiment.claim-expired`
- `embodiment.claim-renewal-unattested`

On any other rule it is a load error (`rule '<id>' does not take the option 'scope'`). The only value
is `changed`; omit the option to judge every record. On a plain `check` there is no diff, so such a
rule runs and examines nothing; `check --stdin-path` treats the proposed record as the changed one.
The candidates it skips are counted in the rule's population as `deferred_by_scope`, with one `info`
notice. `urzua explain --rule <id>` shows whether a rule has the option.

To see what a rule judges, its options and their current values, ask the binary:

```sh
urzua rules                              # every rule this build ships, with a description
urzua explain --rule embodiment.consistency
```

## Record types

```yaml
record_types:
  adr:
    dir: docs/adr
    ...
```

| Key | What it does |
|---|---|
| `dir` | Required. The directory records sit directly in. Nested directories belong to no type. |
| `extra_dirs` | More directories holding records of the same type. They share one id namespace with `dir`, and `new` numbers past the highest id in any of them. |
| `prefix` | The id prefix, for example `ADR`. Used in file names, ids and `new`. |
| `identity` | `{from: filename, pattern: <regex>}`. Overrides the default `TYPE-NNNN-slug.md` file name convention. The pattern must capture `prefix` and `number` as named groups (`(?P<prefix>...)`, `(?P<number>...)`). |
| `exclude_files` | File names directly in `dir` that are not records, such as `README.md`. |
| `header_shape` | `yaml-frontmatter` (default) or `none`. |
| `required_fields` | Fields every record must carry with a real value. `header.required-fields` and `field.quality` read it. |
| `known_fields` | The closed set of fields a record may carry besides `required_fields`. Declare it, even as `[]`, to make the field set a conscious choice. |
| `pointer_fields` | Fields that name other records, resolved by `pointer.resolution`. |
| `narrative_fields` | Fields that name other records without being a dependency. They are resolved like pointer fields and appear in `urzua graph` tagged `narrative`. |
| `spec` | The id of the spec that documents the type (`SPEC-16`). |
| `relation_fields` | Which field plays which role, for the tool: `status`, `embodiment_state`, `embodiment_locator`, `supersession`, `date`, `author`, `title`. `new` fills `date`, `author` and `title`. |
| `companion_fields` | Pairs of fields that must be declared together, such as `[[embodiment_state, embodiment_locator]]`. |
| `statuses` | The status vocabulary. See below. |
| `sections` | Required headings. See below. |
| `revision_log` | How the revision log is found and named. See below. |
| `role` | `suppression` makes this the waiver type. See [waivers and drift](waivers-and-drift.md). |
| `effective_requires` | For a suppression type: fields that must hold a real value before a waiver takes effect. |
| `permanent` | For a suppression type: where a waiver without `Expires` may take effect. |
| `fields` | `{from: prefix-lines}` (or `yaml-frontmatter`, or `none`): where a record's fields are read from. `header_shape` is the usual way to say it; `fields` exists for a corpus whose headers are bare `Key: value` lines. |

### `statuses`

| Key | What it does |
|---|---|
| `values` | The allowed statuses. |
| `success`, `failure` | Which values are a good or a bad outcome. |
| `implementable` | Statuses whose records code may legitimately keep citing. |
| `claim_retained` | Statuses that keep a record's evidence claim even though it is no longer current. |
| `initial` | Statuses a record may be filed in. |
| `transitions` | `{From: [To, ...]}`: the lawful moves. |
| `require`, `forbid` | Lists of `{when: [status, ...], fields: [field, ...]}`: fields that must, or must not, hold a value in those statuses. |
| `pending` | The type's placeholder vocabulary: whole values (`unknown`) and `{pattern: <regex>}` entries that count as missing. |
| `blocked`, `progress` | Statuses read by the derived `blocked` value and `sequence.status-agreement`. |
| `delivery` | How a status agrees with delivery evidence, including per-item evidence in a list-form "Done means" section (`items`). See `SPEC-3`, "Done-means items". |

### `sections`

```yaml
sections:
  from: headings
  depth: 2                 # 1 reads `##` only; 2 also reads `###` as nested sections
  required:
    - {name: Context}
    - {name: Decision, aliases: [Outcome]}
    - {name: Consequences, level: 3, under: Decision}
    - {name: Summary, shape: y-statement, marker: "**TL;DR**"}
    - {name: Options, shape: table-min-rows, min_rows: 2}
  exempt_statuses: [Rejected]
```

`name` is the heading text; `aliases` are other accepted headings; `level` (2 to 6, default 2) and
`under` ask for a nested heading; `shape` is `y-statement` (with an optional `marker`, and
`preamble: true` to find the block outside any section) or `table-min-rows` (with `min_rows`).
`exempt_statuses` lists statuses whose records are eligible but not examined. `items` (default
`false`) turns on item reading for the done-items rules; see `SPEC-3`. `section.required` and
`section.shape` read this.

Three more keys switch on structure rules for a type, each off by default:

| Key | Read by | What it does |
|---|---|---|
| `unique_headings: true` | `section.duplicate` | The same heading text twice at one level under one parent is an error. |
| `no_skipped_levels: true` | `section.level-skipped` | A section nested more than one level below the one above it is an error. Needs `depth` of at least 2. |
| `order: {from: template}` | `section.order` | A record's sections must follow the order of the headings in `.urzua/templates/<type>.md`. `template` is the only source. |

### `revision_log`

```yaml
revision_log:
  marker: "**Revision log**"   # a whole line; or `heading: "Revision log"` for a markdown heading
  required: true               # default true: `fix --apply` refuses a record with no log
  classes: [substantive, structural]
  class_column: Class
  date_column: Date
  tool_class: structural       # the class tool-written rows carry
```

Give exactly one of `marker` or `heading`. Omitted, the marker is `**Revision log**`. `version_bump:
{field, classes, initial}` names a field that must change when a row of a listed class is added
(read by `revision-log.version-bump-required`), and the value `new` starts it at.

## Choosing what to declare

Start from what `init` writes and add declarations as you need the rule that reads them:

| You want to catch | Declare |
|---|---|
| A missing or placeholder field | `required_fields` |
| A misspelled field | `known_fields` |
| A dangling reference | `pointer_fields` |
| A status typo or a bad status move | `statuses` with `values` and `transitions` |
| A record without its Context section | `sections.required` |
| A decision recorded as built but not | `embodiment` and `Realized-by` |
| A change to code with no record | the `diff.source-requires-record` rule with `source_paths` |

`urzua init` withholds rules that read a declaration it cannot infer, and says which in a
`rule-applicability` notice. Enable such a rule only after declaring what it reads: depending on the
rule, enabling it first is either a load error that names the missing declaration, or a rule that
examines nothing, which `check` shows with `examined: 0`.
`diff.source-requires-record` is a diff rule: it only judges a `check --base` run.
