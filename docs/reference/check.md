# `check` and its JSON output

`urzua check` validates the records against the rules your config enables and reports what it found.
It never writes.

```sh
urzua check                      # every tracked record
urzua check docs/adr             # only the records below a directory
urzua check docs/adr/ADR-3-x.md  # one file
urzua check --base origin/main   # only what changed since the merge base with a ref
```

## What is examined

By default `check` examines the records git tracks (and staged files) under the declared record
directories. An untracked file is not examined; instead `check` adds an `info` notice named
`discovery.untracked-skipped` that lists up to 20 of them. Run `git add` on a new record, or set
`discovery.include_untracked: true` in the config, to have it examined before it is staged. A file
you name directly on the command line is always examined.

`check` needs a git repository with the history the config's rules read. Drift detection reads
history, so a shallow checkout cannot detect drift; the CI gate refuses a shallow checkout for that
reason (see [CI](ci.md)).

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Ran, and nothing blocks. Findings below the `fail_on` threshold, `info` findings and waived findings do not block. |
| 1 | Ran, and at least one unwaived finding is at or above the threshold. `"blocking": true`. |
| 2 | Could not run: no config, an unparseable config, not a git repository, an unknown rule in `--rules`, a bad `--base`. The report has `"status": "not-run"` and an `error`. |

The threshold is `fail_on` in the config (`error` by default; `warning` makes unwaived warnings
block too), and `--fail-on error|warning` overrides it for one run without changing the JSON.

## `check --base <ref>`

`--base` reports on only the records changed between the merge base of `<ref>` and `HEAD`, including
staged and unstaged edits. It is what a pull request gate runs, so a legacy backlog never blocks a
change that did not touch it.

- **Dependents are included.** An unchanged record that refers to a changed or deleted one (names its
  id, cites its evidence, or shares an evidence locator) is examined too, so deleting a target cannot
  hide the dangling reference it leaves behind.
- **References still resolve against the whole tracked corpus**, not just the changed files.
- **A change to the config examines every record.** `scope.whole_corpus_reason` says so.
- **An empty diff is `ok`**, with an info notice (subject `scope`) saying nothing was examined, not `not-run`.
- **Diff rules only judge here.** Rules that compare a change with its base, such as
  `status.transition-allowed`, `diff.source-requires-record` and `diff.decider-required`, are
  `not-applicable` on a plain `check` and run on `--base`.
- `--actors <file>` hands `diff.decider-required` the pull request's author and approvers as JSON
  (`{"schema": 1, "pr_author": LOGIN, "approvers": [LOGIN, ...]}`). The CI gate writes this file;
  urzua never fetches it. It cannot be combined with `--stdin-path`.

## Other ways to scope a run

- `--rules <id>[,<id>...]` runs only those declared rules. It narrows and never enables: an id the
  config does not declare is an error (exit 2). The report names the filter in `rules_selected` and
  omits the others from `rules_executed`.
- `--stdin-path <path>` reads proposed content for one record from stdin and judges it without
  writing. The path need not exist yet. It is what an editor hook calls. It cannot be combined with
  paths or `--base`.
- `--as-of YYYY-MM-DD` evaluates every date comparison (waiver expiry, claim age) as of that UTC
  date, echoed in `as_of`.

## The JSON report

```json
{
  "status": "findings-present",
  "as_of": {"date": "2026-11-13", "source": "clock"},
  "files_examined": 2,
  "records_read_by_any_rule": 2,
  "rules_executed": [
    {
      "rule": "header.required-fields",
      "population": {"unit": "field", "eligible": 6, "examined": 6, "out_of_scope": 0, "unreadable": 0},
      "status": "ran"
    },
    {"rule": "pointer.resolution", "status": "not-enabled"}
  ],
  "rules_selected": null,
  "scope": {
    "source": "tracked-sweep",
    "base": null,
    "record_types": ["adr"],
    "include_untracked": false,
    "untracked_included": 0
  },
  "blocking": true,
  "findings": [
    {
      "rule": "header.required-fields",
      "severity": "error",
      "file": "docs/adr/ADR-0002-use-queue.md",
      "line": null,
      "message": "missing required header field 'Deciders' for record type 'adr'",
      "suggested_action": {
        "kind": "edit-field",
        "op": "set",
        "target": {"file": "docs/adr/ADR-0002-use-queue.md", "field": "Deciders"}
      }
    }
  ]
}
```

| Field | Meaning |
|---|---|
| `status` | `ok` (no findings), `findings-present`, or `not-run` (could not run; see `error`). Branch on this, and on `blocking`. |
| `error` | Present only when `status` is `not-run`: why. |
| `as_of` | The date comparisons were made against: `date`, and `source` (`clock`, or the `--as-of` flag). |
| `files_examined` | How many record files the run loaded. |
| `records_read_by_any_rule` | How many distinct records at least one rule examined. |
| `rules_executed` | One entry per rule this build ships, each with its `status`, so a check that found nothing is distinguishable from a check that ran on nothing. |
| `rules_executed[].status` | `ran`; `not-enabled` (the rule is off or not declared); `not-applicable` (enabled but the run cannot supply what it judges, for example a diff rule without `--base`). |
| `rules_executed[].population` | For a rule that ran: its `unit` (`record`, `field`, `section`, `record-type`, ...), how many things were `eligible`, how many `examined`, how many were `out_of_scope` and how many `unreadable`. `eligible` greater than `examined` means the rule could not look at some, and the difference is not a pass. |
| `rules_selected` | The ids from `--rules`, or `null`. |
| `scope` | What was selected. `source` is `tracked-sweep`, `argv` (a file named directly), `git-diff` (`--base`) or `stdin`. `base` is the ref for `git-diff`, else `null`. `record_types`, `include_untracked` and `untracked_included` are always present. A `git-diff` run adds `changed_paths`, `dependents` and `whole_corpus_reason`. |
| `blocking` | `true` when the run exits 1. |
| `next_action` | One command to run next, as a bare string such as `urzua fix --apply`, or `null` when nothing is better than reading the findings. On every urzua command's JSON, not only `check`'s. Unwaived error and warning findings that offer a `run-command` pick it, in the order `urzua fix --apply`, `urzua index --apply`, `urzua refs refresh`. |
| `next_action_reason` | Why, from counts: `repairs 3 of 5 findings`. `null` exactly when `next_action` is. |
| `findings` | The findings, below. |
| `notices` | Present when the run has something to say that is not a finding (an untracked record skipped, an empty diff, a section alias in use, ...). Each has `severity`, `subject`, `message` and sometimes `detail`. A notice never changes the exit code. |

### A finding

| Field | Meaning |
|---|---|
| `rule` | The rule id. `urzua explain --rule <id>` says what it judges. |
| `severity` | `error`, `warning` or `info`. `info` is reported and counted and never blocks. |
| `file` | The record, or the config file, the finding is about. |
| `line` | The line, or `null`. |
| `message` | What is wrong, in a sentence. |
| `waived` | Present only on a finding a [waiver](waivers-and-drift.md) covers: the waiver record's path. A waived finding stays in the report and does not block. |
| `suggested_action` | Present when the fix is mechanical enough to name: a `kind` (`edit-field`, `edit-config`, `edit-section`, `run-command`, ...), an `op` (`set`, `add`, ...) and a `target` saying which file, field, section or config key. A program can act on it without reading `message`. |

## Reading a report in a script

```sh
urzua check > report.json; code=$?
case $code in
  0) echo "clean" ;;
  1) jq -r '.findings[] | select(.waived == null and .severity == "error") | "\(.file): \(.message)"' report.json ;;
  *) jq -r '.error' report.json; exit 2 ;;
esac
```

A gate should treat exit 2 as a failure to run, never as a pass.
