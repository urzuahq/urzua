---
Stable-Id: 01M6Q32HB672E7RB57RFQ9QEB3
Status: Draft
Date: 2026-11-06
Version: '0.3'
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

`urzua harness run edit` and `urzua harness run session-start` read one JSON object from stdin. An
unreadable one is not a crash: it is a stated verdict (below).

| Field | Type | Meaning |
|---|---|---|
| `event` | string | `edit` or `session-start`. Informational: the subcommand decides. |
| `tool` | string or null | The agent's tool name, informational. |
| `paths` | list of strings | `edit` requires exactly one path, absolute or relative to `cwd`. |
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
| `block` | 1 | An unwaived error this edit introduced (`block_on`, `SPEC-3`), an unjudged edit refused by `on_unjudged: deny`, or, for an `agent` actor, a move to a status or a write to a path the `authority` table gates (`acceptance`, `protected-paths`). The last is final: a named override is not accepted for it, the message says so, and no override-log row is written. A `human` actor is not affected. |
| `allow-unjudged` | 0 | The engine could not judge: no rule examined the record, history or configuration unreadable, the guardrail is not enabled, an unreadable request. The message says so and the edit proceeds. |
| `allow-override` | 0 | A block was overridden by name; with `override_log` declared, one row per overridden finding is appended to it. |
| `not-governed` | 0 | No record type governs the path. |

`on_unjudged: deny` turns `allow-unjudged` into `block` for an `agent` actor and leaves it `allow-unjudged`
for a `human`; a named override does not lift it. The message names the escape hatch: a person edits
outside the agent harness (a plain editor or a terminal), where no hook runs. When the configuration does
not load, the engine reads `harness.guardrails.editor-hook` from the raw file to decide, so a broken file
does not silently turn a denying guardrail into an allowing one; a file that is not valid YAML cannot be read
and is allowed.

## Adapter layout

The bundle is the directory `rust/crates/urzua-cli/bundle/`, embedded in the binary at build time and
laid out so that it could also be published as a release asset without a second format:

```text
bundle/
  instructions.md                    the agent-instructions block, adapter-independent
  <adapter>/
    adapter.json                     name, description, provides, settings_file, commands_dir, requires
    settings.fragment.json           the hook entries to merge into the agent's project settings
    commands/draft.md                the drafting command, rendered once per declared record type
    commands/<name>.md               a lifecycle command, installed as it is
    files/<repository-relative path> installed at the same relative path
```

`urzua harness adapters` lists one entry per adapter directory. `adapter.json`:

| Key | Meaning |
|---|---|
| `name` | Equals the directory name. |
| `provides` | The guardrails the adapter can wire (`editor-hook`, `session-start`, ...). A guardrail the repository turns on that no chosen adapter provides is reported at install, by `status` and by `doctor`, never skipped. |
| `commands_dir` | Where the agent looks for slash commands. Absent: the adapter installs none. The install writes `<commands_dir>/<type>.md` for every record type the configuration declares (`{{type}}` and `{{dir}}` of `commands/draft.md` filled in) and `<commands_dir>/<name>.md` for every other file of `commands/`. A record type named like one of those lifecycle commands is refused. |
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

- The generated drafting and lifecycle commands (`MILE-199`) and the post-command check (`MILE-202`).
  `post-command` is known to `harness.guardrails`, and an adapter that does not list it in `provides` is
  reported, not silently skipped.
- Cutting this repository over from `scripts/hooks` to the installed bundle (`MILE-204`).
- A published bundle (`ADR-89` option C). The layout keeps it open.

## References

- `ADR-89` (sections 1 to 4 and 6), `ADR-69`, `ADR-71`, `ADR-78`.
- `SPEC-3` (`harness`, `authority`, `block_on`, `override_log`), `SPEC-19` (the guardrails of this repository).
- `MILE-198`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-06 | Initial spec: the neutral request and verdict of `urzua harness run`, the adapter directory layout and the lock file. **Why:** `MILE-198`, decided in `ADR-89` section 6. | **structural** |
> | 2026-11-06 | Bumped to `0.2`. The `block` verdict also covers an agent's move to a gated status or write to a gated path (`acceptance`, `protected-paths`, `MILE-200`), final and not liftable by an override; the guardrails now run whether or not `editor-hook` is enabled. **Why:** `ADR-89` section 4. | **substantive** |
> | 2026-11-06 | Bumped to `0.3`. The adapter layout gains `commands/` and the manifest key `commands_dir` (`MILE-199`); `status` reports a bundle file the lock does not know yet as `missing`. **Why:** `ADR-89` section 1. | **substantive** |
