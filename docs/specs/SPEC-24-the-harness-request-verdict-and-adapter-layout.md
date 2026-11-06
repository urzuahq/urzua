---
Stable-Id: 01M6Q32HB672E7RB57RFQ9QEB3
Status: Draft
Date: 2026-11-06
Version: '0.2'
Author: beauwilliams
Subject: 'The neutral request and verdict `urzua harness run` speaks, the adapter directory layout of the embedded bundle, and the lock file.'
Implements: ADR-89
---
# SPEC-24 — The harness request, verdict and adapter layout

## Purpose

The complete reference for what `ADR-89` (section 6) calls the vendor-neutral contract of the agent
harness: the request and verdict that `urzua harness run <event>` reads and prints, the layout of an
adapter directory in the bundle embedded in the binary, and `.urzua/harness.lock`. The engine names no
agent vendor; an adapter is the only part that does, and it is written against this contract. The
configuration keys (`harness`, `authority`) are in `SPEC-3`; the commands are in `SPEC-3` and `SPEC-19`.

## The request

`urzua harness run edit`, `urzua harness run session-start` and `urzua harness run post-command` read one JSON object from stdin. An
unreadable one is not a crash: it is a stated verdict (below).

| Field | Type | Meaning |
|---|---|---|
| `event` | string | `edit`, `session-start` or `post-command`. Informational: the subcommand decides. |
| `tool` | string or null | The agent's tool name, informational. |
| `paths` | list of strings | `edit` requires exactly one path, absolute or relative to `cwd`. `post-command` reads none: git names the changed paths. |
| `content` | string or null | `edit`: the full proposed content of the file. The adapter computes it from the agent's edit. |
| `actor` | `agent` or `human` | Who is acting. Absent means `agent`. Only `on_unjudged: deny` reads it. |
| `cwd` | string or null | The directory the engine enters before it finds the repository. |
| `override` | `{name, reason}` or absent | A named override of the editor hook (`ADR-78`). The environment variables `URZUA_HOOK_OVERRIDE` and `URZUA_HOOK_REASON` mean the same when the field is absent. |

## The verdict

One JSON object on stdout. `findings` carries the findings the verdict is about (the blocked, the
overridden or the shown ones, as `urzua check` prints them); for `session-start` it carries the
readiness problems, each `{check, message}`.

| `verdict` | Exit | Meaning |
|---|---|---|
| `allow` | 0 | Judged, nothing to report. The message counts the rules that ran and names the enabled rules a single record cannot be judged by. |
| `allow-with-warnings` | 0 | Judged; warnings (or errors already on disk) are in `message`, nothing blocks. |
| `block` | 1 | An unwaived error this edit introduced (`block_on`, `SPEC-3`), or an unjudged edit refused by `on_unjudged: deny`. |
| `allow-unjudged` | 0 | The engine could not judge: no rule examined the record, history or configuration unreadable, the guardrail is not enabled, an unreadable request. The message says so and the edit proceeds. |
| `allow-override` | 0 | A block was overridden by name; with `override_log` declared, one row per overridden finding is appended to it. |
| `not-governed` | 0 | No record type governs the path. |

`on_unjudged: deny` turns `allow-unjudged` into `block` for an `agent` actor and leaves it `allow-unjudged`
for a `human`; a named override does not lift it. The message names the escape hatch: a person edits
outside the agent harness (a plain editor or a terminal), where no hook runs. When the configuration does
not load, the engine reads `harness.guardrails.editor-hook` from the raw file to decide, so a broken file
does not silently turn a denying guardrail into an allowing one; a file that is not valid YAML cannot be read
and is allowed.

### The post-command verdict

`urzua harness run post-command` (`ADR-78`, amendment; `MILE-202`) runs after a shell command and
reports; it never blocks and always exits 0. It does not read the command: it lists the changed files with
`git status --porcelain=v1 -z --no-renames --untracked-files=all` (modified, added and untracked files; a
deleted file is dropped), keeps those a record type governs, drops those whose content hash is the one last
reported, and runs `check PATHS` once over the rest, in process, staging nothing. Its verdict carries three
extra fields:

| Field | Meaning |
|---|---|
| `findings` | The unwaived findings the checked records have now and did not have at `HEAD`, as `urzua check` prints them. |
| `pre_existing` | The unwaived findings the same record already had at `HEAD`, judged from its committed text (a record with no copy at `HEAD` has none). Listed, not new. |
| `checked` | The records `check` ran on this time. Empty when no check ran: a clean tree, no governed record changed, the guardrail is off, or every changed record's content hash was already reported. |

`allow` is a check that found nothing, or none ran (the message says which). `allow-with-warnings` lists the
findings with file, line, severity, rule and message, says that the command had already run so nothing was
prevented, and that pre-commit and CI remain the hard gate. `allow-unjudged` is stated, never a quiet pass,
for a `git` that cannot be run, a working tree git does not know, a configuration that does not load, an
unreadable request and a `check` that cannot run; the message contains "NOT judged". The memory of reported
hashes is `.urzua/cache/post-command.json` (derived, ignored by git): it keeps exactly the records that are
changed now, so a reverted record is forgotten, and a clean tree removes it. The guardrail takes no
`on_unjudged`.

## Adapter layout

The bundle is the directory `rust/crates/urzua-cli/bundle/`, embedded in the binary at build time and
laid out so that it could also be published as a release asset without a second format:

```text
bundle/
  instructions.md                    the agent-instructions block, adapter-independent
  <adapter>/
    adapter.json                     name, description, provides, settings_file, requires
    settings.fragment.json           the hook entries to merge into the agent's project settings
    files/<repository-relative path> installed at the same relative path
```

`urzua harness adapters` lists one entry per adapter directory. `adapter.json`:

| Key | Meaning |
|---|---|
| `name` | Equals the directory name. |
| `provides` | The guardrails the adapter can wire (`editor-hook`, `post-command`, `session-start`, ...). A guardrail the repository turns on that no chosen adapter provides is reported at install, by `status` and by `doctor`, never skipped. |
| `settings_file` | The agent's project settings file the fragment is merged into. A local settings file and user-level settings are neither read nor written. |
| `requires.interpreters` | Programs the translator runs under; `session-start` reports one that is not on `PATH`. |
| `requires.env` | Variables the agent sets for its hook commands; `session-start` reports one that is unset. The adapter's commands do not depend on them. |

The settings fragment holds hook entries, each a command that runs `urzua harness run <handler>`; that
text is how an entry is recognised as urzua's, and an entry is keyed by event, matcher and handler. A
translator is a file under `files/`. It turns the agent's hook payload into the request, runs the engine
command it is given after `--`, and turns the verdict into the agent's output; when the engine cannot be
run it prints a stated message and lets the call proceed.

## The lock

`.urzua/harness.lock` is JSON, written in a stable order with a trailing newline:

| Key | Meaning |
|---|---|
| `schema` | `1`. |
| `bundle_version` | The engine version that wrote it; the bundle is versioned with the engine. |
| `adapters` | The adapters installed. |
| `files` | Path to `sha256:<hex>` of each whole file the install owns. |
| `entries` | Each merged hook entry: `file`, `event`, `matcher` (absent for none), `handler` and the `sha256` of its compact JSON. |
| `blocks` | The instructions file and the `sha256` of the block between its markers, markers included. |

`status` compares the lock with the repository and this binary: a path or entry that is gone is `missing`,
one whose hash differs from the lock's is `modified`, one that matches the lock but not this binary's
bundle is `out-of-date`. `update` rewrites an item whose hash still matches the lock and leaves (and
reports) a modified one unless `--force`; it keeps the old hash for what it left, so `status` keeps
reporting it. `uninstall` removes what matches the lock, takes only urzua's entries and the marked block
out of the shared files, and deletes the lock last.

## What's deliberately not built

- The acceptance and protected-path guardrails, the `authority` table and `diff.decider-required`
  (`MILE-200`); the generated drafting and lifecycle commands (`MILE-199`); the post-command check
  (`MILE-202`); the attestation identity source (`MILE-201`). The names are known to `harness.guardrails`
  and an adapter that does not list them in `provides` is reported, not silently skipped.
- Cutting this repository over from `scripts/hooks` to the installed bundle (`MILE-204`).
- A published bundle (`ADR-89` option C). The layout keeps it open.

## References

- `ADR-89` (sections 1 to 3 and 6), `ADR-69`, `ADR-71`, `ADR-78`.
- `SPEC-3` (`harness`, `authority`, `block_on`, `override_log`), `SPEC-19` (the guardrails of this repository).
- `MILE-198`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-06 | Initial spec: the neutral request and verdict of `urzua harness run`, the adapter directory layout and the lock file. **Why:** `MILE-198`, decided in `ADR-89` section 6. | **structural** |
> | 2026-11-06 | Bumped to `0.2`. Adds the `post-command` event: its request (no paths, git names them), its verdict fields `findings`, `pre_existing` and `checked`, the stated `allow-unjudged` cases, and the hash memory under `.urzua/cache`; the Claude Code adapter provides it. **Why:** `MILE-202`, decided in the `ADR-78` amendment and `ADR-89`. | **substantive** |
