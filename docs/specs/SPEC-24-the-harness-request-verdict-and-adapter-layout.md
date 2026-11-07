---
Stable-Id: 01M6Q32HB672E7RB57RFQ9QEB3
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/harness/run.rs, code:rust/crates/urzua-cli/src/harness/post_command.rs, code:rust/crates/urzua-cli/src/harness/gate.rs, code:rust/crates/urzua-cli/src/harness/lock.rs, code:rust/crates/urzua-cli/src/harness/mod.rs, code:rust/crates/urzua-cli/bundle/claude-code/files/.claude/hooks/urzua-claude-code.py, test:rust/crates/urzua-cli/tests/harness_integration.rs, test:rust/crates/urzua-cli/tests/harness_hardening_integration.rs, test:rust/crates/urzua-cli/tests/harness_post_command_integration.rs, test:rust/crates/urzua-cli/tests/harness_install_report_integration.rs
Date: 2026-11-06
Version: '0.11'
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
unreadable one is not a crash: for `edit` and `post-command` it is a stated verdict (below). `session-start`
needs nothing from the request but `cwd`, so an unreadable one is read as an empty request and the check runs
in the current directory.

| Field | Type | Meaning |
|---|---|---|
| `event` | string | `edit`, `session-start` or `post-command`. Informational: the subcommand decides. |
| `tool` | string or null | The agent's tool name, informational. |
| `paths` | list of strings | `edit` requires exactly one path, absolute or relative to `cwd`. `post-command` reads none: git names the changed paths. |
| `content` | string or null | `edit`: the full proposed content of the file. The adapter computes it from the agent's edit. |
| `actor` | `agent` or `human` | Who is acting. Absent, or any value but `human`, means `agent`. Only `on_unjudged: deny` and the `authority` gates read it. |
| `cwd` | string or null | The directory the engine enters before it finds the repository. For `edit` with an absolute path, the repository that holds the file is used when it differs, so a `cwd` outside the repository still finds it. A `cwd` that cannot be entered makes the call unjudged. |
| `adapter_error` | string or absent | `edit`: set by a translator that could not build a request (an unreadable payload, an edit it could not place). The engine builds no judgement from it. With no `paths` (an unreadable payload) it refuses the call for an agent when `acceptance` or `protected-paths` is enabled with `on_unjudged: deny` (read from the raw configuration). With a path (an edit the translator could not place, such as an `old_string` that is not in the file) it refuses only when that path is a protected path, the configuration, or a record of a type a status gate names (matched as for the gates), judged as written and as resolved. For any other file it returns `allow-unjudged` with a message: the edit tool's own failure handles an absent `old_string`. |
| `override` | `{name, reason}` or absent | A named override of the editor hook (`ADR-78`). The environment variables `URZUA_HOOK_OVERRIDE` and `URZUA_HOOK_REASON` mean the same when the field is absent. |

## The verdict

One JSON object on stdout with `verdict`, `message` and `findings`, and `path` (the record or file the verdict is about, absent when none) and `override` (the name of the override that was applied, on `allow-override` only) when they apply. `findings` carries the findings the verdict is about (the blocked, the
overridden or the shown ones, as `urzua check` prints them); for `session-start` it carries the
readiness problems, each `{check, message}`.

| `verdict` | Exit | Meaning |
|---|---|---|
| `allow` | 0 | Judged, nothing to report. The message counts the rules that ran and names the enabled rules a single record cannot be judged by. For `session-start`, the harness is installed and wired, or the guardrail is not enabled. |
| `allow-with-warnings` | 0 | Judged; warnings (or errors already on disk) are in `message`, nothing blocks. |
| `block` | 1 | An unwaived error this edit introduced (`block_on`, `SPEC-3`), an unjudged edit refused by `on_unjudged: deny`, or, for an `agent` actor, a move to a status or a write to a path the `authority` table gates (`acceptance`, `protected-paths`). The last is final: a named override is not accepted for it, the message says so, and no override-log row is written. A `human` actor is not affected. |
| `allow-unjudged` | 0 | The engine could not judge: no rule examined the record, history or configuration unreadable, the guardrail is not enabled, an unreadable request, an `adapter_error`. The message says so and the edit proceeds. |
| `allow-override` | 0 | A block was overridden by name; with `override_log` declared, one row per overridden finding is appended to it. |
| `not-governed` | 0 | No record type governs the path. |

`on_unjudged: deny` turns `allow-unjudged` into `block` for an `agent` actor and leaves it `allow-unjudged`
for a `human`; a named override does not lift it. The message names the escape hatch: a person edits
outside the agent harness (a plain editor or a terminal), where no hook runs. When the configuration does
not load, the engine reads `harness.guardrails.editor-hook` from the raw file to decide, so a broken file
does not silently turn a denying guardrail into an allowing one; a file that is not valid YAML cannot be read
and is allowed.

A failure of the adapter itself fails the same way, by the same two guardrails: the Claude Code translator
sends `adapter_error` for an unreadable payload or an edit it cannot place, and refuses the edit itself, from
the configuration's text, when the engine command cannot be run or answers with something that is not a
verdict. It does so only where `acceptance` or `protected-paths` is enabled and not set to `on_unjudged: allow`;
under any other configuration the call proceeds with a stated notice. An edit the translator cannot place is
refused only when its file is a protected path or a gated record (the table above); for any other file it
proceeds with the notice, since the edit tool fails by itself when its `old_string` is absent.

Gate matching (`acceptance`, `protected-paths`) compares paths with Unicode case folding on every filesystem (each
character upper-cased then lower-cased, so `ß` folds to `ss`; names are not Unicode-normalised), judges an edit
through a symlink as the file it resolves to, and on Unix judges an edit through a hard link (a file with another
link, found by device and inode among the files under a protected path and the directories of a gated record type)
as that protected or gated file. Off Unix a hard link is judged by its own name. The engine names no agent: how an
agent's tools become a request is the adapter's.

### The post-command verdict

`urzua harness run post-command` (`ADR-78`, amendment; `MILE-202`) runs after a shell command and
reports; it never blocks and always exits 0. It does not read the command: it lists the changed files with
`git status --porcelain=v1 -z --no-renames --untracked-files=all` (modified, added and untracked files; a
deleted file is dropped from the records it checks but kept for protected paths) together with the files
that differ between the commit the last post-command check saw and `HEAD` (so a command that committed its
own change is covered), keeps those a record type governs, drops those whose content hash is the one last
reported, and runs `check PATHS` once over the rest, in process, staging nothing. Its verdict carries these
extra fields:

| Field | Meaning |
|---|---|
| `findings` | The unwaived findings the checked records have now and did not have at `HEAD`, as `urzua check` prints them. |
| `gated_moves` | One line per changed record the command moved to a status the `authority` table gates (a new record, or one whose status at `HEAD` was another), judged against that commit (`HEAD` when the command committed nothing) when `acceptance` is enabled. Reported as errors of their own, never blocked; empty when none, and reported even when `check` cannot run. |
| `protected_changes` | One line per path under an `authority` path gate that the command changed, created, deleted or committed, when `protected-paths` is enabled. Matched with Unicode case folding. Includes a file git does not show (a gitignored one): the harness keeps a content hash of every file under a protected path in `.urzua/cache/protected-hashes.json` (seeded by `session-start`, refreshed by each check) and reports a file whose hash differs, that is new or that is gone. Reported as errors of their own, never blocked, once per content hash: the same change is not reported again. |
| `pre_existing` | The unwaived findings the same record already had at `HEAD`, judged from its committed text (a record with no copy at `HEAD` has none). Listed, not new. |
| `checked` | The records `check` ran on this time. Empty when no check ran: a clean tree, no governed record changed, the guardrail is off, or every changed record's content hash was already reported. |

`allow` is a check that found nothing, or none ran (the message says which); when the protected-path hash scan
stopped at its cap, the verdict is `allow-with-warnings` with the message saying some files were not hashed,
even if no finding was made. `allow-with-warnings` lists the
findings with file, line, severity, rule and message, says that the command had already run so nothing was
prevented, and that pre-commit and CI remain the hard gate. `allow-unjudged` is stated, never a quiet pass,
for a `git` that cannot be run, a working tree git does not know, a configuration that does not load, an
unreadable request and a `check` that cannot run; the message contains "NOT judged". The memory of reported
hashes is `.urzua/cache/post-command.json` (derived, ignored by git): it keeps exactly the records and protected
paths that are changed now, so a reverted one is forgotten, and a clean tree removes it. The commit the next check
starts from is `.urzua/cache/post-command-head`, written by every check that judged and by `session-start`; when
it is absent or no longer in the history the working tree is compared with `HEAD` only. The protected hashes file also holds the protected entries and the hash of the configuration file they were read from; when the configuration is unchanged and every protected file hashes as before, a clean tree ends the check before the configuration is parsed. With no baseline yet (no `session-start`), the first check records one and can report nothing for a gitignored file. The walk stops at 50 000 entries and says so. The guardrail takes no
`on_unjudged`.

### The session-start verdict

`urzua harness run session-start` reports whether the harness can do its job in this session, and never
blocks. Its verdict is `allow` (the harness is installed and wired, or the `session-start` guardrail is not
enabled) or `allow-with-warnings` with one `{check, message}` in `findings` for each thing that would make a
guardrail fail open or not run: a problem in the `harness` key, `urzua` not on `PATH`, a
missing `.urzua/harness.lock`, a bundle older or newer than the binary, an adapter the lock names that this
binary does not embed, a `requires.interpreters` program not on `PATH`, a `requires.env` variable that is unset,
a hook entry of the lock that is not wired in the settings file, and a git hook problem. A configuration that
does not load is `allow-with-warnings` with the message alone. When the `post-command`
guardrail is enabled it also records the commit and the protected-path hashes the first post-command check of
the session starts from.

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
| `provides` | The guardrails the adapter can wire (`editor-hook`, `post-command`, `session-start`, ...). A guardrail the repository turns on that no chosen adapter provides is reported at install, by `status` and by `doctor`, never skipped. |
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
| `entries` | Each merged hook entry: `file`, `event`, `matcher` (absent for none), `handler` and the `hash` (`sha256:<hex>`) of its compact JSON. |
| `blocks` | Each instructions file as `file` and the `hash` (`sha256:<hex>`) of the block between its markers, markers included. |

`status` compares the lock with the repository and this binary: a path or entry that is gone is `missing`,
one whose hash differs from the lock's is `modified`, one that matches the lock but not this binary's
bundle is `out-of-date`. `update` rewrites an item whose hash still matches the lock and leaves (and
reports) a modified one unless `--force`; it keeps the old hash for what it left, so `status` keeps
reporting it. `uninstall` removes what matches the lock, takes only urzua's entries and the marked block
out of the shared files, and deletes the lock last.

## What's deliberately not built

- A guardrail no adapter provides. `harness.guardrails` may name one that the chosen adapters do not list in
  `provides`; the install, `status` and `doctor` report it, and nothing skips it silently.
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
> | 2026-11-06 | Bumped to `0.2`. Adds the `post-command` event: its request (no paths, git names them), its verdict fields `findings`, `pre_existing` and `checked`, the stated `allow-unjudged` cases, the `gated_moves` field (a Bash-made move to a gated status, `MILE-200`), and the hash memory under `.urzua/cache`; the Claude Code adapter provides it. **Why:** `MILE-202`, decided in the `ADR-78` amendment and `ADR-89`. | **substantive** |
> | 2026-11-06 | Bumped to `0.3`. The `block` verdict also covers an agent's move to a gated status or write to a gated path (`acceptance`, `protected-paths`, `MILE-200`), final and not liftable by an override; the guardrails now run whether or not `editor-hook` is enabled. **Why:** `ADR-89` section 4. | **substantive** |
> | 2026-11-06 | Bumped to `0.4`. The adapter layout gains `commands/` and the manifest key `commands_dir` (`MILE-199`); `status` reports a bundle file the lock does not know yet as `missing`. **Why:** `ADR-89` section 1. | **substantive** |
> | 2026-11-06 | Bumped to `0.5`. Drops the not-covered entry for cutting this repository over: it runs on the installed bundle (`SPEC-19`). **Why:** `MILE-204`. | **substantive** |
> | 2026-11-06 | Bumped to `0.6`. Adds the request field `adapter_error` and the adapter-failure rule for `on_unjudged: deny` (`acceptance`, `protected-paths`), the post-command verdict field `protected_changes`, the commit range and `post-command-head` memory, and the case-insensitive and symlink-resolving gate matching. **Why:** the second-pass review of the harness. | **substantive** |
> | 2026-11-06 | Bumped to `0.7`. An edit the translator cannot place is refused under `on_unjudged: deny` only when its file is a protected path or a gated record; any other file is allowed with a notice. An unreadable payload, which names no file, is still refused. **Why:** review of pull request 378: refusing every unplaceable edit overreached onto files no gate covers, where the edit tool fails by itself. | **substantive** |
> | 2026-11-06 | Bumped to `0.8`. Gate matching uses Unicode case folding and resolves a hard link (Unix); `protected_changes` includes gitignored protected files by content hash, kept in `.urzua/cache/protected-hashes.json`; an adapter may send several `paths` with `adapter_error` (the call is refused when any is a protected path or gated record). **Why:** the maintainer's decision of 2026-11-06 to close the guardrail gaps left by pull request 378, under `ADR-89`. | **substantive** |
> | 2026-11-07 | Bumped to `0.9`. Checked against `urzua harness`, the guardrail cores and the Claude Code translator, and corrected: the lock records the `hash` of an entry or block (the text said `sha256`); the not-built list no longer says the drafting commands (`MILE-199`) and the post-command check (`MILE-202`) are not built; an unreadable `session-start` request is read as empty, not a stated verdict; the verdict names its `path` and `override` fields; a truncated protected-path scan makes the post-command verdict `allow-with-warnings`; the `session-start` verdict is described. **Why:** the maintainer approved moving this spec to `Shipped` (2026-11-07), which required verifying each section against the built code. | **substantive** |
> | 2026-11-07 | Bumped to `0.10`. Status changed from Draft to Accepted. **Why:** Maintainer decision, 2026-11-07: the maintainer approved moving this spec to `Shipped`, and the lifecycle moves one step at a time (`Draft` to `Accepted` to `Shipped`). | **substantive** |
> | 2026-11-07 | Bumped to `0.11`. Realized-by names the implementing code and tests, ready for `Shipped` (`MILE-7`). **Why:** Maintainer decision, 2026-11-07: moving this spec to `Shipped` requires `Embodiment` and `Realized-by`. | **structural** |
> | 2026-11-07 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/harness/run.rs, code:rust/crates/urzua-cli/src/harness/post_command.rs, code:rust/crates/urzua-cli/src/harness/gate.rs, code:rust/crates/urzua-cli/src/harness/lock.rs, code:rust/crates/urzua-cli/src/harness/mod.rs, code:rust/crates/urzua-cli/bundle/claude-code/files/.claude/hooks/urzua-claude-code.py, test:rust/crates/urzua-cli/tests/harness_integration.rs, test:rust/crates/urzua-cli/tests/harness_hardening_integration.rs, test:rust/crates/urzua-cli/tests/harness_post_command_integration.rs, test:rust/crates/urzua-cli/tests/harness_install_report_integration.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
