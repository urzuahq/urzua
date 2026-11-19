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
| `export` | map | `link_prefix` (optional): text put before a record's repository-relative path to make the `link` each exported record carries. Must start with `https://`, `http://` or `/`. Absent, no record carries a `link`, and the board has no link to a record's file. |
| `new` | map | How `urzua new` numbers records. `check_open_prs` (default `false`) fetches pull request head refs to skip numbers open pull requests took; `pull_ref_patterns` (default `refs/pull/*/head`), `remote` (default `origin`), `fetch_timeout_seconds` (30), `lock_timeout_seconds` (10). |
| `discovery` | map | `include_untracked` (default `false`): also read untracked, non-ignored files directly under a record directory, so `check` sees a draft before `git add`. |
| `attestation` | map | Who a tool-written revision row names. `identity` is `gh-verified` (default) or `asserted`; `require_verified` (default `false`) refuses a run with no verified identity. `reason` sets the least a `--reason` must hold for `fix --rebaseline --apply` and `reverify --apply`: `min_words` (default `3`, runs holding a letter or digit) and `min_chars` (default `12`). |
| `harness` | map | The agent harness. See [the agent harness](agent-harness.md). `report_outside_writes` (default `false`) brings back the notice for an allowed write outside the repository. Its `limits` map sets the hook timeout and the input caps, each of which may be raised but never lowered below its default; see [limits](agent-harness.md#limits). |
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

`types` narrows a rule to the record types it judges (`ADR-104`). It is a list of declared record-type
names, accepted on any rule entry in the map form whose rule judges loaded records:

```yaml
rules:
  status.children-agreement:
    level: error
    types: [milestone]
  header.required-fields: error   # every type, as before
```

A record of a type the list does not name is not judged by that rule: a finding on it is not reported, and
its candidates are counted in the rule's population as `out_of_scope` and in `out_of_types`, present only
when non-zero. The rule still reads every record as context, so a parent is judged against all its
children and a reference against its target whatever their type. Omitted, the rule judges every type. A
name that is not a declared type is a load error naming the rule and the type, and so is an empty list. A
rule that judges the configuration, files that are not records, a diff's changed paths, the run itself or
a type's directory does not take the key (`rule '<id>' does not take the option 'types'`, with the
reason). A listed type the rule can never judge (a `header_shape: none` type for a rule that reads a
header, a type no `part_of: true` field targets for `status.children-agreement`, a type without the
`sections` key a section rule reads) loads, and `check` names every such entry in one
`rule-types-never-judged` warning notice. `types` holds the same way on `check --base`, the editor hook's
`check --stdin-path` and `fix`, composes with `scope: changed` and the rule's other options, and
`urzua explain --rule <id>` shows the declared list.

`citation.issue-exists` takes the structured option `issue_refs` (`snapshot`, `repository`, `forms`, and
optionally `fields` and `workers`). `workers` is how many pages of the issue listing `urzua refs refresh`
requests at once when GitHub names the last page: an integer from 1 to 16, default 4, anything else a
load error. A configuration without it behaves as it did before the key existed.

### Rule vocabulary

The words rules compare against are options, each defaulting to the list built in before it
(`SPEC-3`, "Rule vocabulary"). A declared list **replaces** the default, except `rejected_values`,
which **adds** to a built-in floor no declaration removes. An empty list, a blank or repeated value,
and a marker without its colon are load errors naming the key.

| Option | Rule | Default | A declared list |
|---|---|---|---|
| `implements_marker` | `embodiment.back-pointer-resolution` | `Implements:` | replaces |
| `verifies_marker` | `embodiment.back-pointer-resolution` | `Verifies:` | replaces |
| `comment_leaders` | `embodiment.back-pointer-resolution` | `<!--` `///` `//!` `//` `/*` `--` `#` `;` `*` | replaces |
| `trailing_comment_leaders` | `embodiment.back-pointer-resolution` | `<!--` `//` `/*` `--` `#` | replaces |
| `closing_verbs` | `claim.status-agreement` | `closes` `fixes` `resolves` | replaces |
| `rejected_values` | `override.unreviewed` | none | adds to blank, dashes, the placeholder words and `pending` |

The four back-pointer options are read by every rule and command that reads a back-pointer, so they
are declared once, on `embodiment.back-pointer-resolution`:

```yaml
rules:
  embodiment.back-pointer-resolution:
    level: error
    back_pointer_paths: [src]
    implements_marker: "Realises:"
```

Four more are declared in a record type's configuration: `statuses.placeholders`, a `y-statement` shape's `phrases`,
and, on the suppression type, `rejected_values` and `waiver_fields` (below).

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
| `pointer_fields` | Fields that name other records, resolved by `pointer.resolution`. An entry may be `{name: Parent, targets: [epic]}` to limit the record types the field may name; see below. |
| `field_values` | `{Field: [value, ...]}`: the values a declared field may hold, compared exactly; each element of a comma-separated or list value is checked. Read by `field.value-not-declared`, on by default at `error` once any type declares this key. Not for the status field (`statuses.values`) or a pointer or narrative field. |
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
| `rejected_values` | For a suppression type: whole values (compared without case) that never meet an `effective_requires` field or an `Expires`, added to the built-in placeholder words and `pending`, which cannot be removed. |
| `waiver_fields` | For a suppression type: `{reason: <field>, owner: <field>}`, the fields `urzua waive` writes a draft's reason and owner into; `Reason` and `Owner` by default. `Rule`, `Scope`, `Expires`, `Variant` and `Key-path` are read by name and cannot be renamed. |
| `fields` | `{from: prefix-lines}` (or `yaml-frontmatter`, or `none`): where a record's fields are read from. `header_shape` is the usual way to say it; `fields` exists for a corpus whose headers are bare `Key: value` lines. |

### Target types of a reference field

A `pointer_fields` entry is a field name or `{name, targets}`. With `targets`, a reference to an existing record of
any other type is a `pointer.target-type` error, on by default. A relation pair limits its forward field the same way
with `target_types`:

```yaml
record_types:
  story:
    pointer_fields: [Implements, {name: Parent, targets: [epic]}]
relation_pairs:
  - {name: parent, forward: Parent-of, inverse: Child-of, reciprocity: required, target_types: [story]}
```

Each element of a list is judged; with no target types declared anywhere the rule reports `not-enabled`. A reference that names no record is not this rule's finding but `pointer.resolution`'s, and that rule must be enabled for a dangling link to be reported; a freshly initialised config does not turn it on. Each target type
must be a declared record type, the list must not be empty, and only a `pointer_fields` entry takes `targets`. A pair
with `cross_type: false` may list only types that carry its forward field.

### Containment: `part_of`

A reference with target types does not say "is part of", so containment is its own declaration. Add
`part_of: true` to a `pointer_fields` entry that declares `targets`:

```yaml
record_types:
  step:
    pointer_fields: [{name: In, targets: [stage], part_of: true}]
```

A record naming another through that field is part of it. `urzua export` reads `parent`, `children`,
`children_by_state` and `state_disagrees_with_children` from it, [the board](board.md) draws its tree
from it, and `roadmap.group_by` follows a record that carries no value for a group field to the value of its
nearest parent. A configuration without `part_of` has no tree, and its types are flat groups. `part_of`
requires `targets`, and a type declares at most one.

The opt-in rule `status.children-agreement` holds a parent's `Status` to its children's. Enable it under
`rules:` (`status.children-agreement: error`); it has no options and reads only `part_of` and each type's
`statuses`. On the parent's file it reports a parent that is not started (a status outside its type's
`statuses.progress`) while a child is under way or finished, and a parent that is finished (`statuses.success`) while a
child is neither finished nor dropped. A parent whose children are all finished and that is still under way is not
reported, nor is a dropped parent, a parent of a type that declares no `statuses.progress` (unless it is finished), a parent whose
`Status` its type does not declare, or a record whose `part_of` field names several records. Where the type declares
exactly one progress status, its transitions lead there and no `authority` gate reserves the move, the finding's suggested action sets the parent's
status to it; a gated move is never suggested, and the finding names who may make it. `urzua fix` does not apply it. With
`--base`, a child the change added, or whose status or `part_of` value differs from the merge base's, brings its parent
into the run, so the finding appears on the pull request that caused it. Enabled with no `part_of` field, or with a
parent type that declares no `statuses.progress`, `check` says so in one notice.

### `statuses`

| Key | What it does |
|---|---|
| `values` | The allowed statuses. |
| `success`, `failure` | Which values are a good or a bad outcome. |
| `implementable` | Statuses whose records code may legitimately keep citing. |
| `claim_retained` | Statuses that keep a record's evidence claim even though it is no longer current. |
| `initial` | Statuses a record may be filed in. Declared only with `transitions`; one without the other is a load error. `urzua board` names a type that has statuses between its start and its end but no `statuses.progress` only when `initial` is declared, since without it a start status cannot be told from an in-between one. |
| `transitions` | `{From: [To, ...]}`: the lawful moves. |
| `require`, `forbid` | Lists of `{when: [status, ...], when_field: {Field: [value, ...]}, fields: [field, ...]}`: fields that must, or must not (absent or blank), hold a value when the status is one of `when` and each `when_field` field holds one of its values. Declare `when`, `when_field` or both. `status.required-field-missing` is on at `error` wherever some type declares a `require` entry, and `status.forbidden-field-present` wherever one declares a `forbid` entry; a rule you name under `rules:` keeps the level you give, and `urzua doctor` warns of an entry whose rule you turned off. A finding says whether the field is `missing from the header` or `empty`, and names the values the field may hold when the type declares `field_values` for it. |
| `pending` | The type's placeholder vocabulary: whole values (`unknown`) and `{pattern: <regex>}` entries that count as missing. |
| `placeholders` | Whole values (compared without case) that are unedited template text, replacing the built-in words `name`, `name(s)`, `yyyy-mm-dd`, `tbd`, `todo`, `(project lead)`, `(session author)` for this type. An empty list is a load error. |
| `blocked`, `progress` | Statuses read by the derived `blocked` value and `sequence.status-agreement`. |
| `delivery` | How a status agrees with delivery evidence, including per-item evidence in a list-form "Done means" section (`items`). See [the checklist page](checklist.md) for a complete block and record, and `SPEC-3`, "Done-means items". |

A vocabulary and a requirement keyed on another field: `Kind` must be one of three values, and a bug must carry a `Severity`.

```yaml
record_types:
  work:
    dir: docs/work
    required_fields: [Status, Kind]
    known_fields: [Severity, Parent]
    pointer_fields: [Parent]
    narrative_fields: []
    field_values:
      Kind: [feature, bug, chore]          # field.value-not-declared reports `Kind: Bug`
    statuses:
      values: [Open, Done]
      require:
        - {when: [Done], fields: [Parent]}                  # keyed on the status
        - {when_field: {Kind: [bug]}, fields: [Severity]}   # keyed on another field
```

The forbid form takes the same keys: `forbid: [{when_field: {Kind: [chore]}, fields: [Severity]}]` keeps a chore from carrying a severity (absent or blank).

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
`preamble: true` to find the block outside any section, and an optional `phrases` list replacing the
five default phrases `in the context of`, `facing`, `we decided`, `to achieve`, `accepting`) or
`table-min-rows` (with `min_rows`).
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
  evidence_class: evidence     # optional: the class of re-pin rows (fix --rebaseline, reverify, fix --conflicts);
                               # must be in classes; leave it out of version_bump.classes so a re-pin needs no bump
  narrative_column: Change     # optional: the narrative header of a log fix --rebaseline/--pin creates (default Change; no `|` or line break in any *_column)
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
