# Command reference

```text
urzua [--config <path>] [--as-of <YYYY-MM-DD>] <command> ...
```

Two flags are accepted by every command:

- `--config <path>` is the config to use. It defaults to the nearest `.urzua/config.yaml` at or above
  the current directory.
- `--as-of <YYYY-MM-DD>` evaluates every date comparison (waiver expiry, claim age, status expiry) as
  of that UTC date instead of today, and echoes it in the report's `as_of`. It is accepted by the
  read-only modes of `check`, `audit`, `fix` and `reverify`; it is refused with `--apply`, where it would write
  a revision row with a false date, and by commands that compare no dates.

Every command prints one JSON object to stdout. Where a command has a `status`, it is one of the
values listed for it. Exit code 0 means the command ran and nothing blocks, 1 means it ran and found
something blocking, and 2 means it could not run (the report then has `"status": "not-run"` and an
`error`). `urzua --help` lists the commands and `urzua <command> --help` lists the flags; the pages
below were written from that output.

| Command | Does what | Writes? |
|---|---|---|
| [`init`](#init) | Propose and write the config for an existing corpus | `.urzua/config.yaml` |
| [`check`](#check) | Validate the records | no |
| [`new`](#new) | Create a record with a stable ID | a new record |
| [`fix`](#fix) | Detect, and with `--apply` repair, derived fields and stale pins | with `--apply` |
| [`reverify`](#reverify) | Renew an expired evidence claim | with `--apply` |
| [`waive`](#waive) | Preview waiver drafts for the current findings | no |
| [`mcp`](#mcp) | Serve the corpus to an agent over MCP, read-only | no |
| [`doctor`](#doctor) | Report the health of the config and its wiring | no |
| [`explain`](#explain) | Which records govern a path, or what a rule is | no |
| [`graph`](#graph) | The record relationship graph | no |
| [`roadmap`](#roadmap) | The order of work, computed from the records | no |
| [`rules`](#rules) | Every rule this build ships | no |
| [`migrate`](#migrate) | Backfill stable ids, upgrade the config, preview a new required field | with `--apply` or `config` |
| [`export`](#export) | Emit the records as JSON | no |
| [`index`](#index) | Regenerate declared index files | with `--apply` |
| [`refs`](#refs) | Refresh the issue and pull request number snapshot | the snapshot file |
| [`harness`](#harness) | Install and run the agent harness | settings, hooks, lock |
| [`audit`](#audit) | Deprecated alias of `check` | no |
| [`import`](#import) | Not implemented | no |

## init

```text
urzua init [--dry-run] [--config <path>]
```

Adopts an existing corpus: proposes one record type per directory of numbered records and writes
`.urzua/config.yaml`. It never moves or rewrites a record, never overwrites an existing config (it
reports what exists and exits 2), and is idempotent. With `--dry-run` it prints the config it would
write and writes nothing; it still runs when a config exists, with a notice saying so.

Output: `status`, `dry_run`, `config_path`, `proposed` (each with `name`, `dir`, `record_count`,
`prefix`, `header_shape`), `written`, `config_yaml` (the file as one string) and `notices`.
Back-filling stable ids is a separate step: [`migrate ids`](#migrate). See [Adopt an existing
corpus](adopt.md).

## check

```text
urzua check [PATHS]... [--base <ref>] [--fail-on error|warning] [--rules <id>,...]
            [--stdin-path <path>] [--actors <file>]
```

Validates the records. Exits 1 when a finding blocks. Every flag and every output field is described
in [`check` and its JSON output](check.md).

## new

```text
urzua new <record-type> [<title>] [--by <name>]
```

Creates a record of a type declared in the config, with a stable id assigned. It never asks you to
choose a number: it takes one past the highest in the type's directories, skipping numbers used on
any local branch or remote-tracking ref (it names the skipped numbers on stderr; run `git fetch`
first, since refs never fetched are invisible). With `new.check_open_prs: true` in the config it also
fetches pull request head refs so numbers taken by open pull requests are skipped.

The header is built from the type's `required_fields`, with the `Stable-Id`, the date, the author
(from `--by`, `gh`, or `git config user.name`) and the first declared status filled in; every other
field is left for you. If `.urzua/templates/<type>.md` exists, its body (title and sections) is
appended. A type with `header_shape: none` is refused, since it has nowhere to store a stable id.

Output: `status`, `path`, `display_number`, `stable_id`, `pull_refs` (whether the open pull request
fetch ran and which numbers it skipped) and `skipped`.

## fix

```text
urzua fix [PATHS]... [--apply --ids <path>,... | --force] [--by <name>] [--actor agent|human]
          [--tier 1] [--rebaseline --reason <text>] [--stdin-path <path>]
```

Finds fields whose stated value disagrees with what urzua computes. Read-only by default. Tier 1,
the `Embodiment` field, is the only tier implemented; any other `--tier` exits 2.

- `PATHS`: examine only these record files or directories. A path that matches no record is reported
  as a failure.
- `--apply` writes the computed value back. It requires `--ids` (repo-relative record paths) or
  `--force` (every detected repair), and an identity: `--by`, or resolved from `gh`/`git config`.
  Each write changes only that field's line and appends a structural row to the record's revision
  log; a record with no revision log is refused, not skipped. The report's `identity` says who was
  recorded and from which source; a verified `gh` login takes precedence over `--by`, with a notice.
- `--actor agent|human` records who ran the command in the revision row, beside the identity
  source. It is the caller's claim, not proof. Without it the row records `unspecified`.
- `--rebaseline` re-pins stale `@sha256:` evidence pins instead of repairing `Embodiment`. Without
  `--apply` it lists each stale pin with its recorded and current hash. With `--apply` it needs
  `--ids` or `--force`, `--by` and `--reason` (why the evidence still holds), rewrites the pins and
  appends an attributed revision row.
- `--stdin-path <path>` judges proposed content for one record from stdin without writing. It cannot
  be combined with `PATHS` or any apply flag.

`status` is `ok`, `repairs-available`, `applied` or `partial-failure`. Other output keys: `population`,
`repairs` (`record`, `field`, `line`, `current_value`, `computed_value`, `tier`, `evidence`),
`rebaselines` (`record`, `field`, `line`, `pins` with `locator`, `recorded_hash`, `current_hash`),
`failed_pins`, `failed`, `identity` and `notices`. See [Waivers and drift](waivers-and-drift.md).

## reverify

```text
urzua reverify [--ids <path>,... | --force] [--apply --reason <text>] [--pin-only]
               [--by <name>] [--actor agent|human]
```

Renews an expired evidence claim: re-resolves and re-hashes every locator of a record, refuses to
renew over changed evidence, pins `code:` and `test:` locators that have no pin, and records who
renewed it and why. It needs the `embodiment.claim-expired` rule declared in the config; without it
the command reports `not-run` and says so.

- Without `--apply` it reports. With no `--ids` it reviews the records whose claim has expired (with
  `--pin-only`, those with an unpinned locator).
- `--apply` writes. It requires `--ids` or `--force`, `--reason` (one line, without `|`, recorded in
  every row) and an identity. At most `max_batch` records per run (a rule option, default 1).
  `--force` is refused when it would select more than one record.
- `--pin-only` pins the unpinned `code:` and `test:` locators and renews nothing: the attested way to
  pin a corpus for the first time. A pin that has changed is `fix --rebaseline`'s.
- `--actor agent` is refused where the claim period's level is `error`.

Output keys include `status` (`repairs-available`, `applied`, `not-run`, ...), `mode`, `pin_only`,
`population`, `records` (each with `status`, `tier`, `age_days`, `period_days` and per-locator
`resolved`, `pinned`, `current_hash` and `check`), `failed`, `identity` and, with `--apply`, the
`reason`.

## waive

```text
urzua waive --from-report [--rule <id>,...] [--path <path>]... [--by <name>]
```

Drafts one waiver per rule and file for the current unwaived findings, in your repository's own
waiver format. **This build previews only: it writes nothing.** `--apply`, which writes the drafts,
is not accepted yet; passing it is a usage error. Its `mode` is always `dry-run`.

`--from-report` is required and selects the source: a `check` run in process over the whole corpus.
`--rule` narrows to the named rules (a rule id this build does not ship exits 2). `--path` narrows to
a file or a directory (a path, not a glob; repeatable). `--by` names who ran it.

A draft has `Rule` and `Scope` copied from the finding and a pending `Reason`, with no `Expires`, so
it suppresses nothing until a person fills in the reason and the date. The command refuses to run
where a draft would take effect on its own (the waiver type must require `Reason`, and either declare
`permanent` or require `Expires`).

Output: `status`, `mode`, `pending_value`, `pending_source`, `drafts` (each with `path`, `rule`,
`scope`, `findings` and `written`), `skipped` (a finding that gets no draft, with a `reason` such as
`already-waived`, `expired` or `unwaivable`) and `unmatched_filters`. Exit 0 whenever it ran. See
[Waivers and drift](waivers-and-drift.md).

## doctor

```text
urzua doctor
```

Reports on urzua's own configuration and wiring, not on record content: whether there is a config,
whether it parses (an unrecognized key is an error here), whether each record type's directory
exists, whether a CI workflow invokes `urzua check` or the gate action, whether git pre-commit and
pre-push hooks run it, and the harness and authority declarations.

Output: `status` (`ok`, `warn` or `error`) and `checks`, each with `check`, `status` and `message`.
Exit 0 for `ok` and `warn`, 1 for `error`, 2 when there is no config to read.

## explain

```text
urzua explain <path>
urzua explain --rule <rule-id>
```

With a path (a file, directory, glob, or `path#scope`): every record whose `Realized-by` names it as
evidence, in `governing_records` (each with `record`, `record_type` and `via`). A path that is
neither in the repository nor cited by a record exits 2.

With `--rule`: what the rule is. Its `rule` object has `id`, `description`, `scope`, `judges`,
`population_unit`, `spec`, the `level` your config sets, `waivable`, its `options` with their current
values and its `actions`.

## graph

```text
urzua graph
```

Every relationship edge between records, as data. Each edge is `{from, relation, to, kind, dangling}`
with `kind` either `pointer` or `narrative` and `dangling` true when the target does not exist. The
edges come from the fields the config declares as `pointer_fields` and `narrative_fields`, plus
`Supersedes` and `Superseded-by`.

## roadmap

```text
urzua roadmap
```

The order of work, computed from the records: waves, the ready set, the critical path, what each
record blocks and a grouping. It reads the sequencing pairs declared under `roadmap.pairs` in the
config and stores nothing. A repository that declares none gets a note (`no-sequencing-pair`), not an
empty roadmap. Output: `status` and `roadmap` with `schema`, `pairs`, `waves`, `ready`,
`critical_path`, `records`, `groups`, `needs_replan`, `cycles` and `notes`.

## mcp

```text
urzua mcp [--root <dir>]
```

A read-only Model Context Protocol server on stdio, so an agent can read the corpus without a shell. It
speaks newline-delimited JSON-RPC 2.0 on stdin and stdout until stdin closes, and stdout carries protocol
frames only. `--root` is the repository to serve; without it the server uses the repository holding the
current directory. It exits 2, with a JSON could-not-run on stderr, when it cannot read the config.
`--as-of` is refused.

Five tools, each returning the report its command prints as `structuredContent` and as the same JSON in
one text block, and each advertised read-only:

- **`urzua_check`** takes optional `paths`, `base` and `rules` and returns the `check` report.
- **`urzua_list_records`** takes an optional `type` and optional `status` list and returns each matching
  record's id, type, path and header fields, without its text. Rejected and superseded decisions are
  `type: "adr"` with `status: ["Rejected", "Superseded"]`.
- **`urzua_show_record`** takes an `id` and returns that record in full and the relation edges that touch it.
- **`urzua_get_findings_for_path`** takes a `path` and returns the `explain` report.
- **`urzua_get_graph`** takes nothing and returns the `graph` report.

No tool writes. A call to any other tool name is refused. Register it in your agent's MCP configuration
with `command: urzua` and `args: ["mcp"]` (add `--root` when the agent does not start it in the repository);
`harness install` does not write MCP configuration. [The MCP server](mcp.md) has the details.

## rules

```text
urzua rules
```

Every rule this build ships, each with `id`, `description`, `scope`, `judges`, `population_unit`,
`spec` and `actions`. It reads no config: it says what the binary supports, not what your repository
enabled. Use [`explain --rule`](#explain) for one rule's options and your current level.

## migrate

```text
urzua migrate ids [--apply]
urzua migrate config
urzua migrate schema --report --field <name>
```

- **`ids`** backfills a `Stable-Id` into every record lacking one, keeping the file name's number as
  the display number so existing references keep resolving. A dry run unless `--apply`. Output:
  `status`, `apply`, `missing` (the records that lack one).
- **`config`** rewrites the config from the previous schema version to the current one, in place,
  keeping comments, and reports every key it added, changed, renamed, moved or dropped. When the
  config is current it reports `outcome: nothing-to-do`. A version outside what this build reads is
  refused.
- **`schema --report`** previews which existing records lack a real value for `--field`, before the
  field is added to the config as required. `--assist-waivers` and `--apply` are not implemented.

## export

```text
urzua export --format json
```

Every record as plain JSON data. Output: `status`, `record_count`, `types` and `records`, each record
with `id`, `type`, `path`, `stable_id`, `header`, `header_readable`, `preamble`, `sections` and
`raw_text`. The format must be named. `--format agdr` is not implemented and exits 2.

## index

```text
urzua index [--apply]
```

Regenerates the index files declared under the `index.freshness` rule's `indexes` option from the
records. It reports which artifacts differ and writes nothing; `--apply` writes them. Without a
declared index it reports `not-run`.

## refs

```text
urzua refs refresh
```

Fetches every issue and pull request number of the declared repository from the GitHub REST API and
rewrites the snapshot file that `citation.issue-exists` checks citations against. It is the only
command that uses the network (`check` never does). It reads `GITHUB_TOKEN` or `GH_TOKEN` when set
and needs `curl`. A failed fetch writes nothing and exits 2. Commit the snapshot afterwards.

## harness

```text
urzua harness install [--adapter <name>] [--dry-run]
urzua harness status
urzua harness update [--force] [--dry-run]
urzua harness uninstall [--dry-run]
urzua harness adapters
urzua harness run <edit | session-start | post-command>
```

Installs and runs the agent harness. See [the agent harness](agent-harness.md) for what each does.

## audit

```text
urzua audit
```

A deprecated alias of `check` over the configured rules: it prints `check`'s report plus a
deprecation notice that never changes the exit code. It is removed in the next release. Use `check`,
or `check --rules <id>,...` for a narrower run.

## import

```text
urzua import --format <json|agdr> [PATHS]...
```

Not implemented. It exits 2.
