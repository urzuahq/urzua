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
   again is silent, by design. A later edit to a path that a reported weakening
   un-protected is not reported (amendment of 2026-11-09). A commit is seen only on the checked-out branch: a commit made on another
   branch or in a sibling worktree is silent until it lands on the session's `HEAD`, and a
   `git push origin side:main` of another ref is never seen. An MCP tool that runs a shell command is not a
   Bash tool call (the post-command matcher is `Bash` only): what it writes is neither judged nor reported.
   A Bash write to the translator is caught at the next hook run, because every hook command checks the
   translator's sha256 before running it (amendment of 2026-11-08), but the programs the command finds on
   `PATH` (`python3`, `urzua`) are not verified.
2. **`git commit --no-verify`**, which skips the commit hook.
3. **A pull request that changes its own workflow.** The gate runs from the head commit's workflow, so a pull
   request can drop or weaken the gate step it is judged by. Only `CODEOWNERS` and branch protection, which
   are host settings, can stop that, and neither is configured by this decision. The same holds for a
   self-hosted gate whose binary is built from the pull request: a pull request that changes the code of the
   binary the gate runs (this repository's `checks.yml` runs `rust/target/release/urzua` built from the pull
   request, and `rust/` is under no path gate) can change the rule it is judged by.
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
   these files. Deleting or forging the baseline (`.urzua/cache/protected-hashes.json` or the cache
   directory) in the same command as a gate removal hides the removal (amendment of 2026-11-09).
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
   deny edits inside it, which costs the agent, not the guardrail. A linked worktree of the session's
   repository is judged against the session's gates too when `harness.outside_writes` is `shared` (amendment
   of 2026-11-09, outside writes); with it undeclared or `allow`, a worktree at a commit without gates is
   an unjudged place to write.
9. **The agent-instructions block is not a protected path.** The block in `AGENTS.md` (or the file
   `harness.instructions_file` names) is a control this decision relies on, but an edit tool can remove it
   with no refusal and no session-start note; only `urzua harness status` reports it `modified`.
10. **A waiver record is not gated.** A waiver is an ordinary record, not a gated status, and
    `override.unreviewed` is waivable, so an edit-tool waiver can silence that rule. Where
    `authority.waivable` is undeclared or `any`, `diff.decider-required` is waivable too, and a non-decider can
    accept a record by adding a waiver of it in the same pull request: that route needs no Bash write and no
    token, and so adds power beyond routes 1 and 4. `authority.waivable: never` (or `base`) closes it for that
    rule (amendment of 2026-11-09, gate waivers); `override.unreviewed` stays waivable.
11. **Deleting the `harness:` section.** With no `harness` key no guardrail is enabled; `post-command` and
    `session-start` then report only "guardrail not enabled", and the CI diff gate at the pull request is the
    place the change is seen (amendment of 2026-11-09).
12. **A separate clone.** A `git clone` of the repository shares no `git-common-dir` with the session's, so
    `harness.outside_writes: shared` does not judge a write there, and a commit made in it, or a push of a
    side branch from a linked worktree, is not seen by the post-command check (amendment of 2026-11-09,
    outside writes). `outside_writes: deny` refuses the write; CI sees the pull request.
13. **A stale base.** when the base revision has no `authority` (for example a branch made before the gates were introduced), the head table is judged; a pull request whose head `authority` names its own author as a decider then passes, because that is indistinguishable from the first introduction of `authority`. Branch protection that requires an up-to-date branch, or a merge queue, removes this route by making the base current. (amendment of 2026-11-09, decider table; accepted by the maintainer on 2026-11-09).

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
  settings command follows the same rule: it finds the translator from `CLAUDE_PROJECT_DIR` and, when that
  is unset, from the nearest real git working tree at or above the current directory that holds both
  `.urzua/config.yaml` and the translator (found with `git rev-parse --show-toplevel`, stepping outward past
  a nested repository that holds neither), never from the outermost configuration. This replaces the
  fallback of the amendment of 2026-11-07; a configuration written above the project, with or without a
  `.git`, is never chosen over the project, and when the engine judges it can only add denies.
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
  (`output`, `out`, `dest`, `target`, `save` and the like) or a content key (`content`, `text`, `body`, `data`, `payload`, `value` and the like). A path string is trimmed,
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

## Amendment (2026-11-09): the accepted-risk list, made exact for what the review of the baseline found

Status of this amendment: accepted by the maintainer on 2026-11-09. It documents existing behaviour and was first decided by the orchestrator under the
maintainer's 2026-11-07 decision that the accepted-risk list be exact. It
changes no code and no decision; it states what the harness already does, as the independent review of
the post-command baseline found it.

- **Route 1, a weakening is reported once.** The post-command check reports a removed gate (a path taken out
  of the `authority` path gates, or the configuration itself) in the command that made it. A later edit to a
  path that was protected before the removal is not reported, because the next baseline is taken from the
  weakened configuration. The report is the only signal; it is not repeated.
- **Route 6, the harness baseline can be deleted or forged in the same command.** Deleting or rewriting
  `.urzua/cache/protected-hashes.json`, or the whole cache directory, in the same Bash command that removes a
  gate hides the removal: with no baseline the check records one from the weakened configuration and has
  nothing to compare, so it reports no shrunk set. The edit tools cannot write these files; a Bash command
  can.
- **Route 11 (new), deleting the whole `harness:` section turns the guardrails off.** With no `harness`
  key no guardrail is enabled, so `post-command` and `session-start` report only that the guardrail is not
  enabled, and the editor hook has no gate to apply. The change is a change to `.urzua/config.yaml`, which is
  visible to the CI diff gate at the pull request, as any configuration change is; locally it is not
  prevented and not reported beyond that note.

The Decision is unchanged. The Status of this ADR stays Accepted.

## Amendment (2026-11-09, outside writes)

Status of this amendment: accepted by the maintainer on 2026-11-09. It follows the review that found a linked
worktree of the repository, at a commit that holds no gates, an unjudged place to write and to commit from.

- **The problem.** An agent runs `git worktree add --detach ../wt <old commit>` (or clones the repository at an
  old commit). The commit has no `harness` and no `authority`, so an edit-tool write to `../wt/.github/workflows/x.yml`
  is judged by the worktree's own configuration, which enables nothing, and is allowed unjudged. The session's
  repository does not hold the path, so its gates are not applied, and the post-command check reads only the
  session's repository, so a commit made in the worktree and a push of a side branch are never seen. Route 8
  said a repository the session did not start in only adds restrictions; here it removed them for the
  repository's own files.
- **The decision: `harness.outside_writes`.** A new key under `harness`, with one of three values, and an
  optional list `harness.allow_paths` of absolute or `~`-relative directories read only with `deny`:
  - `allow`: an edit is judged as before (the repository that holds the file judges it, and a place with no
    gates judges nothing). This is also the meaning of an undeclared key: no behaviour change.
  - `shared`: when the git toplevel of the file's working tree is not the session repository's own and shares
    the session repository's `git rev-parse --git-common-dir` (a linked worktree of the same repository,
    including one nested inside the session repository's directory), the file's path relative to that
    toplevel is also judged against the session's configuration: its acceptance gates and its protected paths.
    A file whose toplevel is the session repository's own is the session's and is judged as before. A request
    that carries no content is still judged by its path (the protected paths).
    A deny wins, as in the amendment of 2026-11-08. The record rules of the editor hook are not re-run on the
    session's copy of the file; only the gates are. A plain git repository or a submodule nested inside the
    session repository has its own object database (a submodule has its own git directory), so it shares no
    common directory with the session's and stays unjudged like a separate clone (route 12).
  - `deny`: an agent's edit-tool write to a file outside the session's repository (links resolved) is refused
    while any guardrail is enabled, unless the file is under a directory in `allow_paths`. A plain git
    repository or a submodule nested inside the session repository counts as outside, because its git
    toplevel is not the session's, so a write there is refused unless the file is under `allow_paths`. The override of
    `ADR-78` is not accepted for this verdict.
  Any other value, or an `allow_paths` entry that is not an absolute or `~`-relative directory, is a
  configuration load error; `allow_paths` beside a mode other than `deny` is reported by `doctor` as read by
  nothing. A person's edit (actor `human`) is not affected.
- **Where it is declared.** `urzua init` proposes `harness: {outside_writes: shared}` (it enables no
  guardrail by itself). `urzua doctor` warns when a guardrail is enabled and `outside_writes` is undeclared,
  and recommends `shared`. The documentation recommends `shared`. This repository declares `shared`.
- **Accepted risk, new route 12.** A fresh `git clone` (or any repository with its own object database)
  shares no common directory with the session's, so `shared` does not reach it: a write there is judged by
  that clone's own configuration or not at all, and its commits and pushes are never seen. `deny` closes the
  write; nothing local closes the push, which CI sees at the pull request against the head's table.
  Route 1 now also names a commit made in a linked worktree and a push of a side branch: neither is seen by
  the post-command check, which reads the session's repository.

## Amendment (2026-11-09, fifth review): the status gates and the MCP call bound

Status of this amendment: accepted by the maintainer on 2026-11-09. It fixes behaviour against what this ADR promises and
changes no decision.

- **Route 1, a weakening of a status gate is reported like a path gate.** The post-command baseline also holds
  the status gates and the `acceptance` flag. A gate removed or narrowed, or `acceptance` switched off, in the
  same command as a move to the gated status still reports the move, and the loss is reported once ("the status
  gates shrank"), at `session-start` as well. A later move is not reported, as for a path gate. A new session
  clears the post-command memory, so a change reported in an earlier session and made again is reported again.
- **A change of a status gate's decider group (`by`) is not shrinkage.** The baseline compares which
  `(type, status)` pairs are gated, not who may decide them. A gate whose `by` is changed to a wider group (or any
  other group) still gates the status, so the loss line ("the status gates shrank") is not raised for it. What is
  reported instead: a move to the status in the same command is reported in `gated_moves`, naming the gate as the
  configuration now has it, so the widened group is named; and `.urzua/config.yaml` is itself a protected path, so
  its change is reported in `protected_changes`. Who may decide is checked by CI's `diff.decider-required` against
  the head's table, which this amendment does not change. A report that names the new group is what the post-command
  check can offer after the fact; it never prevents the change.
- **The Timeout bullet above is replaced:** the `edit` hook entry sets `timeout` to 120 seconds. The Claude Code
  translator judges an MCP call's files against the gates first, in one engine process for all of them (each file
  with its own proposed content), however many files the call names, and stops at the first block. It then judges
  at most 8 files that are checked as records (8.8 to 12.0 seconds each in the release build on a loaded 4-CPU
  machine, the measurement is in `SPEC-24`). A call with more is an edit that was not judged. So is a call that
  names more than 512 files (the first 512 are judged against the gates, the rest are not) and a call with more than
  512 distinct short strings, rather than silently cut short. Not judged means what it means elsewhere: refused
  where `acceptance` or `protected-paths` is `on_unjudged: deny` and covers a file the call names, allowed with a
  notice otherwise. Before this the gates ran one engine process per file, 128 seconds for 2200 files with the
  protected one last, past the timeout, where a host kill fails open.

The maintainer is asked to accept exactly these:

1. The post-command baseline holds the status gates and the `acceptance` flag, and a removed or narrowed gate, or
   `acceptance` switched off, is reported like a path gate (route 1), at `session-start` as well; a new session
   clears the post-command memory.
2. A change of a status gate's `by` is not reported as shrinkage; the move and the configuration change are
   reported as stated above.
3. The `edit` hook entry's `timeout` of 120 seconds.
4. The MCP bounds: one gates process for up to 512 files, each judged with its own content; more than 512 files,
   more than 8 record-checking files or more than 512 short strings is an edit that was not judged.
5. The engine request gains `contents` and the gates-only verdict gains `records` (`SPEC-24`); both are read only
   by the translator of this bundle, and a translator that gets no `records` asks one file at a time.

The Decision is unchanged. The Status of this ADR stays Accepted.

## Amendment (2026-11-09, decider table)

Status of this amendment: accepted by the maintainer on 2026-11-09. It changes what `diff.decider-required`
reads; the rest of section 4 is unchanged.

**The problem.** The rule read `authority` from the pull request's own checkout. A pull request that adds its
author to a decider group, deletes a gate, and moves a record to a gated status in the
same change was judged against the table it had just written, and reported nothing. The sentence above that the
rule is "a check that means something" with a second human held only for a pull request that did not edit the
table.

**The decision.** `authority` takes a key `decider_table`: `base`, `head` or `both`.

- `head` reads the table (groups, logins, gates, evidence) from the checkout, as before. Undeclared reads as
  `head`, so the rule is opt-in and no existing configuration changes behaviour.
- `base` reads who may decide from the base revision of the `--base` diff: its groups, its logins, its gates and
  its `evidence`. The head's table is not consulted for the verdict.
- `both` judges every gated change under both tables, so a gated change needs a decider under each, and a gate
  that is in either table applies (a gate only the head declares is judged by the head's groups, a gate only the
  base declares by the base's).
- The mode in force is the stricter of what the head and the base revision declare (`head` < `base` < `both`),
  so a pull request cannot loosen the mode by editing the key; a loosening takes effect once it is merged.
- The base table is read from the configuration file as it was at the merge base, by the same mechanism that
  reads a record's committed copy (`git show` through `urzua_io`). Only the `authority` section is read, so a
  base config written by an older build need not load whole. A base with no config file, or with no `authority`
  section (a branch cut before the table existed), has no table to judge against, and reading it as "no deciders
  and no gates" would let a pull request that declares its own table pass unjudged. Under `base` and `both` such a
  base is therefore not trusted: the head table is judged instead (a gate the head declares applies, and a decider
  must be in the head's group), the finding names "the head's table", and an `info` notice says the base had no
  authority. A base whose config cannot be read, or whose `authority` section does not load, is said in a warning
  notice and the head table is used for that run.
- The base's rule setting is read from the same configuration text. When the mode in force is `base` or `both`,
  the level and options of `diff.decider-required` and `fail_on` are the stricter of the base's and the head's
  (`off` < `info` < `warn` < `error`; `error` < `warning`), and an `authority` section the head dropped is
  replaced by an empty one so the base table is still judged. Deleting the rule's entry, setting it to `off`, or
  lowering it to `warn` with `fail_on: error`, in a pull request, is therefore judged under the base's stricter
  setting; an `info` notice names each setting taken from the base. The stricter `fail_on` applies to the whole
  run, and an explicit `--fail-on` still wins.
- With no `--base` there is no base revision. The rule reports `not-applicable` as before, the editor and
  post-command guardrails keep reading the checkout's table, and when `decider_table` is `base` or `both` the
  report carries an `info` notice saying the head table is used.
- Findings read as before under `head`. Under `base` and `both` the gate in the message names the table it
  comes from.
- `urzua init` writes `decider_table: base` in the configuration it proposes. `urzua doctor` warns, as a
  structured check named `authority-decider-table`, when gates are declared and `decider_table` is not, and
  recommends `base`. The gate action's documentation recommends `base`. This repository declares `base`.

**What it does not change.** The accepted-risk list stands. A pull request that drops `--actors` from the
workflow command, or edits the workflow that runs the gate, still removes the check without touching the
configuration: that is the remaining known limit and is accepted risk 3. Turning the rule off or downgrading it
in the configuration is no longer part of that limit under `base` and `both`: it is judged under the base's
stricter setting. The configuration is protected only if a gate lists `.urzua/config.yaml` (as this repository's does); `authority` does not make it a protected path by itself, so a pull request can edit `exclude_files`, `dir` or a status field name and accept a record without a decider unless a path gate covers the file. `urzua doctor` warns (`authority-config-gated`) when gates are declared and none lists it. On a stale base that has no `authority` the route in item 13 applies.

Stale base: when the base revision has no `authority` (for example a branch made before the gates were introduced), the head table is judged; a pull request whose head `authority` names its own author as a decider then passes, because that is indistinguishable from the first introduction of `authority`. Branch protection that requires an up-to-date branch, or a merge queue, removes this route by making the base current. This sentence and accepted risk 13 were accepted by the maintainer on 2026-11-09, as was the rest of this amendment.
With a single maintainer the evidence is still authorship, and the rule still cannot tell an agent acting under
the maintainer's token from the maintainer (item 4).

The Decision is otherwise unchanged. The Status of this ADR stays Accepted.

## Amendment (2026-11-09, gate waivers)

Status of this amendment: accepted by the maintainer on 2026-11-09. It changes whether a waiver can silence
`diff.decider-required`; the rest of section 4 is unchanged.

**The problem.** With `decider_table: base` the table is the base's, but waivers are read from the head. A
non-decider who moves a record to a gated status and adds a waiver record (`Rule: diff.decider-required`, the
record as `Scope`) in the same pull request had the finding listed as waived, and the gate passed. Risk 10
called this no power beyond routes 1 and 4; it needs neither a Bash write nor the maintainer's token.

**The decision.** `authority` takes a key `waivable`: `never`, `base` or `any`.

- `any` is today's behaviour and what undeclared means: any active, effective waiver of the rule counts.
- `base`: a waiver of `diff.decider-required` counts only if the same waiver record exists in the base revision
  unchanged (the diff neither adds nor edits it). A waiver the pull request adds or edits is ignored for this
  rule, and an `info` notice names it.
- `never`: no waiver of either variant (`gated-move-without-decider`, `gated-path-without-decider`) counts; a
  gated change is cleared by a decider, not by a record, as `status.done-with-unresolved-items` is.
- The setting in force is the stricter of the head's and the base revision's (`never` > `base` > `any`), the
  same rule as `decider_table`, so a pull request cannot loosen it by editing the key.
- Waivers of other rules, including `override.unreviewed`, are unaffected.
- `urzua init` proposes `waivable: never`. `urzua doctor` warns (`authority-waivable`) when gates are declared
  and the key is not. This repository declares `never`.

**Also decided in the same review.** Under `base` and `both` the base rule's options are merged as the stricter
too: `require_evidence` is on when either side sets it, so a pull request that keeps the level and drops the
option no longer turns a missing actors file into an `info`.

Risk 10 is reworded above. The Decision is otherwise unchanged. The Status of this ADR stays Accepted.

## Amendment (2026-11-09, rename pairing)

Status of this amendment: accepted by the maintainer on 2026-11-10 (first decided by the orchestrator as a fix to
the gate's guarantee). It narrows how `diff.decider-required` and `status.transition-allowed`
find a changed record's committed status; the rest of section 4 is unchanged.

**The problem.** A record the base lacks at its path was paired with any record the diff deleted that carried
the same `Stable-Id` (`BUG-191`, meant for renames) and judged against the deleted record's status. The head
chooses its own `Stable-Id`, so a non-decider could delete an Accepted ADR and add a new `Status: Accepted`
record copying that `Stable-Id`: the gate saw no move and passed, with no Bash write, token or waiver.

**The decision.** The pairing needs the same record id too (type and number, as the engine derives it from
the file name or the type's declared identity) as well as the `Stable-Id`.

- A slug rename (same number, new slug, same `Stable-Id`) still pairs: it is judged against the status it had,
  so an unchanged status is not a move.
- A renumber or a retype that keeps the `Stable-Id` no longer pairs: the added record is new and is judged from
  no prior status, so an Accepted one is a gated move that needs a decider, like any new record.
- A record with no readable record id never pairs. Several deleted records with the same id and `Stable-Id`
  still count the record unreadable.
- `status.transition-allowed` uses the same pairing, so a renumbered record is judged as new there too.
- The deleted base copy is parsed with the type's declared document model, so a type that derives its record id
  from a declared `identity` pattern gets the same id on both sides; parsed by file name convention alone, a
  deleted copy of such a type would have no id and never pair.

**Accepted consequence.** A slug rename that keeps the record id pairs even when its body changes, the same as an
in-place edit of an Accepted record, which this ADR already leaves to the amendment convention and does not gate.
A case-only file name rename (`adr-1-case.md` for `ADR-1-case.md`) reads as a new record and is gated.

**Merge queue (nit).** On `merge_group` runs `checks.yml` passes no pull request number, so the gate action
writes no actors file and a gated change in the queue run is reported `info` (or an error under
`require_evidence`). The pull request run judged the same change with the author's identity; the queue run
adds no check of this rule. This is stated here, not changed: it is not a bypass, and it is a limit of what the
queue run can know. A repository that wants the queue run to fail closed sets `require_evidence: true`.

**Actors outage (nit).** With `actors: "true"` on the gate action and `require_evidence` off, a GitHub API
failure while writing the actors file passes the gate with a warning (amendment of 2026-11-06). The gate action
documents this on its `actors` input; set `require_evidence: true` to fail closed.

The Decision is otherwise unchanged. The Status of this ADR stays Accepted.

## Amendment (2026-11-10, deletions)

Status of this amendment: accepted by the maintainer on 2026-11-10. It extends what `diff.decider-required`
counts as a gated move; the rest of section 4 is unchanged.

**The problem.** The gate looked only at records present in the head. A non-decider could `git rm` an Accepted
ADR, regenerate the index, and pass: the record's status had effectively moved from Accepted to gone, a stronger
move than Superseded or Retired (both gated), with no finding and no post-command report.

**The decision.** A record the diff deletes, whose `Status` at the merge base is one a gate names (a `to` status
of a gate of its type), is a gated move ("the record was deleted while Status was 'Accepted'"). It is judged like
any other gated move: the same decider group, `decider_table`, `waivable` and `require_evidence` apply, and it is
reported as the variant `gated-deletion-without-decider`. Gates name only `to` statuses, so there is no `from`
reading.

- A record that pairs as a rename (the same `Stable-Id` and the same record id still exists on a head record of
  its type) is exempt. A record with no `Stable-Id`, or a renumbered one, does not pair and is a deletion.
- A deleted record at an ungated status (a Draft) is not judged.
- Under `decider_table: base` or `both` which deleted files are records of a gated type is decided by the base
  revision's declaration of the type (`dir` and its further directories, `exclude_files`) as well as the head's: a file either revision
  governs counts, so a pull request cannot take an Accepted record out of reach by adding it to `exclude_files` or
  moving the type's `dir` in the change that deletes it. Under `head` or undeclared only the head's declaration is
  read. A base config that does not load is said in a warning notice and the head's declaration alone is used.
- The post-command check reports a gated record the command deleted (Bash `rm`, `git rm`) in `gated_moves`, as it
  reports a Bash move, and never blocks.

**Case of the gated status.** Status strings are compared for the gates after trimming and folding case, and the
status field is read for the gate with its name's case folded, so `accepted` or `ACCEPTED` is the gated status
`Accepted`. Before this, a respelled status passed the gate and, with `status.undeclared-value` waived or off, the
rest of the checks. A case-only difference between the base's and the head's status is not a move.

The Decision is otherwise unchanged. The Status of this ADR stays Accepted.

## Amendment (2026-11-14, symlinked records)

Status of this amendment: decided by the orchestrator on 2026-11-14, pending maintainer review. It closes a
route around the status gate; the rest of section 4 is unchanged.

**The problem.** The gate judges a record by the path it is tracked at. A tracked record file that is a
symlink was read through the link. A non-decider could land an Accepted record in two pull requests: the first
moves a Proposed record out of the governed directory (`git mv docs/adr/ADR-1-a.md notes/x.md`) and leaves a
symlink in its place, which changes no `Status`; the second sets `Status: Accepted` in `notes/x.md`, a path no
record type governs and no gate names, while the link at `docs/adr/ADR-1-a.md` now reads as an Accepted record.
Neither change is a gated move. Separately, a link to a file outside the repository was read and, by
`urzua export`, published.

**The decision.** A tracked record file that is a symlink, or sits behind a symlinked directory, is refused by
every command and never read (`identity.record-symlink-escapes`, `SPEC-2`). A record is read where it is tracked,
never through a link.

- `check`, `check --base`, `audit` and the MCP check tool report it as a blocking finding (an error) that names
  the link and where it points. It is reported on a path-scoped or `--base` run too, and no waiver excuses it.
  Every other command that loads the corpus (`fix`, `reverify`, `waive`, `migrate`, `export`, `graph`, `index`,
  `roadmap`, `explain`) stops with exit 2 naming the file, instead of shrinking the corpus without saying so.
- No link is allowed, an alias to a sibling record included. A first version of this amendment let a link to a
  file in the same type directory through; a review showed that moving a record to `docs/adr/draft.txt`, a name no
  type governs, and leaving `ADR-1-a.md` linked to it, hides the record just as well. A record has no use for an
  alias. A link that does not resolve, or resolves to a directory, keeps the loader's own "could not read" error.
- The rule is on for every configuration, and a configuration sets its level or turns it `off` under `rules:` like
  any rule (maintainer principle, 2026-11-14: everything is a config); an earlier version made `off` a load error.
  **Turning it off accepts the route above:** records are then read through links as before and nothing is reported.
  `urzua init` does not write an entry for it, and this repository's configuration does not declare it. Unless
  declared `off` it runs under every `--rules` selection and so appears in the `rules_executed` of a narrowed run.
- **Why a default-on rule departs from opt-in configuration.** `ADR-53` makes a rule opt-in because a rule is a
  repository's policy about its records, and a repository may decline a policy. This is not a policy: it is a
  refusal that keeps the engine from reading a record where the gates do not look, a security property of the
  tool and not a style rule, so it is on unless a repository says otherwise, and the risk of saying so is written next to the rule.
- The two-pull-request route is closed at its first step: the pull request that leaves the symlink behind carries
  the blocking finding, and the second pull request, had the first merged, carries it too, because the finding
  does not depend on what the second changes. The diff gate does not need to resolve a changed path back to the
  record that links to it.

The Decision is otherwise unchanged. The Status of this ADR stays Accepted.

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
> | 2026-11-08 | Corrected the `## Amendment (2026-11-08)` after review of pull request 438: with `CLAUDE_PROJECT_DIR` unset, the settings command resolves the nearest real git working tree that holds the configuration and the translator, not the outermost configuration (which let a configuration written above the project take the wrapper); `body`, `data`, `payload` and `value` join the content keys. `Status` stays Accepted. **Why:** the independent review found the wrapper still taking an outer configuration. | **substantive** |
> | 2026-11-09 | Added an `## Amendment (2026-11-09)`, documentation of existing behaviour decided by the orchestrator under the maintainer's 2026-11-07 decision that the accepted-risk list be exact, pending maintainer review: a weakening is reported once and a later edit to a formerly protected path is not (route 1); deleting or forging the harness baseline in the same command as a gate removal hides the removal (route 6); deleting the whole `harness:` section turns the guardrails off, visible to the CI diff gate (new route 11). `Status` stays Accepted. **Why:** the review of pull request 459 found these three routes behaving as described and the list naming none of them. | **substantive** |
> | 2026-11-09 | Amendment of 2026-11-09 (the accepted-risk list made exact: routes 1, 6 and 11) accepted. **Why:** Maintainer decision, 2026-11-09. | **substantive** |
> | 2026-11-09 | Added an `## Amendment (2026-11-09, outside writes)`, accepted by the maintainer on 2026-11-09: `harness.outside_writes` (`allow`, `shared` or `deny`) (with `allow_paths` for `deny`) decides how an edit-tool write outside the session's repository is judged; under `shared` a file of a linked worktree of the same repository is also judged against the session's acceptance and protected-path gates, under `deny` any such write is refused except under `allow_paths`, undeclared acts as `allow`; `init` proposes `shared`, `doctor` warns when it is undeclared with a guardrail enabled; a separate clone joins the accepted-risk list as route 12 and routes 1 and 8 name the worktree commit and the side-branch push. `Status` stays Accepted. **Why:** Maintainer decision, 2026-11-09, on the review that found a linked worktree at a commit without gates an unjudged place to write. | **substantive** |
> | 2026-11-09 | Added an `## Amendment (2026-11-09, fifth review)`, pending maintainer review: a removed or narrowed status gate, or `acceptance` switched off, is judged against the baseline in the post-command check (route 1); a change of a gate's `by` is not shrinkage (the move and the configuration are reported); the `edit` hook sets a 120 second `timeout` and the translator bounds an MCP call (all gates in one engine process for at most 512 files, then at most 8 record files). `Status` stays Accepted. **Why:** the fifth review of the harness found these behaving otherwise than the ADR promises. | **substantive** |
> | 2026-11-09 | Amendment of 2026-11-09 (fifth review: the status gates and the MCP call bound) accepted. **Why:** Maintainer decision, 2026-11-09. | **substantive** |
> | 2026-11-09 | Added an `## Amendment (2026-11-09, decider table)`, accepted by the maintainer on 2026-11-09: `authority.decider_table` (`base`, `head`, `both`; undeclared is `head`) says which revision's table `diff.decider-required` reads, closing the route where a pull request adds its author to a decider group, or deletes a gate, in the change it asks to have approved; `urzua init` writes `base`, `urzua doctor` warns when gates are declared and the key is not, and this repository declares `base`. `Status` stays Accepted. **Why:** Maintainer decision, 2026-11-09, on the review that found the rule judging a pull request against its own table. | **substantive** |
> | 2026-11-09 | Corrected the decider-table amendment after review: a base with no config or no `authority` is judged against the head table with an `info` notice (not read as no gates), and under `base` and `both` the rule's level and `fail_on` are the stricter of the base's and the head's; the problem statement no longer names an `enabled` key, and the remaining known limit is dropping `--actors` or editing the workflow (accepted risk 3). **Why:** review found a stale merge base and a rule switched off in the pull request each gave no finding. | **substantive** |
> | 2026-11-09 | Added accepted risk 13 and a sentence to the decider-table amendment, documented by the orchestrator, pending maintainer review: with a stale base that has no `authority`, the head table is judged and a pull request naming its own author as a decider passes, indistinguishable from the first introduction of `authority`; an up-to-date branch requirement or a merge queue removes the route. The amendment no longer calls the configuration a protected path in that case. `Status` stays Accepted. **Why:** the review of pull request 469 found the route undocumented. | **substantive** |
> | 2026-11-09 | Marked accepted risk 13 (a stale base) and the matching sentence of the decider-table amendment as accepted by the maintainer on 2026-11-09, replacing "documented by the orchestrator, pending maintainer review". `Status` stays Accepted. **Why:** maintainer decision of 2026-11-09: the stale-base route is an accepted risk, mitigated by requiring branches to be up to date before merging or a merge queue (`COND-0006`). | **substantive** |
> | 2026-11-09 | Stated in the outside-writes amendment that under `deny` a plain git repository or a submodule nested inside the session repository counts as outside (refused unless under `allow_paths`), and that under `shared` it stays unjudged like a separate clone (route 12). Documents existing behaviour; no decision changes. **Why:** the review of the outside-writes amendment found the nested case unstated. | **substantive** |
> | 2026-11-09 | Added an `## Amendment (2026-11-09, gate waivers)`, accepted by the maintainer on 2026-11-09: `authority.waivable` (`never`, `base`, `any`; undeclared is `any`) decides whether a waiver can silence `diff.decider-required`; under `base` only a waiver the base revision holds unchanged counts, under `never` none does, and the stricter of the base's and the head's applies; `init` proposes `never`, `doctor` warns when gates are declared and the key is not, and this repository declares `never`. Also under `base` and `both` the rule's `require_evidence` option is the stricter of the two. Risk 10 is reworded. `Status` stays Accepted. **Why:** Maintainer decision, 2026-11-09, on the review that found a non-decider accepting a record by adding a waiver record in the same pull request. | **substantive** |
> | 2026-11-09 | Corrected the decider-table amendment: the configuration is a protected path only when a gate lists `.urzua/config.yaml`, not because the base declares `authority`; `doctor` warns (`authority-config-gated`) when no gate lists it. Risk 3 now names a self-hosted gate whose binary is built from the pull request. `Status` stays Accepted. **Why:** the sixth review found the amendment claiming a protection the engine does not give, and risk 3 not naming the PR-built binary. | **substantive** |
> | 2026-11-09 | Added an `## Amendment (2026-11-09, rename pairing)`, first decided by the orchestrator, accepted by the maintainer on 2026-11-10: a record the base lacks is paired with a deleted record of the same `Stable-Id` only when the record id (type and number) is also the same, so a new Accepted record copying a deleted record's `Stable-Id` is a gated move; a slug rename still pairs, a renumber is new; the deleted copy is parsed with the type's declared document model; `status.transition-allowed` uses the same pairing. A slug rename that keeps the record id pairs even when its body changes, like an in-place edit; a case-only file rename reads as new and is gated. States that the merge-queue run passes no pull request number (the pull request run judged the change) and that an actors API outage passes the gate unless `require_evidence` is on. `Status` stays Accepted. **Why:** the seventh review reproduced a non-decider accepting a brand-new ADR with no finding. | **substantive** |
> | 2026-11-10 | Rename-pairing amendment accepted. **Why:** Maintainer decision, 2026-11-10. | **substantive** |
> | 2026-11-10 | Added an `## Amendment (2026-11-10, deletions)`, accepted by the maintainer on 2026-11-10: a record the diff deletes while its base `Status` is one a gate names is a gated move (`gated-deletion-without-decider`), judged like any other and reported by the post-command check; a rename pair is exempt; under `base`/`both` the base revision's type declaration also decides which deleted files are records of a gated type; gate statuses compare case-insensitively after trim and the status field is read case-insensitively for the gate. `Status` stays Accepted. **Why:** Maintainer decision, 2026-11-10, on the review that found a non-decider deleting an Accepted ADR with an ok check, and a lower-case `accepted` passing the gate. | **substantive** |
> | 2026-11-14 | Added an `## Amendment (2026-11-14, symlinked records)`, decided by the orchestrator on 2026-11-14, pending maintainer review: a tracked record file that is a symlink resolving outside the repository or outside its type's directory is refused by every command and reported by `check` as the blocking, unwaivable finding `identity.record-symlink-escapes`, on for every configuration and not switchable off; this closes the two-pull-request route (move a record out of the governed tree leaving a symlink, then accept the moved copy). `Status` stays Accepted. **Why:** an external review reproduced a non-decider landing an Accepted record through a symlink. | **substantive** |
