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
    editor-hook:   {enabled: true, on_unjudged: allow}
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
| `urzua harness status` | Compares the lock with the files, the hook entries, the instructions block and the binary. Reports what is missing, modified or out of date. Exit 1 on any. |
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
