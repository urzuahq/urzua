# The agent harness

If an AI agent writes records in your repository, urzua can judge its edits before they land. The
harness is a bundle, embedded in the binary and versioned with it, that you install into the
agent's project settings. It does three jobs:

- **Judge an edit** to a record before it is written, and say what is wrong with it.
- **Report after a shell command** which records the command changed, and run `check` on them.
- **Refuse an agent's move** of a record to a status a person must set (accepting a decision), and
  an agent's write to a protected path, where you have declared who may do those things.

Nothing is enabled unless the config names it.

## Install

Declare what you want in `.urzua/config.yaml`:

```yaml
harness:
  adapters: [claude-code]
  instructions_file: AGENTS.md
  outside_writes: shared
  guardrails:
    editor-hook:   {enabled: true, on_unjudged: allow, pending_pair: allow}
    session-start: {enabled: true}
    post-command:  {enabled: true}
```

Then:

```sh
urzua harness adapters          # the adapters this binary embeds
urzua harness install --dry-run # what would change
urzua harness install
```

`install` writes, for each adapter:

- the hook files (for `claude-code`: `.claude/hooks/urzua-claude-code.py` and
  `.claude/hooks/urzua-record.sh`);
- drafting commands for the agent, one per record type plus `amend`, `done`, `repin` and `reverify`
  (for `claude-code`: `.claude/commands/*.md`);
- hook entries merged into the agent's project settings (`.claude/settings.json`);
- one block in the instructions file, between `<!-- urzua-harness:begin -->` and
  `<!-- urzua-harness:end -->`, telling the agent what it must not do;
- `.urzua/harness.lock`, recording a hash of everything written.

It never overwrites a file that differs, refuses a settings file it cannot merge into (exit 2), and
run twice produces identical bytes. Commit all of it. With no `harness` key and no `--adapter`,
`install` writes nothing and says so. The hooks need `python3` on the machine the agent runs on.

## Keep it in sync

| Command | Does |
|---|---|
| `urzua harness status` | Compares the lock with the files, the hook entries, the instructions block and the binary. Reports what is missing, modified or out of date, and a setting that switches every hook off (`disableAllHooks`). Exit 1 on any. |
| `urzua harness update [--force] [--dry-run]` | Rewrites the files whose hash still matches the lock. A file a person changed is reported and left alone unless `--force`. Run it after upgrading urzua. |
| `urzua harness uninstall [--dry-run]` | Removes the files whose hash matches the lock, takes the hook entries and the block out of the shared files, reports what a person changed and leaves it, and deletes the lock last. |

## The guardrails

A guardrail is on only when named with `enabled: true`. The names are fixed; a name the build does
not know is a load error, with the nearest valid name suggested.

| Guardrail | Judges | Default on an unjudged edit |
|---|---|---|
| `editor-hook` | A write to a record, before it lands: runs the rules that can judge one record against the proposed content (`check --stdin-path`). | `allow`, with a message |
| `session-start` | At the start of a session: that the harness can do its job. | not applicable |
| `post-command` | After a shell command: runs `check` on the records git shows as changed, and reports. It never blocks. | not applicable |
| `acceptance` | An agent's move of a record to a status your `authority` table reserves for a person. | `deny` |
| `protected-paths` | An agent's write to a path your `authority` table protects (the harness's own files, CI workflows, `.urzua/config.yaml`, anything you list). | `deny` |

### A change that needs two records to agree

Some rules judge two records together: under a declared relation pair (`relation.pair-reciprocity`) or the
combined supersession field (`relation.supersession-reciprocity`), if record A says it supersedes B, B must say
it is superseded by A. The editor hook judges one file at a time, so neither half matches until both exist.

`editor-hook.pending_pair` decides what happens to the first half:

- `allow` (the default): an edit whose only new errors are such findings, each satisfied by one edit of
  another record, is allowed. The message the agent reads names the other record, the field and value to add,
  and the command that writes it, `urzua fix --apply --ids <other record> --by <you> --actor agent`, which
  changes only that field and adds a revision-log row. It never changes a `Status`: a move to `Superseded` is a
  separate step, and gated where your `authority` table names it.
- `refuse`: the edit is refused as it was before this key existed, and the refusal names the same command.

Any other new error in the same edit still refuses, and the gates are unchanged: an edit that writes the second
half and moves the record to a gated status is still refused for the status. A pair left unmatched is still a
`urzua check` finding, so the post-command hook reports it and pre-commit and CI block it. The default `allow`
is the maintainer's decision of 2026-11-16; set `pending_pair: refuse` for the earlier behaviour.

`on_unjudged` (`allow` or `deny`) is what happens when the guardrail cannot run: the hook is missing,
the config does not load. `acceptance` and `protected-paths` default to `deny` because an unjudged
gate is not a gate; the agent's edit is refused, and a person's is not, since a person edits outside
the agent. `acceptance` and `protected-paths` need an `authority` table in the config (a load error
otherwise), which names the decider groups and the gates:

```yaml
authority:
  decider_table: base        # read who may decide from the pull request's base revision
  deciders:
    maintainers: {logins: [alex]}
  gates:
    - {type: adr, to: [Accepted, Rejected, Superseded], by: maintainers}
    - paths: [".github/workflows", ".urzua/config.yaml"]
      by: maintainers
```

See `SPEC-3`, "Who may accept a record", for the full table. In CI the same table is enforced on a
pull request by the `diff.decider-required` rule, fed by the gate action's `actors` input
([CI](ci.md)).

`outside_writes` decides how a write outside the session's repository is judged: `allow` (leave it to
that repository), `shared` (also judge a file of a linked worktree of the same repository by the
same gates; recommended) or `deny` (refuse an agent's write outside, except under `allow_paths`).
`urzua init` proposes `shared`.

An agent's write that is allowed and lands outside the repository (a note, a scratch file) prints
nothing. Set `report_outside_writes: true` under `harness` to see the old notice, "urzua did NOT judge
this edit ... is outside the repository", for each such write. A refused write, and an edit inside the
repository that urzua could not judge, are reported either way.

The post-command check prints nothing for a command that changed nothing governed. A gated move or a
protected-path change is reported once, to you, however it arrived; when it came in by a `git pull`,
`checkout`, `merge`, `reset` or `stash pop` the message says so. Ordinary findings in records a pull or
checkout brought in are not reported. The agent gets a short line; you get a message only for a gated
move, a protected-path change, or a check that could not run.

## Limits

`limits` sets how much of one agent call the harness reads, and how long the agent waits for the
editor hook. Every key is optional; one you leave out keeps its built-in value, so a config without
`limits` behaves exactly as before.

```yaml
harness:
  limits:
    hook_timeout_seconds: 180   # 120 to 600; default 120
    paths_per_call: 1024        # default 512, and never lower
```

Every key is a security floor: you may raise it, never lower it. That includes the hook timeout,
because Claude Code lets a call through when the hook runs past it. A lower
value, a key urzua does not know, or a value that is not a whole number stops the config from
loading, with a message naming the key and the floor. Raising a cap moves only the number. A call
beyond it is still an edit that was not judged, and `on_unjudged` decides what happens to it.

| Key | Default | Bounds |
|---|---|---|
| `hook_timeout_seconds` | 120 (at most 600) | seconds the agent waits for the editor hook |
| `paths_per_call` | 512 | files of one call judged against the gates |
| `records_per_call` | 8 | record-checking writes of one MCP call judged |
| `loose_strings_per_call` | 512 | other strings of one MCP call looked at as possible paths |
| `input_depth` | 24 (at most 256) | nesting levels of one MCP call's input read |
| `path_length` | 4096 | characters of a string read as a path |
| `string_length` | 65536 | characters of a string looked into for paths |
| `symlink_hops` | 40 | symbolic links followed resolving one path |
| `json_depth` | 128 (at most 1024) | nesting levels of a settings file read |
| `mcp_line_bytes` | 1048576 (at most 67108864) | bytes of one request line `urzua mcp` reads |
| `gate_walk_files` | 50000 | files walked under the path-gate entries |

`urzua harness limits` prints the values in force. The translator asks it for them before it scans
an MCP call; if it gets no answer it keeps the built-in values. `hook_timeout_seconds` is written
into the installed hook entry, so after changing it run `urzua harness update`; until then `urzua
harness status`, `urzua doctor` and the session-start check report the entry as out of date.

## What it can and cannot promise

The hooks bind an agent that goes through its edit tools. They do not see a shell command that writes
a record (the `post-command` guardrail reports after the fact, and never blocks), `git commit
--no-verify`, a pull request that changes its own workflow, or an agent acting under a person's own
credentials. The instructions block says so to the agent in plain words. CI is the guarantee; the
harness shortens the loop.

## What the hooks run

The hook entries call `urzua harness run`, which a person never needs to run by hand. It reads a
neutral request as JSON on stdin and prints a verdict as JSON on stdout:

```text
urzua harness run edit            # judge a proposed edit to a record
urzua harness run session-start   # check the harness can do its job
urzua harness run post-command    # run check on what a shell command changed
```

The request, verdict and adapter layout are specified in [`SPEC-24`](../specs/SPEC-24-the-harness-request-verdict-and-adapter-layout.md),
for anyone writing an adapter for another agent.

## Checking it works

`urzua doctor` reports whether the harness is declared and installed (`harness-config`,
`harness-installed`), and warns when a guardrail is enabled without `outside_writes` declared.
`urzua harness status` should print `"status": "ok"`.
