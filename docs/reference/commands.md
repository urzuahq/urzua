# Command reference

```text
urzua [--config <path>] [--as-of <YYYY-MM-DD>] <command> ...
```

Two flags are accepted by every command:

- `--config <path>` is the config to use. It defaults to the nearest `.urzua/config.yaml` at or above
  the current directory.
- `--as-of <YYYY-MM-DD>` evaluates every date comparison (waiver expiry, claim age, status expiry) as
  of that UTC date instead of today, and echoes it in the report's `as_of`. It is accepted by the
  read-only modes of `check`, `audit`, `fix`, `reverify` and `waive`; it is refused with `--apply`, where it
  would write a revision row with a false date, and by commands that compare no dates.

Every command prints one JSON object to stdout. Where a command has a `status`, it is one of the
values listed for it. Exit code 0 means the command ran and nothing blocks, 1 means it ran and found
something blocking, and 2 means it could not run (the report then has `"status": "not-run"` and an
`error`). `urzua --help` lists the commands and `urzua <command> --help` lists the flags; the pages
below were written from that output.

Every report also carries `next_action`, `next_action_reason` and `next_action_inputs`, on every command and on a `not-run`
report. `next_action` is one command to run next, as a bare string such as
`urzua fix --apply --ids <record> --actor agent`, or `null` when nothing is better than reading the
findings; `next_action_reason` says why, from counts, and is `null` exactly when `next_action` is;
`next_action_inputs` lists the `<...>` placeholders the command carries, and is empty when it runs as
printed. `new` names `git add -- :/<path>` when untracked records are not examined; a `not-run` report
names a command when one removes the cause (the nearest rule id for a mistyped one, `git fetch` for
a `--base` ref of a configured remote that is not here, `git fetch --unshallow` for a shallow clone) and
`null` when the remedy is an edit or a choice only you can make (an unknown configuration key, a
path outside the repository, a directory that is not a git repository).
[`check`](check.md) describes how it is chosen.

### What uses the network

Almost nothing. `check`, `fix` and `reverify` without `--apply`, `explain`, `graph`, `roadmap`,
`rules`, `doctor`, `export`, `index`, `migrate`, `mcp`, `init` and `harness` make no network call.
Four commands do:

- `urzua refs refresh` fetches issue and pull request numbers from the GitHub REST API (with `curl`).
  It is the only command whose job is to reach the network.
- `urzua new`, `urzua waive` (with or without `--apply`) and `fix --apply` and `reverify --apply`
  (including `fix --rebaseline --apply`) resolve who is acting by running `gh api user`, which asks
  GitHub for the logged-in login. A missing, logged-out or slow `gh` (it is given five seconds) is
  not an error: the command falls back to `--by`, then to `git config user.name`. A verified `gh`
  login takes precedence over `--by`.
- `urzua new` with `new.check_open_prs: true` runs `git fetch` against the configured remote to read
  pull request head refs.

| Command | Does what | Writes? |
|---|---|---|
| [`init`](#init) | Propose and write the config, for an existing corpus or from built-in profiles | `.urzua/config.yaml`, templates |
| [`check`](#check) | Validate the records | no |
| [`new`](#new) | Create a record with a stable ID | a new record |
| [`fix`](#fix) | Detect, and with `--apply` repair, derived fields and stale pins | with `--apply` |
| [`reverify`](#reverify) | Renew an expired evidence claim | with `--apply` |
| [`waive`](#waive) | Draft waivers for the current findings | with `--apply` |
| [`mcp`](#mcp) | Serve the corpus to an agent over MCP, read-only | no |
| [`doctor`](#doctor) | Report the health of the config and its wiring | no |
| [`explain`](#explain) | Which records govern a path, or what a rule is | no |
| [`graph`](#graph) | The record relationship graph | no |
| [`roadmap`](#roadmap) | The order of work, computed from the records | no |
| [`rules`](#rules) | Every rule this build ships | no |
| [`migrate`](#migrate) | Backfill stable ids, rename files to the filename convention, upgrade the config, preview a new required field | with `--apply` or `config` |
| [`export`](#export) | Emit the records as JSON | no |
| [`index`](#index) | Regenerate declared index files | with `--apply` |
| [`refs`](#refs) | Refresh the issue and pull request number snapshot | the snapshot file |
| [`harness`](#harness) | Install and run the agent harness | settings, hooks, lock |
| [`audit`](#audit) | Deprecated alias of `check` | no |
| [`import`](#import) | Not implemented | no |

## init

```text
urzua init [--dry-run] [--types <name>,...] [--dir <dir>] [--config <path>]
```

Two modes. **Adopt** (the default) proposes one record type per directory of numbered records and
writes `.urzua/config.yaml`. It never moves or rewrites a record. **Greenfield**, for a repository with
no records yet, is selected by `--types`: a comma-separated list of built-in profiles (`adr`, `rfc`,
`spec`, `prd`), each written out into the config as an ordinary, editable `record_types` entry with
its statuses and required fields, plus a starter template at `.urzua/templates/<type>.md` that `new`
fills in. `--dir` is the layout root for `--types` (default `docs`): each type lives in
`<dir>/<type>`. It must be a relative path inside the repository. On a terminal, in a repository with
no record directory, plain `init` asks which types to govern; with no terminal it refuses rather than
block on a prompt. An unknown or repeated type name, or `--dir` without `--types` when adopt would run,
exits 2 and writes nothing. Greenfield does not create the type directories: `urzua new` creates one
when it writes the first record.

Both modes never overwrite an existing config (they report what exists and exit 2), except that a
greenfield re-run whose `--types` and `--dir` would write exactly the config that exists is a no-op
(exit 0, `written: false`; a missing template is still written; an existing template is kept). With
`--dry-run` it prints the config it would write and writes nothing; it still runs when a config exists,
with a notice saying so.

Output: `status`, `dry_run`, `config_path`, `proposed` (each with `name`, `dir`, `record_count`,
`prefix`, `header_shape`), `written`, `config_yaml` (the file as one string) and `notices`. Greenfield
adds `templates`, each `{type, path, state}` with `state` `written`, `kept` or `would-write`; adopt has
no such key. The starter config sets `authority: {decider_table: base, waivable: never}` and
`harness: {outside_writes: shared}`. Back-filling stable ids is a separate step: [`migrate
ids`](#migrate). See [Adopt an existing corpus](adopt.md).

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
any local branch or remote-tracking ref (it lists the skipped numbers, with the reason, in the `skipped`
field of its JSON and writes nothing about them to stderr; run `git fetch` first, since refs never
fetched are invisible). With `new.check_open_prs: true` in the config it also
fetches pull request head refs so numbers taken by open pull requests are skipped.

The header is built from the type's `required_fields`, with the `Stable-Id`, the date, the author
(from `--by`, `gh`, or `git config user.name`) and the first declared status filled in; every other
field is left for you. A field left for you is never written `null`: it carries the template's text
for it, or `—` when the type requires it and the template has none, or is left out. `to_fill` lists
each such field as `{field, reason}` (`placeholder` or `required-empty`), and `next_action_reason`
names them. If `.urzua/templates/<type>.md` exists, its body (title and sections) is
appended. A type with `header_shape: none` is refused, since it has nowhere to store a stable id.

Output: `status`, `path`, `display_number`, `stable_id`, `pull_refs` (whether the open pull request
fetch ran, and which numbers it skipped and who held them), `skipped`, `to_fill` and `notices` (when present: the identity used in place of `--by`, a failed fetch, a type directory that does not exist yet; whatever `new` prints to stderr is also a notice). `skipped` is always present and empty
when nothing was skipped: one `{number, reason}` per number, with `reason` `local`, `worktree <path>`
or `open-pr <ref>`. When the type's directory does not exist yet, `new` numbers from 1 and creates it,
saying so on stderr and in a `notices` entry. It resolves an identity with `gh api user` first (see [What uses the
network](#what-uses-the-network)).

## fix

```text
urzua fix [PATHS]... [--apply --ids <path>,... | --force] [--by <name>] [--actor agent|human]
          [--tier 1] [--rebaseline --reason <text>] [--base <ref>] [--conflicts] [--stdin-path <path>]
```

Finds fields whose stated value disagrees with what urzua computes. Read-only by default. Tier 1
recomputes `Embodiment` and `Status`; `--tier` accepts only 1, and any other value exits 2 (the one Tier 2 repair, below, runs without it).

With `relation.pair-reciprocity` or `relation.supersession-reciprocity` on, it also lists the missing
other half of a relation (a Tier 2 repair, run by the default command): when A claims a relation with B
and B's inverse field does not name A, the repair adds A's id to B's field (appending to a list, adding
the field when absent, and keeping a flow list, block list or quoted value in its form). It never writes a protected path (a path gate of `authority`; a notice says a decider makes the change), never changes a `Status` (the repair's `inverse.status` says the status move
is a separate, gated step), never overwrites a value that names no record (reported as a notice), and
writes nothing for a target that does not exist or a type the pair does not cover. The reciprocity
findings print this command, `urzua fix --apply --ids <B>`.

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
  appends an attributed revision row. A record with no revision log gets one, in the type's template's
  or declared shape, placed where the template orders its log, with that row first
  (`creates_revision_log: true`, in the listing too). Refused, in the listing and the apply: a record
  with no heading below its header (`next_action` appends one), one ending inside an open code fence,
  and one a committed copy of which had a log: `HEAD`'s, or the copy at the merge base of `--base
  <ref>` (default `origin/HEAD`) and `HEAD`, followed across a rename or by `Stable-Id`
  (`next_action` is the `git show` of that copy, to restore it). With no base ref resolving the
  record's recent history is searched instead and a `fix.revision-log-base` warning says so. `fix
  --pin` takes `--base` and plans the log the same way.
- `--conflicts` looks at the records a merge or rebase left unmerged and lists each by `kind`. Kind
  `pin` is a record whose two sides differ only in pin hashes (in the evidence field or in a Done-means
  item's evidence clause), the version field and rows added to the revision log. With `--apply` (and
  `--ids` or `--force`, and an identity) it keeps both sides' rows (by date, and within a date the
  merged-in side's first and this branch's last, in a merge and a rebase alike), takes the higher
  version, recomputes each pin both sides moved from the merged file and appends a row saying the pin
  was recomputed on merge. Any other conflict is `refused` with its reason and left untouched; a record
  whose pinned file is itself still unmerged lists that file under `waits_on`, and `next_action` says to
  resolve and stage it first, then re-run. It does not stage: `git add` the resolved records.
  `status` is `nothing-conflicted`, `no-record-conflicts`, `resolvable`, `resolved` or
  `partial-failure`.
- `--stdin-path <path>` judges proposed content for one record from stdin without writing. It cannot
  be combined with `PATHS` or any apply flag.

`status` is `ok`, `repairs-available`, `applied` or `partial-failure`. Other output keys: `population`,
`repairs` (`record`, `field`, `line`, `current_value`, `computed_value`, `tier`, `evidence`, and
`inverse` for a Tier 2 repair),
`rebaselines` (`record`, `field`, `line`, `pins` with `locator`, `recorded_hash`, `current_hash`,
and `creates_revision_log` when the write creates the record's log),
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
urzua waive --from-report [--rule <id>,...] [--path <path>]... [--by <name>] [--apply]
```

Drafts one waiver per rule and file for the current unwaived findings, in your repository's own
waiver format. Without `--apply` it previews and writes nothing (`mode` is `dry-run`). With `--apply`
it writes each draft as a new record the way `new` does: numbered past the highest id, a fresh
`Stable-Id`, today's date, and under the same numbering lock (`mode` is `apply`). A second `--apply`
writes nothing, because the findings its drafts name are then `already-waived`. The written files are
untracked: `git add` them, or `check` will not read them. `--as-of` is refused with `--apply`.

`--from-report` is required and selects the source: a `check` run in process over the whole corpus.
`--rule` narrows to the named rules (a rule id this build does not ship exits 2). `--path` narrows to
a file or a directory (a path, not a glob; repeatable). `--by` names who ran it in each draft's
revision log; it defaults to the git identity.

A draft has `Rule` and `Scope` copied from the finding and a pending `Reason`, with no `Expires`, so
it suppresses nothing until a person fills in the reason and the date: once the files are tracked,
`check` reports the original finding as before and, where `waiver.matches-nothing` is enabled, one
more finding per draft saying it is not in effect, until they do. The command refuses to run
(exit 2, nothing written) where a draft would take effect on its own: the waiver type must list
`Reason` in `effective_requires`, and either declare `permanent` or list `Expires` in
`effective_requires` too.

Output: `status` (`ok`, `partial-failure` or `not-run`), `mode`, `pending_value`, `pending_source`,
`drafts` (each with `path`, `stable_id`, `rule`, `scope`, `findings` and `written`; `stable_id` is
`null` in a preview), `skipped`, `unmatched_filters` and `notices` (when present: the identity used in place of `--by`, and what numbering reported; the same text is on stderr, but the JSON is the contract). A `skipped` entry is `{rule, file, reason}`
(plus `waiver` when one exists) for a finding that gets no draft; `reason` is one of
`already-waived` (a waiver for the same rule and file exists and has not expired, but is not yet
filled in), `expired`, `unwaivable`, `no-file`, `matches-nothing`, `waiver-record`,
`resolved-by-drafts` or `resolved-by-own-draft`. A finding that a filled-in, unexpired waiver already
covers is not a candidate and is not listed. Exit 0 whenever it ran; 2 when it could not run, or an
`--apply` failed before writing any draft; 1 when an `--apply` failed after writing some
(`partial-failure`). See [Waivers and drift](waivers-and-drift.md).

## doctor

```text
urzua doctor
```

Reports on urzua's own configuration and wiring, not on record content: whether there is a config,
whether it parses (an unrecognized key is an error here), whether each record type's directory
exists, whether a CI workflow invokes `urzua check` or the gate action, whether git pre-commit and
pre-push hooks run it, and the harness and authority declarations.

Output: `status` (`ok`, `warn` or `error`) and `checks`, each with `check`, `status` (`ok`, `info`,
`warn` or `error`; `info` never changes the report's `status`) and `message`.
A `hook-wired` check also carries `layer`, `state`, `hook_layout` (`default`, `hooks-path`, `husky` or
`pre-commit-framework`) and, when the hook is not wired, `fix`: the command that wires it,
usually `urzua hooks install`. The report's `next_action` is the first such command.
The `check` ids include `config-exists`, `config-parses`, `record-type-dir`, `ci-wired`, `hook-wired`,
`authority-host` (whether the declared authority gates anything), `authority-decider-table`,
`authority-waivable`, `harness-config` (the declared harness, and any key of it this build does not
recognize), `declared-value-vocabulary` (`info`: a declared status value that is also a placeholder
or pending word, which is a real value in the status field and still reported elsewhere) and `template-sections` (whether a type's starter template has a heading for each section
the type requires).
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
edges come from the fields the config declares as `pointer_fields` and `narrative_fields`, the
forward and inverse fields of every declared `relation_pairs` entry (tagged `pointer`), plus
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
`spec` and `actions`, and a top-level `status` of `ok` like every other command's report. It reads no config: it says what the binary supports, not what your repository
enabled. Use [`explain --rule`](#explain) for one rule's options and your current level.

## migrate

```text
urzua migrate ids [--apply]
urzua migrate filenames [--apply]
urzua migrate config
urzua migrate schema --report --field <name>
```

- **`ids`** backfills a `Stable-Id` into every record lacking one, keeping the file name's number as
  the display number so existing references keep resolving. A dry run unless `--apply`. Output:
  `status`, `apply`, `missing` (the records that lack one).
- **`filenames`** renames every record file whose name does not match its type's filename convention
  (the type's `identity.pattern`, else `TYPE-NNNN-slug.md`) and rewrites the citations of the renamed
  paths, so a corpus named `0001-use-postgres.md` becomes `ADR-0001-use-postgres.md`. The number and
  slug are kept, so a record's `Stable-Id` and id do not change. A dry run unless `--apply`. Output:
  `status`, `apply`, `renames` (`record_type`, `from`, `to`), `rewrites` (`file`, `line`, `from`, `to`),
  `unrewritten` (a citation in a tracked file that is not markdown, which it reports and does not edit),
  `stale_pins` (evidence pins whose file the rewrite changed; `fix --rebaseline` re-pins them),
  `skipped` (a file it leaves alone, with a `reason` such as `no-number`, `pattern-rejects`,
  `target-exists` or `number-collision`) and `notices`. Run `migrate ids --apply` first: without a
  `Stable-Id` a diff cannot pair a rename with the old file, and the command says so in a
  `stable-id-missing` notice. It uses the filesystem, not `git mv`, so nothing is staged; a second
  applied run finds nothing to rename.
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
command whose job is to reach the network (`check` never does; [which commands do](#what-uses-the-network)).
It reads `GITHUB_TOKEN` or `GH_TOKEN` when set and needs `curl`. When GitHub's first answer names the last page, the other pages are requested `issue_refs.workers` at a time (default 4, 1 to 16); the output is the same either way. A failed fetch writes nothing and exits 2. Commit the snapshot afterwards.

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

## hooks

```text
urzua hooks install [--dry-run] [--write-config]
urzua hooks status
urzua hooks uninstall [--dry-run]
urzua hooks verify [<pre-commit | pre-push>] [--run] [--allow-outside]
urzua hooks run <pre-commit | pre-push>
```

Wires the git hooks `doctor` reports. `install` finds where the hooks live (`.git/hooks`, a
`core.hooksPath` inside the repository, husky's `.husky/`, or the pre-commit framework) and makes the
pre-commit hook run `urzua hooks run pre-commit` and the pre-push hook `urzua hooks run pre-push`. A
missing hook is created; an existing shell hook gets a block between `# >>> urzua-hooks begin` and
`# <<< urzua-hooks end` at its top (after the shebang, comments and a husky v8 source line), so urzua runs
first and the rest of the hook is unchanged; a hook declared under `hooks.<hook>.managed: declared` is not
written. When `hooks.path` names the directory the hooks are committed in, `install` first sets this clone's
`core.hooksPath` to it, refusing (and naming the value's scope and origin) when `core.hooksPath` already names
another directory or the directory resolves outside the repository; `uninstall` leaves the committed files and
unsets the local value when it is that directory; `status` reports `not-pointed` until then. It
refuses, writing nothing and printing the line to add, a Node or Python hook, a hook without the execute bit,
a symlinked or hard-linked hook file, and a hooks directory that resolves outside the repository and its git directory or
cannot be written. With the pre-commit
framework it writes no hook file: the report's `stanza` is the `repo: local` entry to add, and
`--write-config` appends it to `.pre-commit-config.yaml`.

`run` is what the hooks call: it runs the arguments `hooks.pre-commit` or `hooks.pre-push` names in the
config (default `check --base HEAD` and `check`; only `check` or `audit` may be named) and prints, on
stderr, the blocking findings' rule, file and message and the command for the full report. A clean run
prints nothing (`hooks.print_success: true` prints `no blocking findings`). A blocking
finding always stops the commit or push. When urzua is missing, is a release without `hooks run`, or
cannot run, the hook prints one line saying the records were NOT checked, how to install or upgrade urzua
and that CI will check them, and lets the commit through (`hooks.on_not_run: block` stops it instead). See
[CI](ci.md#local-hooks).

Output: `status` (`ok`, `drift` from `status` when a hook is `missing`, `not-wired`, `not-executable`,
`unmanaged-call` (urzua is mentioned outside the managed block, which text cannot prove runs), `stale-policy`,
`stale-block`, `duplicated`, `declared-missing`, `declared-not-executable` or `declared-no-call`; a declared hook
that is plausible is `declared`, which counts as wired), `not-run` when `install` or `uninstall` refused),
`action`, `dry_run`, `layout`, `items` (`{hook, path, state}`), `stanza`, `problems`. `uninstall` removes
only the blocks, restoring the file byte for byte, deletes only a hook file it created, and refuses where
`install` would. Exit 0, 1 (`status` found a hook not wired) or 2 (refused).

`verify` proves a hook reaches urzua by running it. Without `--run` it runs nothing and says that `--run`
executes the repository's hooks. With `--run` it runs each hook once, for real, as git runs it (`git hook
run`): urzua's check runs with its real result, and everything else in the hook happens (a formatter
rewrites files). Any urzua the hook starts records in a probe directory that it was reached and otherwise
behaves exactly as it always does. pre-push gets the remote `urzua-verify urzua-verify://not-a-remote` and
empty stdin; a hook running longer than `hooks.verify_timeout_seconds` is stopped and reported; a
`core.hooksPath` outside the repository needs `--allow-outside`. Output: `status` (`explained`, `ok`,
`unreached`, `not-run`), `run`, `hooks` (`{hook, path, managed, reached, exit_code, timed_out, invocation,
urzua_output, note}`). Exit 1 when a hook is not reached or timed out.

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
