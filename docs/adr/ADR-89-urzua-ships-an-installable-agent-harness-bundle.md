---
Stable-Id: 01M6NWW8WX0N9Z2R86HX2E9SBM
Status: Accepted
Accepted-on: 2026-11-05
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
- **The hook fails open in several ways, and closed in one.** No `jq` (`scripts/hooks/record-edit-hook.sh:71`),
  no binary (`:86`, through the `unjudged()` helper at `:69`) and an unreadable payload all allow the edit with
  a message. A missing `python3` makes the command exit 127, which the agent harness treats as non-blocking,
  so the agent is told nothing. An unset `CLAUDE_PROJECT_DIR` fails the other way: the command becomes
  `python3 -I "/scripts/hooks/..."`, `python3` cannot open that file and exits 2 (reproduced on 2026-11-05),
  and 2 is a blocking exit, so every Write, Edit and MultiEdit is refused with no verdict that says why.
  Both come from one gap: nothing checks at the start of a session that the hook can run. The session-start
  check (section 1) reports a missing interpreter or an unset project-directory variable, and the adapter's
  command does not depend on that variable (the translator takes the project directory from the request's
  `cwd`), so an unset variable becomes a stated verdict and not an opaque block. That fix is part of `MILE-198`.
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
  Where `gh` is unavailable `--by` is taken as given (`lib.rs:272-276`), so the same command is verified in one
  environment and asserted in another, and the revision row does not say which
  (`rust/crates/urzua-core/src/fix.rs:304`). Two re-pin reasons were found on 2026-11-05 that did not
  describe the change they covered. The tier order is deliberate (`RFC-2`, `MILE-66`: a verified login beats a
  typed name), and `BUG-16` records that no tier proves who is acting. In particular a verified login says
  nothing about whether a person or an agent holding that login's token ran the command.
- **An Accepted ADR was edited in place** on 2026-11-05 (the change to `ADR-85` was moved into a dated
  amendment afterwards, `docs/adr/ADR-85-*.md` revision log), and the maintainer reports that record numbers
  from `urzua new` have collided between branches; both were caught by a person reading a diff, not by the
  harness.
- **Adopters get none of it.** The only thing urzua installs into another repository is the CI gate action
  (`.github/actions/urzua-gate/action.yml:1-15`). A repository that adopts urzua has to write its own hook,
  commands and agent instructions, and every hand-written copy drifts from the engine.

The maintainer decided on 2026-11-05: urzua ships an agent harness (skills, slash commands, hooks, settings)
as an installable bundle, so that a repository which installs it has working guardrails; who may accept a
record is enforced by the harness and its CI and not by prose; the attestation identity source is a
configuration option; and Bash writes to records are caught by a check that runs after the command. This is
an agent guardrail and belongs in Phase 0. The same day the maintainer decided, on the question of agent
identity, that the repository documents the limits plainly and accepts the risk for now (section 4, "Accepted
risk").

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
with the guardrail logic in the engine behind a vendor-neutral contract, and to declare who may accept a
record in one table that the local hook enforces for the editor tools and that CI reports against**, to
achieve guardrails that work the day a repository installs them and that stop in-band self-acceptance through
the edit tools and leave an authorship trail in CI, accepting a bundle that is released with the engine, an
adapter to write for each agent vendor, and the bypasses listed under "Accepted risk" in section 4, which the
maintainer accepted on 2026-11-05.

### 1. What the bundle contains

Everything below is generated or copied by the install, so a repository holds no hand-written copy:

- **A drafting command for every record type the configuration declares**, named for the type
  (`/adr`, `/rfc`, `/milestone`, `/bug`, `/condition`, `/waiver`, `/spec` here). Each is the one generic
  `ADR-70` template; the type name and description come from `.urzua/config.yaml`, and the engine adds no
  schema knowledge. A new type in the configuration gets a command on the next install or update.
- **Lifecycle commands**, each a thin wrapper over a urzua command that does the checking:
  `/amend` (a dated `## Amendment` on an Accepted ADR, with a revision row), `/repin`
  (`fix --rebaseline`, naming the real delta as its reason and listing the re-pin for the reviewer),
  `/done` (sets the `Realized-by` evidence and then runs `urzua fix --apply`, so the delivered status is the one
  the engine computes from that evidence, only for a type whose status the configuration derives from evidence,
  `ADR-84`; the command never writes a status itself). `/done` is also how a bug is filed `Fixed`: `/bug`
  always files at the scaffolded `Open`. `/done` never sets a status an acceptance gate names (section 4).
  `ADR-70` forbids a drafting command to write `Done` or `Fixed`; a dated amendment to `ADR-70`
  (2026-11-05, proposed, pending maintainer acceptance) lets a lifecycle command set a status only when the
  engine computes it from evidence, never by an agent's choice, and that amendment is what `/done` stands on.
- **The editor hook**, now `urzua harness run edit`: it judges a Write, Edit or MultiEdit as `ADR-69` and
  `ADR-78` describe, plus the acceptance and protected-path refusals of section 4. It replaces the
  `jq`, `bash` and `python3` chain, so the paths that fail open today become a verdict the engine states
  (`allow-unjudged` is kept for the genuine cases of `ADR-71`, and is configurable per guardrail below).
- **The post-command check**, `urzua harness run post-command`, specified by the amendment to `ADR-78`.
- **A session-start readiness check**, `urzua harness run session-start`: the binary is found and its
  version matches the installed bundle, the hook entries are wired in the agent's project settings, the git hooks
  are wired (`urzua doctor`, `MILE-179`), and the adapter's own prerequisites exist (the interpreter, and the
  project-directory variable the vendor sets). It prints what would
  fail open, to the agent, at the start of the session.
- **Protected paths** (section 4) and **an agent-instructions block**: a section between begin and end
  markers in the repository's agent-instructions file, rewritten by the install and never touched outside the
  markers (the same marker mechanism as `index.freshness`), stating what an agent must not do (accept a
  record, set an override, edit a protected path). The file is named in the configuration
  (`harness.instructions_file`; `AGENTS.md` here), and the engine does not assume a name.
- **A lock file**, `.urzua/harness.lock`: the bundle version, the adapter, and a hash of every installed
  file, including the hook entries the install merged into the agent's settings.

### 2. Installation and versioning

Option B. `urzua harness install [--adapter NAME] [--dry-run]` writes the files for the adapter named in
`--adapter`, or in `harness.adapters` in the configuration; `urzua harness status` compares the lock with the
files and the binary and reports missing, modified and out-of-date files, and exits 1 on any drift so a CI
step can run it (it is a separate step and not part of `urzua check`); `urzua harness update` rewrites
files whose hash still matches the lock, and reports (and does not overwrite) a file a person changed unless
`--force`; `urzua harness uninstall` removes the files whose hash still matches the lock, takes the urzua
hook entries and the marked instructions block out of the shared files, reports and leaves a file a person
changed, and deletes the lock last. The bundle version is the engine version, so a binary older than the lock is reported by
`urzua doctor`. Customisation goes in the configuration, not in the installed files, which is what makes
`update` safe. `--dry-run` writes nothing and lists what would change. Rules for files the install shares with a person:

- **An existing agent settings file is merged, never replaced.** The install adds its hook entries to the
  project settings file (`.claude/settings.json` for the first adapter), keyed by event, matcher and command;
  every other key and entry is left as written; an entry already present is not duplicated, so a second install
  changes nothing. An entry is urzua's when its command runs `urzua harness run`, which is how `update` and
  `uninstall` find it. If the file is not valid JSON, or holds an entry with the same key that a person edited,
  the install refuses (`not-run`, exit 2) and names the entry; it never guesses.
- **Only the project settings file is written.** A local settings file (`settings.local.json`) and the
  user-level settings are neither read nor written: the install cannot see them and says so in `status`.
- **The agent-instructions block is idempotent.** The install rewrites only the text between its markers and
  adds the markers once at the end of the file when absent; running it twice yields identical bytes.

The bundle directory is laid out so it
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
`acceptance` or `protected-paths` enabled with no top-level `authority` key (section 4) is a load error naming
the guardrail: a gate with nothing to enforce is not left to read as protection.

**The escape hatch for `on_unjudged: deny`.** A denying guardrail that cannot judge (a broken `authority`
table, a missing binary, a version mismatch) refuses an agent's edit to the protected paths, including
`.urzua/config.yaml`, the file that would repair it, and the override of `ADR-78` is deliberately not accepted
for this verdict. The way out is a person: a human edits outside the agent harness (a plain editor or a
terminal), where no hook runs. `urzua doctor` and the session-start check say so in the message that reports
the unjudged state, and a test pins both that message and that a request with actor `human` is allowed when
the guardrail is unjudged. A repository that does not want this lockout sets `on_unjudged: allow` for the
guardrail, and the loud message remains.

### 4. The acceptance guardrail

The configuration names who may move a record to an accepting status. One table is read in two places, and
what each place can and cannot do is stated here and nowhere claimed beyond it. A top-level `authority` key
(structured, `ADR-64`; the precedent for a top-level key the hook and the engine both read is `override_log`):

```yaml
authority:
  deciders:
    maintainers: {logins: [beauwilliams]}
  evidence: [pr-author]                # pr-author | pr-approver, any of; the default is pr-author
  gates:
    - {type: adr, to: [Accepted, Rejected], by: maintainers}
    - {type: rfc, to: [Accepted, Rejected], by: maintainers}
    - {paths: [".claude", "scripts/hooks", ".git-hooks", ".github/workflows", ".urzua/config.yaml"], by: maintainers}
```

`evidence` names the pull-request facts the host supplies (`pr-author`, `pr-approver`) and a decider group lists
`logins`; both are the host's vocabulary, read from a file the gate writes. The default is `pr-author`. It is
not `pr-approver`, because GitHub bars a pull request's author from approving it, so on a single-maintainer
repository `pr-approver` could never be satisfied and no acceptance could merge.

**What the guardrail does, exactly:**

- **Locally, it refuses the edit tools.** `urzua harness run edit` takes the actor from the request (`agent`,
  set by the adapter, which is only ever invoked by an agent). For an agent, a Write, Edit or MultiEdit that
  moves a record of a gated type to a gated status, compared with the copy on disk or at `HEAD`, is **denied
  outright**, and so is a write under a gated path. This verdict cannot be overridden: the override of `ADR-78`
  is a name in an environment variable the agent controls, so it is not accepted for this verdict, and the
  output says so. A human edits outside the agent harness and is not affected.
- **After a Bash command, it reports and does not block.** `urzua harness run post-command` (`ADR-78`,
  amendment) sees a gated move or a change under a gated path made by a shell command, and reports it to the
  agent as an error-level finding in its own right, not only as the generic record finding. The command has
  already run, so nothing is refused.
- **In CI, the only evidence is authorship, and it is always the maintainer's.** A new opt-in rule
  `diff.decider-required` reads the base copy of every changed record (the status-transition mechanism,
  `diff_scope::committed_statuses`, `rust/crates/urzua-cli/src/commands/check.rs:770`; the base-text loader
  at `:104-135` serves the version-bump rule and is not the one used) and, for each gated move or path,
  requires that one of the configured `evidence` kinds names a login in the gate's group. The identities arrive
  as input, never fetched by the engine (`check` does not use the network): the gate action
  (`.github/actions/urzua-gate/action.yml`) writes a file from the GitHub API with the pull request's author and
  the logins whose approval is current for the head commit, and passes it as
  `urzua check --base REF --actors FILE`. A stale approval (given before a later push) does not count.
  Agents act under the maintainer's GitHub token, so with a single maintainer every pull request an agent opens
  is authored by a decider and the rule passes. **With a single maintainer, CI cannot enforce who accepted.**
  What it gives is a recorded trail and, in a repository that has a second human or a distinct agent
  identity, a check that means something.
- **No silent no-op.** A gated move with no `--actors` file is reported, as `info` on a local run (pre-commit
  cannot know) and as an error when the option `require_evidence: true` is set, which the gate action
  sets. A `by` group that names no login, a login in no group, and a gate whose `type` or `to` the
  configuration does not declare are load errors.
- **The protected paths name the table and the harness.** `.urzua/config.yaml`, the harness files and the
  workflow that runs the gate are in the paths a decider must authorise, so an edit-tool change to them is
  refused locally. This is not a barrier against the routes listed next, and the required status check and
  branch protection live in the host's settings, which the engine can read neither to enforce nor to verify.
  `urzua doctor` says so and does not claim it.

**Accepted risk.** The maintainer decided on 2026-11-05: "We just need to clearly document this for now and
accept the risk." These routes get past the guardrail, and they are accepted (list revised 2026-11-06, see the
revision log):

1. **A Bash command that writes the status or a protected path.** `sed -i 's/^Status: Proposed/Status: Accepted/'`,
   `python -c`, `cp`, `tee` or a heredoc is outside the editor hook's matcher. The post-command check reports
   it and cannot stop it: a gated move, a changed protected path (a gitignored one such as
   `.claude/settings.local.json` included, by content hash against a baseline taken at session start) and a
   commit the same command made, and a record changed after `git update-index --assume-unchanged` or
   `--skip-worktree` (found by comparing the file with the index). A change is reported once; the same change
   again is silent, by design. A commit is seen only on the checked-out branch: a commit made on another
   branch or in a sibling worktree is silent until it lands on the session's `HEAD`, and a
   `git push origin side:main` of another ref is never seen. An MCP tool that runs a shell command is not a
   Bash tool call (the post-command matcher is `Bash` only): what it writes is neither judged nor reported.
   A Bash write to the translator is caught at the next hook run, because every hook command checks the
   translator's sha256 before running it (amendment of 2026-11-08), but the programs the command finds on
   `PATH` (`python3`, `urzua`) are not verified.
2. **`git commit --no-verify`**, which skips the commit hook.
3. **A pull request that changes its own workflow.** The gate runs from the head commit's workflow, so a pull
   request can drop or weaken the gate step it is judged by. Only `CODEOWNERS` and branch protection, which
   are host settings, can stop that, and neither is configured by this decision.
4. **An agent acting as the maintainer.** Agents use the maintainer's token. CI cannot tell an agent from the
   maintainer, so `pr-author` is always satisfied and no CI check can say who accepted.
5. **The limits of what the editor hook can read.** A hard link is resolved to the protected or gated file on
   Unix only, and elsewhere is judged by its own name. Names are case-folded but not Unicode-normalised. An MCP
   tool call that names no string a path can be read from (a path encoded, split or computed) is not seen as an
   edit; an MCP tool that applies a patch (a `patch` or `diff` field with `---` and `+++` headers) or runs a
   shell command is an unparsed route, not judged and not reported; and an MCP server that resolves a relative path against its own directory instead of the session's is
   judged against the session's. A change made before the session's baseline was taken is not reported. The
   agent's user-level settings and the project's local settings file are neither protected nor read: a hook
   entry or `disableAllHooks` set only there is not seen by the session-start check. A link or file swapped
   between the judgement and the write is not seen (the gap between check and use).
6. **A Bash command that writes what the harness keeps.** The edit tools cannot write `.urzua/cache`, the
   harness lock, the override log, `.git/hooks` or `.git/config`, but a Bash command can. A post-command
   baseline written ahead of a later change disarms the report for that change, an override row can be given
   a `Reviewed-by`, and a `core.hooksPath` turns the commit layer off; the post-command check does not report
   these files.
7. **The edit hook cannot run and the configuration cannot be read as text.** When python3 or the translator is
   missing, the settings command reads `.urzua/config.yaml` as text and refuses the edit only if it finds
   `acceptance` or `protected-paths` enabled without `on_unjudged: allow`; a configuration that scan misreads
   (an unusual layout, a key spelled by an anchor or an include) is read as not closed. The same holds when
   the configuration is invalid or empty YAML (only a Bash write can make one, and the post-command check
   reports it): the engine allows every edit with a message, because the gates are read from parseable YAML
   and the text scan is not used on that path. The refusal then
   covers every edit tool call, not only a protected one, because without the translator the target is not
   known. The post-command and session-start commands can only state that they did not run.
8. **Which repositories judge an edit.** An edit is judged by the session's own repository and by every
   governed repository (a real git working tree with `.urzua/config.yaml`) that holds the file, inner or
   outer, and a deny from any wins (amendment of 2026-11-08). A repository the session did not start in can
   add restrictions and never remove them; one made with a Bash `git init` and a configuration can therefore
   deny edits inside it, which costs the agent, not the guardrail.
9. **The agent-instructions block is not a protected path.** The block in `AGENTS.md` (or the file
   `harness.instructions_file` names) is a control this decision relies on, but an edit tool can remove it
   with no refusal and no session-start note; only `urzua harness status` reports it `modified`.
10. **A waiver record is not gated.** A waiver is an ordinary record, not a gated status, and
    `diff.decider-required` and `override.unreviewed` are waivable, so an edit-tool waiver can silence the CI
    gate for a gated move made by a Bash command. It adds no power beyond routes 1 and 4.

What is claimed is therefore only this: through the edit tools (Write, Edit, MultiEdit, NotebookEdit and any MCP tool
that carries a file path and content) an agent cannot move a gated record or write a
protected path; a Bash route is reported, not prevented; and in CI the evidence is authorship, which for a
single maintainer is always the maintainer's. The agent-instructions block and the person reading the diff stay
the controls for the rest. **A future option, not decided here:** give agents a distinct identity (a separate
GitHub App or bot account, with a fine-grained token) so that `pr-author` can tell them apart from the
maintainer, and add `CODEOWNERS` on the protected paths with branch protection requiring the code owner. A later
decision would record it; nothing in this one depends on it.

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
- **The identity source and the actor kind are recorded in the revision row.** The source is `gh-verified`,
  `asserted` or `git-config`; the actor kind is `agent`, `human` or `unspecified`. A verified login proves
  which account's token ran the command and not who held it, and with agents running under the maintainer's
  token a `gh-verified` row names the maintainer as the attester of what an agent did. That is the rubber stamp
  `RFC-2` and `BUG-16` describe, so the row says it: the lifecycle commands of section 1 pass `--actor agent`
  to `fix`, a direct run records `human` only when `--actor human` is given and `unspecified` otherwise, and
  the label is the caller's claim and not proof (the same standing as `--by`). A reader can tell a verified
  attestation from an asserted one, and an agent's from a person's, without the report. The existing row text
  (`Tool-authored (by NAME): ...`) gains both; `SPEC-8` is revised.
- **`asserted` reverses a principle** of `RFC-2` and `MILE-66` (a verified login beats a typed name). It is
  opt-in, never the default, and the row says `asserted`, so the reversal is visible where it applies.
  `MILE-18` (real identity binding for roles) is the longer-term answer; a `signed-commit` source (`BUG-16`'s
  own suggestion) is a possible later value of `identity` and is not decided here.

### 6. Vendor neutrality

The engine source names no agent vendor. The embedded bundle's adapter directory does, because an adapter is
exactly the vendor-specific part (the string `claude-code` and the `.claude/` paths); a test that scans for
vendor names covers the engine source and excludes the bundle directory. It speaks a neutral request (`event`, `tool`, `paths`, `content`,
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
become an interface adapters are written against; and the revision-row text gains the identity source and the actor kind, which
history rows then cite. The cost of being wrong is a gate that blocks a legitimate acceptance (a decider
reaches the PR late), or a lock file that reports drift nobody acts on.

## Consequences

- `SPEC-3` gains `harness`, `authority` and `attestation`; `SPEC-2` the rule `diff.decider-required` and
  `--actors`; `SPEC-8` the identity source; `SPEC-19` is revised, since the harness is now more than
  `AGENTS.md`; a new spec for the neutral contract and the adapter layout is for the first milestone.
  `make rule-table-check` covers the new rule.
- `scripts/hooks/*.sh` and `*.py` are replaced by the engine subcommands and one translator. The tests that
  pin their behaviour move to the engine and keep their planted cases.
- The strength of the CI check depends on the host: where agents run under a decider's own token, the
  pull-request author is the decider and `pr-author` proves nothing about who accepted. A single-maintainer
  repository cannot use `pr-approver`. The local refusal then covers the edit tools only, and the routes in
  "Accepted risk" (section 4) stay open until a distinct agent identity and `CODEOWNERS` exist.
- The deny verdict for an agent edit to an accepting status makes an agent unable to accept a record through
  the edit tools even when a person asks it to in conversation. The person edits or approves; that is the intent.
- A denying guardrail that cannot judge locks an agent out of the protected paths until a person edits outside
  the harness (section 3).
- A bundle older than the binary is a finding at session start and in `doctor`, so every release that
  changes the bundle asks installed repositories to run `update`.
- Every adapter is another thing to keep working; one with no post-command event holds the other guardrails
  and relies on pre-commit and CI for Bash writes.
- The agent-instructions block is prose, and stays advisory; only the engine verdicts bind.
- Milestones `MILE-198` to `MILE-202` and `MILE-204` build this and are not blocked on each other beyond their
  `Depends-on`.

## Open questions

1. **Which moves are gated?** The example gates `Accepted` and `Rejected`. Should the gate also cover
   `Accepted` to `Superseded` or `Retired`, and an in-place edit of an Accepted ADR's Decision text (the
   `ADR-85` incident, which an amendment now handles by convention only)? Both are expressible as more
   `gates` entries; whether this repository ships them on is for the maintainer when accepting.
2. **Is a `signed-commit` identity source worth adding** to section 5, as `BUG-16` suggests, once agents have
   a distinct identity? Not needed for the first build.
3. **A distinct agent identity and `CODEOWNERS`.** Whether and when to adopt them is the future option of
   section 4, not decided.
4. **Embedded bundle (B) or published bundle (C)?** B is chosen for Phase 0 and the directory layout keeps C
   open; a decision to publish separately would be its own ADR.

Resolved on 2026-11-05 and recorded above: `/done` is covered by an amendment to `ADR-70` (section 1);
the CI evidence default is `pr-author`, with the single-maintainer residual risk accepted by the maintainer
(section 4); the attestation default stays `gh-verified` with the actor kind added to the row (section 5).

## Amendment (2026-11-06): `require_evidence` is a repository option, and the gated statuses

Accepted by the maintainer on 2026-11-06.

Section 4 says the gate action turns `require_evidence` on. That cannot be: `require_evidence` is an option
of the rule `diff.decider-required`, set in the repository's `.urzua/config.yaml`, and an action cannot
set a rule option. The corrected statement is: `require_evidence` is a config option the repository sets;
the gate action always passes `--actors` (input `actors: "true"`) so the evidence file exists in CI; this
repository leaves `require_evidence` off. CI therefore records who authored and approved a gated change
and does not fail on it, which is the accepted risk of section 4 (with a single maintainer the evidence is
authorship, always the maintainer's). A repository with a second human or a distinct agent identity can
turn it on.

Open question 1 is answered for this repository: its `authority` table gates `Accepted`, `Rejected`,
`Superseded` and `Retired` for ADRs and `Accepted`, `Rejected` and `Superseded` for RFCs (an RFC has no
`Retired` status). An in-place edit of an Accepted ADR's Decision text stays handled by the amendment
convention and is not gated. The Status of this ADR stays Accepted.

## Amendment (2026-11-07): a guardrail name the build does not ship is a load error

Accepted by the maintainer on 2026-11-07.

Section 3 keeps an unknown key or guardrail name as a problem that `harness install`, `doctor` and the
session-start hook report, and lets the configuration load. For a guardrail name that is wrong: the gate
guardrails (`acceptance`, `protected-paths`) are the security-relevant ones, and a misspelled name
(`acceptence`) leaves the one meant switched off while `urzua check`, the CI gate, runs as if it were
configured, and `NEEDS_AUTHORITY` validation is skipped because the real guardrail is not enabled. The
decision is now: **an unknown name under `harness.guardrails` is a configuration load error for every
command**, so `check` exits 2. The message names the unknown name and, when one is close, the nearest valid
name (`acceptence`: did you mean `acceptance`?), and lists the names the build ships. An unknown key
elsewhere in `harness`, or an unknown key inside a known guardrail, is still kept and reported, not a load
error. `urzua doctor` reports the bad name as a failed `config-parses`, and its `config-parses` line no longer
says "no unrecognized keys" while the harness table holds unrecognized entries; it counts them and points to
`harness-config`. A broken file still fails the way section 2 says for a configuration that does not load
(the editor hook reads `harness.guardrails.editor-hook` from the raw file). The Status of this ADR stays
Accepted.

## Amendment (2026-11-07): the settings command, the repository an edit is judged in, and what the harness keeps

Proposed in the third review of the harness (2026-11-07) and accepted by the maintainer on 2026-11-07. The maintainer's review of a pull request that carries an override-log row is the approval of that row, until a distinct agent identity exists.
The Decision is unchanged; this corrects statements above and records what the review found.

- **The settings command depends on the project directory, with a fallback.** The Context says the adapter's
  command does not depend on `CLAUDE_PROJECT_DIR`, and the first implementation found the translator with
  `git rev-parse --show-toplevel`. Both are replaced: the command resolves the project root from
  `CLAUDE_PROJECT_DIR` and, when it is unset, from the outermost directory above the current one that holds
  `.urzua/config.yaml`. The current directory follows a Bash `cd`, so a git repository created below the
  project (`git init docs/nest && cd docs/nest`) made the old command look for the translator in the wrong
  place, find none and let the edit through, ignoring `on_unjudged: deny`. When the translator or `python3`
  cannot run, the editor hook's command now reads `.urzua/config.yaml` as text and refuses the edit where
  `acceptance` or `protected-paths` is enabled and not set to `on_unjudged: allow` (accepted risk 7). The
  unset variable stays a session-start problem.
- **An edit is judged by the outermost governed repository, with symlinks followed to the end.** The engine
  enters the outermost git working tree with `.urzua/config.yaml` that holds the file the edit writes (after
  following every link), else the one that holds the session's directory. A nested repository with no
  configuration of its own therefore no longer makes its files unjudged, and a link inside it to a protected
  file is the protected file. A link whose target does not exist yet is the target it would create, so
  writing through `docs/foo -> ../.github/workflows/evil.yml` is a write to a workflow.
- **The harness keeps some files for itself.** While `protected-paths` is on, the edit tools are refused
  for `.urzua/cache`, `.urzua/harness.lock`, the file `override_log` names, `.git/hooks` and `.git/config`.
  They are not in the `authority` table, so CI's `diff.decider-required` does not ask a decider for an
  override row that the override path appends. The override path writes the log from the engine, not through
  an edit tool, so it still works.
- **The session-start check compares the lock.** It reports an installed file whose hash differs from the
  lock or that is gone (a neutered or deleted translator), a hook entry that is present by name but is not the
  one the install wrote, and a settings key the adapter names as switching every hook off
  (`disableAllHooks`).
- **The MCP path reading.** An MCP tool call is read for every string that can name a file, at any depth and
  in any list, `file://` URIs included. A tool named for reading is left alone.

## Amendment (2026-11-08): the repository an edit is judged in, and the translator's hash

Proposed in the fourth review of the harness (2026-11-08) and accepted by the maintainer on 2026-11-08.
The Decision is unchanged. This amends the "outermost" rule of the amendment of 2026-11-07, which let an
agent replace the session's gates with two writes outside the repository.

- **The repository an edit is judged in.** The rule "an edit is judged by the outermost governed repository"
  is withdrawn. The outermost repository was attacker-controllable: an empty `P/.git` and a
  `P/.urzua/config.yaml` written above the project (both are "outside the repository", so both were
  allowed unjudged) became the governing repository, and the project's `.claude`, workflows, hook files and
  ADR statuses were then judged by a configuration with no gates. The decision is now: **an edit is judged
  against the session's own repository** (the one that holds the directory the session started in, which the
  translator names in the request as `project_dir`, else the one that holds the current directory) **and
  against every governed repository that holds the resolved file, inner or outer, and a deny from any of
  them is the verdict.** A repository the session did not start in can add restrictions and never remove
  them. A `.git` counts only when it is a real git working tree (`git rev-parse --show-toplevel` succeeds
  there and names that directory), so an empty file or directory called `.git` makes nothing. Session-start
  and the post-command check enter the session's own repository the same way, not the outermost one. The
  settings command is unchanged: it still finds the translator from `CLAUDE_PROJECT_DIR`, else from the
  outermost configuration above the current directory, and the translator's hash (below) is what makes a
  translator found in a minted directory useless.
- **The translator is verified before it runs.** Every hook command of the Claude Code adapter embeds the
  sha256 of `.claude/hooks/urzua-claude-code.py`, written by `harness install` and `harness update` from the
  bundle, and reads the file once, compares its hash and runs those bytes (nothing can swap the file between
  the check and the run). On a mismatch the command does not run the file: it takes the same path as a
  missing translator. The `edit` command refuses an agent's edit where `acceptance` or `protected-paths` is
  enabled and not set to `on_unjudged: allow` (it reads `.urzua/config.yaml` as text), and otherwise states
  that the hook did not run; the `post-command` and `session-start` commands state it. The message names the
  mismatch. A Bash write to the translator therefore no longer silences every later report (accepted risk 1
  said it was reported; it was not). `harness status` and the session-start check keep comparing the file
  and the hook entry with the lock; `update` rewrites both, and `uninstall` removes them.
- **MCP tools.** A tool named for reading is still judged when the call carries a key naming an output
  (`output`, `out`, `dest`, `target`, `save` and the like) or a content key. A path string is trimmed,
  cut at a NUL and normalised lexically (`.` and `..`) before its length is tested; a string that is too
  long as written is a path only if it collapses to a shorter one, and is otherwise a candidate looked at
  only for a refusal. An MCP tool that runs a shell command or applies a patch stays an unparsed route
  (accepted risks 1 and 5): it is not judged and not reported.
- **Timeout.** The release build's `harness run edit` of an ADR in this repository's corpus took 5 to 7 seconds
  on the review machine, against the host's default of 60 seconds for a command hook, so the settings do not
  set a `timeout`. A host that kills the hook at its limit fails open; that is the host's behaviour, not
  the guardrail's.
- **The accepted-risk list** is made exact: a commit is seen only on the checked-out branch (route 1); a
  Bash write to the translator is caught at the next hook run; an MCP shell or patch tool is unparsed (routes 1
  and 5); the invalid-YAML case is named (route 7); route 8 is rewritten for the rule above; the
  instructions block is not a protected path (route 9) and a waiver record is not gated (route 10).
The Status of this ADR stays Accepted.

## References

- `ADR-69`, `ADR-70`, `ADR-71`: the hook, drafting commands and layered gate this extends; `ADR-70` gains an
  amendment for lifecycle commands (`/done`); `ADR-78`: the override log and `block_on`, which gains an
  amendment for the post-command check.
- `RFC-2`, `BUG-16`, `MILE-18`, `MILE-66`: identity resolution, why a verified login is not an attestation of
  who acted, and the longer-term identity binding.
- `ADR-64`: structured in, structured out. `ADR-80`: declared boundaries. `ADR-84`: status follows evidence.
- `ADR-87`: the `info` level used for an unverified local run.
- `SPEC-2`, `SPEC-3`, `SPEC-8`, `SPEC-19`: the specs this revises.
- `MILE-19` (human attestation with expiry) and `MILE-79` (a guarded edit command) are related and not
  replaced; `MILE-179` (git hooks and `doctor`) is the readiness check this extends.
- `MILE-198`, `MILE-199`, `MILE-200`, `MILE-201`, `MILE-202`, `MILE-204`: the build.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** Maintainer decisions, 2026-11-05 (ship an installable harness bundle in Phase 0; acceptance enforced by the harness and CI; attestation identity source a configuration option); the maintainer accepts it in a later change. | **substantive** |
> | 2026-11-05 | Amended after independent review of the Proposed text. The acceptance guardrail now claims only what holds (edit tools refused locally, Bash reported, CI evidence is authorship) and gains an "Accepted risk" section (bypasses accepted by the maintainer); the default `evidence` is `pr-author`; the `ADR-70` question is resolved by an amendment there; actor kind joins the attestation row and `RFC-2`, `BUG-16`, `MILE-18` and `MILE-66` are cited; `harness uninstall`, settings-merge rules, the `on_unjudged: deny` escape hatch and an Open questions section are added; citation and vendor-neutrality corrections. **Why:** Maintainer decisions of 2026-11-05 on the review at pull request 355; `Status` stays Proposed. | **substantive** |
> | 2026-11-05 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-05. | **substantive** |
> | 2026-11-06 | Added an `## Amendment (2026-11-06)`, accepted by the maintainer on 2026-11-06: `require_evidence` is a config option the repository sets, the gate action always passes `--actors`, and this repository leaves it off (an action cannot set a rule option); open question 1 answered (this repository gates `Accepted`, `Rejected`, `Superseded` and `Retired` for ADRs and `Accepted`, `Rejected` and `Superseded` for RFCs). `Status` stays Accepted. **Why:** Maintainer decisions, 2026-11-06, on the review of pull request 374. | **substantive** |
> | 2026-11-06 | Accepted-risk list brought in line with the harness after a second-pass review; the decision is unchanged. A Bash write to a protected path, and any change the same command commits, is now reported by the post-command check (route 2 shrinks from silent to reported; routes 1 and 2 are still not prevented); the settings file outside the install stays unreported. An adapter failure no longer defeats `on_unjudged: deny` for `acceptance` and `protected-paths`. **Why:** the review found the report claimed more than the check did, and the adapter failing open where the configuration said deny. | **structural** |
> | 2026-11-06 | Accepted-risk list revised after the maintainer decided on 2026-11-06 to close the remaining guardrail gaps in Phase 0; the decision is unchanged. NotebookEdit and file-writing MCP tools now reach the editor hook, a Bash write to a gitignored protected file is reported by content hash, gate matching uses Unicode case folding, and a hard link is judged as the file it shares an inode with (Unix). Route 2 (the shell edit of a guardrail) joins route 1 and now includes the agent's local settings file under `.claude`; `--no-verify`, a pull request that changes its own workflow and an agent acting under the maintainer's token stay; a fifth item states what remains of the hook's reading (hard links off Unix, no Unicode normalisation, an MCP call with no path, no baseline before session start, user-level settings). A repeat of an already-reported change is silent and is documented, not changed. **Why:** the second-pass review left these gaps; the list now says exactly what remains. | **structural** |
> | 2026-11-07 | Added an `## Amendment (2026-11-07)`, accepted by the maintainer on 2026-11-07: a `harness.guardrails` name the build does not ship is a config load error for every command (`check` exits 2) with the nearest valid name suggested, and `doctor`'s `config-parses` line no longer says "no unrecognized keys" beside unrecognized harness entries. `Status` stays Accepted. **Why:** Maintainer decision, 2026-11-07, on the core-model review. | **substantive** |
> | 2026-11-07 | Added a proposed `## Amendment (2026-11-07)`, pending maintainer acceptance, after the third review of the harness: the settings command resolves the project root from `CLAUDE_PROJECT_DIR` or the outermost configuration above the current directory and refuses an edit under `on_unjudged: deny` when it cannot run the translator; an edit is judged by the outermost governed repository with symlinks followed, a dangling link included; the harness protects its own cache, lock, override log and git hooks from the edit tools; session-start compares the installed files, hook entries and `disableAllHooks` with the lock; MCP calls are read for every path-like string. The Accepted-risk list gains items 6 to 8 (a Bash write to the harness's own files, an unreadable configuration in the shell fallback, a nested repository with its own configuration), and items 1 and 5 are widened (`--assume-unchanged` is now reported; local settings and a swap between check and write are named). `Status` stays Accepted. **Why:** the third review found the wrapper failing open outside the project root, a dangling link and a nested repository escaping the gates, and several silent routes. | **substantive** |
> | 2026-11-07 | Amendment (settings command, judged repository, harness-owned paths, override-log approval by PR review) accepted. **Why:** Maintainer decision, 2026-11-07. | **substantive** |
> | 2026-11-08 | Added an `## Amendment (2026-11-08)`, accepted by the maintainer on 2026-11-08, after the fourth review of the harness: an edit is judged against the session's own repository and every governed repository that holds the file, and a deny from any wins (the "outermost" rule of the 2026-11-07 amendment is withdrawn, because two unjudged writes above the project minted a governing repository with no gates); a `.git` counts only when git accepts it; every hook command embeds the translator's sha256 and refuses to run a translator that differs; a read-named MCP tool is judged when the call names an output or carries content; MCP path strings are trimmed, NUL-cut and normalised before the length test. The Accepted-risk list is made exact (items 1, 5, 7 and 8 revised, items 9 and 10 added). `Status` stays Accepted. **Why:** Maintainer decisions, 2026-11-08, on the fourth review of the harness. | **substantive** |
