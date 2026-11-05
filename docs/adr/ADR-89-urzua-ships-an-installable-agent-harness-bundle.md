---
Stable-Id: 01M6NWW8WX0N9Z2R86HX2E9SBM
Status: Proposed
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: ADR-69, ADR-70, ADR-71
---
# 89 — urzua ships an installable agent harness bundle

## Context

`ADR-69` to `ADR-71` and `ADR-78` built an enforcement harness for agents that edit records: a hook that
relays `urzua check --stdin-path`, three drafting commands, and a layered gate in which only CI is relied
on. It exists only in this repository's checkout, and a review of it on 2026-11-05 found that it holds back
less than its documents say. What is true today:

- **The editor hook does not see Bash.** Its matcher is `Write|Edit|MultiEdit` (`.claude/settings.json:5`)
  and the adapter returns for any other tool (`scripts/hooks/claude-code-record-edit.py:55`). A record
  written by a heredoc, `sed -i` or `tee` is never judged at the editor; the layers left are the
  pre-commit hook, which warns and exits 0 when the binary is absent (`.git-hooks/pre-commit`, `not_run`) and
  is skipped by `--no-verify`, and CI.
- **The hook fails open in several ways.** No `jq` (`scripts/hooks/record-edit-hook.sh:71`), no binary
  (`:69`) and an unreadable payload all allow the edit with a message. A missing `python3` or an unset
  project-directory variable makes the command itself exit non-zero, which the agent harness treats as
  non-blocking, so the agent is told nothing. Nothing checks at the start of a session that the hook can run.
- **Its override is the agent's to take.** The override name is read from an environment variable
  (`record-edit-hook.sh:48`), which any process the agent starts controls.
- **Acceptance is a plain transition.** The `adr` type allows `Proposed -> Accepted` and `Rejected`
  (`.urzua/config.yaml:88-91`) and requires only that `Deciders` is not a placeholder (`:103`); nothing says who
  made the move. An agent `Edit` that sets `Status: Accepted`, `Deciders` and `Accepted-on` produces no
  finding at the editor, the commit or CI. Today an agent is stopped from self-accepting by the agent
  tool's own permission classifier and by prose in `AGENTS.md`, neither of which urzua owns or can test.
- **Nothing protects the guardrails.** An agent can edit `.claude/settings.json`, the hook scripts,
  `.git-hooks/`, `.urzua/config.yaml` (`block_on`, `override_log`, a rule's `level`) and `.github/workflows`
  with no finding, and there is no `CODEOWNERS` file in `.github/`.
- **There are three drafting commands for seven record types.** `.claude/commands/` holds `adr`, `rfc` and
  `milestone`. A bug filed already `Fixed` (allowed by `initial: ["Open", "Fixed"]`, `.urzua/config.yaml:301`)
  cannot go through the command path, because `draft-record.sh finish` refuses any changed scaffold value
  (`scripts/draft-record.sh:55-57`). There is no command for a bug, a waiver, a condition, an amendment to an
  Accepted ADR, a re-pin, or marking a milestone done.
- **Attestation is a string.** `fix --rebaseline --apply` takes `--by` and `resolve_identity` lets a
  verified `gh` login replace it, reporting the difference only as a `warning` field
  (`rust/crates/urzua-io/src/lib.rs:254-267`; callers `rust/crates/urzua-cli/src/commands/fix.rs:192` and `:423`).
  Where `gh` is unavailable `--by` is taken as given (`lib.rs:271-276`), so the same command is verified in one
  environment and asserted in another, and the revision row does not say which
  (`rust/crates/urzua-core/src/fix.rs:304`). Two re-pin reasons were found on 2026-11-05 that did not
  describe the change they covered.
- **An Accepted ADR was edited in place** on 2026-11-05 (the change to `ADR-85` was moved into a dated
  amendment afterwards, `docs/adr/ADR-85-*.md` revision log), and the maintainer reports that record numbers
  from `urzua new` have collided between branches; both were caught by a person reading a diff, not by the
  harness.
- **Adopters get none of it.** The only thing urzua installs into another repository is the CI gate action
  (`.github/actions/urzua-gate/action.yml:1-15`). A repository that adopts urzua has to write its own hook,
  commands and agent instructions, which is how a mature adopter ended up with a bundle of exactly this
  kind: record-type commands, lifecycle commands, an editor hook, a session-start check, and settings, all
  installed together.

The maintainer decided on 2026-11-05: urzua ships an agent harness (skills, slash commands, hooks, settings)
as an installable bundle, so that a repository which installs it has working guardrails; who may accept a
record is enforced by the harness and its CI and not by prose; the attestation identity source is a
configuration option; and Bash writes to records are caught by a check that runs after the command. This is
an agent guardrail and belongs in Phase 0.

## Options considered

How the bundle reaches a repository:

| Option | Pros | Cons |
|---|---|---|
| A. Document the files and let each repository copy them | Nothing to build | Every copy drifts; no version; a copy of a guardrail that is silently out of date reads as protection |
| B. `urzua harness install`, with the bundle embedded in the binary (chosen) | One version number with the engine; works offline; the installed files carry hashes, so drift is detectable; the engine already reads its own config and records | The bundle changes only with a release; the binary grows by the bundle's size |
| C. A separately published bundle (release asset, package or action) with its own version | Updates without a new binary | Two versions to keep compatible; a network fetch on install; an install that can pick a bundle the binary cannot drive |
| D. Only a vendor plugin or marketplace entry | Native for one vendor | Hardcodes the vendor in the distribution; no other agent can use it; outside urzua's tests |

Where the guardrail logic lives:

| Option | Pros | Cons |
|---|---|---|
| X. Keep shell and Python scripts as the cores | Exists, works today | The fail-open paths above are the shell and Python chain; each adapter reimplements the core |
| Y. The cores are engine subcommands speaking a neutral request and verdict; an adapter only translates (chosen) | One tested implementation; fail-open becomes a verdict the engine controls; no vendor in the engine | The scripts under `scripts/hooks/` are replaced; an adapter still has to exist for each vendor |

## Decision

In the context of agents that edit governed records through a harness that does not see Bash, can be
reconfigured by the agent, leaves acceptance and attestation to prose and cannot be installed anywhere else,
we decided to **ship the harness as a versioned bundle that `urzua harness install` writes into a repository,
with the guardrail logic in the engine behind a vendor-neutral contract, and to enforce acceptance by a
declared table of deciders that the local hook and CI both read**, to achieve guardrails that work
the day a repository installs them and that an agent cannot talk its way past, accepting a bundle that is
released with the engine and an adapter to write for each agent vendor.

### 1. What the bundle contains

Everything below is generated or copied by the install, so a repository holds no hand-written copy:

- **A drafting command for every record type the configuration declares**, named for the type
  (`/adr`, `/rfc`, `/milestone`, `/bug`, `/condition`, `/waiver`, `/spec` here). Each is the one generic
  `ADR-70` template; the type name and description come from `.urzua/config.yaml`, and the engine adds no
  schema knowledge. A new type in the configuration gets a command on the next install or update.
- **Lifecycle commands**, each a thin wrapper over a urzua command that does the checking:
  `/amend` (a dated `## Amendment` on an Accepted ADR, with a revision row), `/repin`
  (`fix --rebaseline`, naming the real delta as its reason and listing the re-pin for the reviewer),
  `/done` (sets `Realized-by` and the delivered status together, only for a type whose status the
  configuration derives from evidence, `ADR-84`). `/done` is also how a bug is filed `Fixed`: `/bug` always
  files at the scaffolded `Open`. `/done` never sets a status an acceptance gate names (section 4). It is read
  as outside `ADR-70`, which governs drafting commands and forbids them to set a completion or fix state;
  if the maintainer reads `ADR-70` as covering it, this item needs an amendment there.
- **The editor hook**, now `urzua harness run edit`: it judges a Write, Edit or MultiEdit as `ADR-69` and
  `ADR-78` describe, plus the acceptance and protected-path gates of section 4. It replaces the
  `jq`, `bash` and `python3` chain, so the paths that fail open today become a verdict the engine states
  (`allow-unjudged` is kept for the genuine cases of `ADR-71`, and is configurable per guardrail below).
- **The post-command check**, `urzua harness run post-command`, specified by the amendment to `ADR-78`.
- **A session-start readiness check**, `urzua harness run session-start`: the binary is found and its
  version matches the installed bundle, the hook entries are wired in the agent's settings, the git hooks
  are wired (`urzua doctor`, `MILE-179`), and the adapter's own prerequisites exist. It prints what would
  fail open, to the agent, at the start of the session.
- **Protected paths** (section 4) and **an agent-instructions block**: a section between begin and end
  markers in the repository's agent-instructions file, rewritten by the install and never touched outside the
  markers (the same marker mechanism as `index.freshness`), stating what an agent must not do (accept a
  record, set an override, edit a protected path).
- **A lock file**, `.urzua/harness.lock`: the bundle version, the adapter, and a hash of every installed
  file.

### 2. Installation and versioning

Option B. `urzua harness install [--adapter NAME] [--dry-run]` writes the files for the adapter named in
`--adapter`, or in `harness.adapters` in the configuration; `urzua harness status` compares the lock with the
files and the binary and reports missing, modified and out-of-date files; `urzua harness update` rewrites
files whose hash still matches the lock, and reports (and does not overwrite) a file a person changed unless
`--force`. The bundle version is the engine version, so a binary older than the lock is reported by
`urzua doctor`. Customisation goes in the configuration, not in the installed files, which is what makes
`update` safe. `--dry-run` writes nothing and lists what would change. The bundle directory is laid out so it
can later also be published as a release asset (option C) without a second format.

### 3. Opt-in configuration for each guardrail

A new top-level `harness` key; an absent key installs and enables nothing, and a guardrail is on only when named:

```yaml
harness:
  adapters: [claude-code]
  guardrails:
    editor-hook:    {enabled: true, on_unjudged: allow}   # allow | deny
    post-command:   {enabled: true}
    session-start:  {enabled: true}
    acceptance:     {enabled: true, on_unjudged: deny}
    protected-paths: {enabled: true, on_unjudged: deny}
```

`on_unjudged` says what the hook does when it cannot judge: `allow` with a loud message (the `ADR-71`
principle, the default for the editor hook) or `deny`. The acceptance and protected-path guardrails default to
`deny`, because an unjudged gate is not a gate. A key the build does not know, or a guardrail the chosen
adapter cannot provide (an agent with no post-command event), is reported at install and by `doctor`, never
skipped silently. Commands for record types are generated whenever the harness is installed and need no key.

### 4. The acceptance guardrail

The configuration names who may move a record to an accepting status, and the engine enforces it in two
places from one table. A top-level `authority` key (structured, `ADR-64`; the precedent for a top-level key
the hook and the engine both read is `override_log`):

```yaml
authority:
  deciders:
    maintainers: [beauwilliams]        # GitHub logins
  evidence: [pr-approver]              # pr-author | pr-approver, any of
  gates:
    - {type: adr, to: [Accepted, Rejected], by: maintainers}
    - {type: rfc, to: [Accepted, Rejected], by: maintainers}
    - {paths: [".claude", "scripts/hooks", ".git-hooks", ".github/workflows", ".urzua/config.yaml"], by: maintainers}
```

- **Local: an agent's edit is refused.** `urzua harness run edit` and `post-command` take the actor from the
  request (`agent`, set by the adapter, which is only ever invoked by an agent). For an agent, a change that
  moves a record of a gated type to a gated status, compared with the copy on disk or at `HEAD`, is
  **denied outright**, and so is a write under a gated path. This verdict cannot be overridden: the override
  of `ADR-78` is a name in an environment variable the agent controls, so it is not accepted for this
  verdict, and the output says so. A human edits outside the agent harness and is not affected.
- **CI: a pull request that makes such a move must be authored or approved by a declared decider.** A new
  opt-in rule `diff.decider-required` reads the base copy of every changed record (the mechanism the status
  transition rules already use, `rust/crates/urzua-cli/src/commands/check.rs:92-135`) and, for each gated
  move or gated path, requires that one of the configured `evidence` kinds names a login in the gate's group.
  The identities arrive as input, never fetched by the engine (`check` does not use the network): the gate
  action (`.github/actions/urzua-gate/action.yml`) writes a file from the GitHub API with the pull request's
  author and the logins whose approval is current for the head commit, and passes it as
  `urzua check --base REF --actors FILE`. A stale approval (given before a later push) does not count.
- **No silent no-op.** A gated move with no `--actors` file is reported, as `info` on a local run (pre-commit
  cannot know) and as an error when the option `require_evidence: true` is set, which the gate action
  sets. A `by` group that names no login, a login not in any group, and a gate whose `type` or `to` the
  configuration does not declare are load errors.
- **The protected paths protect the table itself.** `.urzua/config.yaml`, the harness files and the workflow
  that runs the gate are in the paths a decider must authorise, so lowering the gate is itself gated.
  This cannot be complete: the required status check and branch protection live in the host's settings,
  which the engine can read neither to enforce nor to verify. `urzua doctor` says so and does not claim it.

### 5. The attestation identity option

A top-level `attestation` key:

```yaml
attestation:
  identity: gh-verified        # gh-verified | asserted
  require_verified: false
```

- **`gh-verified`** (the default, today's behaviour): a verified `gh` login wins over `--by`; the report
  names the replacement as a field and not only prose. With `require_verified: true`, a command that writes
  an attestation (`fix --apply`, `fix --rebaseline --apply`) does not run (`not-run`, exit 2) when `gh` cannot
  verify, instead of falling back to `--by` or the git configuration.
- **`asserted`**: `--by` is taken as given, for a repository whose agents attest on behalf of a named human
  with no `gh` session. A verified login, when `gh` answers, is recorded beside it, not instead of it.
  `require_verified: true` is refused at load with `asserted`.
- **The identity source is recorded in the revision row** (`gh-verified`, `asserted`, or `git-config`), so a
  reader can tell a verified attestation from an asserted one without the report. The existing row text
  (`Tool-authored (by NAME): ...`) gains the source; `SPEC-8` is revised.

### 6. Vendor neutrality

The engine names no agent vendor. It speaks a neutral request (`event`, `tool`, `paths`, `content`,
`actor`, `cwd`) and a neutral verdict (`allow`, `deny`, `allow-unjudged`, with the message and findings), and
`urzua harness adapters` lists the adapter directories in the bundle. An **adapter** is a directory of
installable files: a settings fragment that wires the vendor's events to `urzua harness run <event>`, and
a translator that turns the vendor's hook payload and output into the neutral form, in the language the
vendor's hook runner accepts. Claude Code is the first adapter (its hook entries and its translator are today's
`.claude/settings.json` and `claude-code-record-edit.py`). Another agent adds a directory and a name; the
engine, the configuration and the guardrails do not change. A guardrail an adapter cannot provide is
declared in the adapter and reported (section 3), and the CI gate still holds for every agent.

## Reversibility

Moderate. The installed files, the lock and the `harness` key are additive and removable, and an absent key
changes nothing, so adopting is easy to undo. The three commitments that are not: the `authority` table and
`diff.decider-required` become a contract that an adopter's CI relies on; the neutral request and verdict
become an interface adapters are written against; and the revision-row text gains the identity source, which
history rows then cite. The cost of being wrong is a gate that blocks a legitimate acceptance (a decider
reaches the PR late), or a lock file that reports drift nobody acts on.

## Consequences

- `SPEC-3` gains `harness`, `authority` and `attestation`; `SPEC-2` the rule `diff.decider-required` and
  `--actors`; `SPEC-8` the identity source; `SPEC-19` is revised, since the harness is now more than
  `AGENTS.md`; a new spec for the neutral contract and the adapter layout is for the first milestone.
  `make rule-table-check` covers the new rule.
- `scripts/hooks/*.sh` and `*.py` are replaced by the engine subcommands and one translator. The tests that
  pin their behaviour move to the engine and keep their planted cases.
- The strength of the CI check depends on the host: on a repository where agents run under a
  decider's own token, the pull-request author is the decider, so `pr-author` evidence proves nothing there
  and `pr-approver` is the meaningful one. A single-maintainer repository cannot approve its own pull
  request; the local refusal is then the barrier, and CI proves only authorship. This is recorded in the
  open question, not hidden.
- The deny verdict for an agent edit to an accepting status makes an agent unable to accept a record even
  when a person asks it to in conversation. The person edits or approves; that is the intent.
- A bundle older than the binary is a finding at session start and in `doctor`, so every release that
  changes the bundle asks installed repositories to run `update`.
- Every adapter is another thing to keep working; one with no post-command event holds the other guardrails
  and relies on pre-commit and CI for Bash writes.
- The agent-instructions block is prose, and stays advisory; only the engine verdicts bind.
- Milestones `MILE-198` to `MILE-202` build this and are not blocked on each other beyond their `Depends-on`.

## References

- `ADR-69`, `ADR-70`, `ADR-71`: the hook, drafting commands and layered gate this extends; `ADR-78`: the
  override log and `block_on`, which gains an amendment for the post-command check.
- `ADR-64`: structured in, structured out. `ADR-80`: declared boundaries. `ADR-84`: status follows evidence.
- `ADR-87`: the `info` level used for an unverified local run.
- `SPEC-2`, `SPEC-3`, `SPEC-8`, `SPEC-19`: the specs this revises.
- `MILE-19` (human attestation with expiry) and `MILE-79` (a guarded edit command) are related and not
  replaced; `MILE-179` (git hooks and `doctor`) is the readiness check this extends.
- `MILE-198`, `MILE-199`, `MILE-200`, `MILE-201`, `MILE-202`: the build.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** Maintainer decisions, 2026-11-05 (ship an installable harness bundle in Phase 0; acceptance enforced by the harness and CI; attestation identity source a configuration option); the maintainer accepts it in a later change. | **substantive** |
