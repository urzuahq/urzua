---
Stable-Id: 01M20SH9CXA18HFPABZJQY795D
Status: Accepted
Date: 2026-09-08
Version: '0.18'
Author: beauwilliams
Subject: 'Repository agent guardrails -- mechanisms that constrain or guide an AI agent working in this repo; today, `AGENTS.md` and the drafting commands in `.claude/commands/`.'
Implements: MILE-36
Parent: —
---
# SPEC-19 — Repository agent guardrails

## Purpose

This spec governs this repository's own agent-guardrail mechanisms — whatever constrains or guides
an AI agent working in this repo, as opposed to `urzua`-the-product's own schema or CLI. **Today
that's exactly one mechanism: `AGENTS.md`.** If a second mechanism is ever added (a CI check
enforcing something `AGENTS.md` states, a second instructions file, a hook), it gets documented here
too, under its own section, rather than spinning up a new spec per mechanism — the same "coherent
feature area" reasoning `ADR-41` already applies to record types, applied here to the different
question of what governs an *agent's* behavior in this repo rather than what governs a *record*.

`AGENTS.md` itself was written directly by
[MILE-36](../milestones/MILE-36-write-agents-md-capturing-this-session-s-practices.md) with no
preceding RFC or ADR, the same "no decision point existed" situation as `doctor` (`SPEC-15`): it was
named as worth writing, not debated into existence. This spec exists to give that already-built
artifact the same governed home every other feature area has, going from the milestone straight to a
spec — skipping the ADR stage is legitimate when, as here, there was no real decision to record
(`RFC-4`'s own "common path, not the only path" already established this).

Distinct from a `spec`-the-record-type instance in one respect: `AGENTS.md` itself is not a governed
record (`urzua check` doesn't validate it — it has no `.urzua/config.yaml` entry, no header, no
`Status`). This spec is the governance layer *about* that file: what belongs in it, how it's
maintained, and a place for `Why` to accumulate as it changes, since `AGENTS.md`'s own prose doesn't
carry a revision log the way a record does.

No `Parent`: this spec doesn't narrow or split off from `SPEC-1` (the v0 CLI's own scope) — its
subject is a different axis entirely (agent behavior, not the product), and its real lineage is
already stated via `Implements: MILE-36`.

## The `AGENTS.md` mechanism: what belongs in it

Its own opening line draws the boundary already: `CONTRIBUTING.md` is the human-facing equivalent;
`AGENTS.md` states what that file doesn't need to. Concretely, as of this spec's `Version: 0.3`,
nine sections, grouped by what kind of guardrail they are:

- **What urzua is, and what it refuses to be** — the tool's own founding constraints, restated from
  `SPEC-1` so an agent has the project's actual engineering goals in view, not just process rules:
  config-driven over hardcoded, no silent no-op, three distinct field states, structural presence is
  not content-scope correctness, replacement (not coexistence) as the real success bar.
- **Git workflow** — never commit directly to `main`; branch and PR for every change.
- **Use the tool on itself** — track multi-step work as a `milestone`/`bug` via `urzua new`, not
  scratch notes; verify a cross-reference actually resolves via `urzua graph` before treating linked
  work as done; `urzua new`'s assigned number is provisional, not reserved (`MILE-89`); check
  declared config before hand-writing a header or field.
- **Verify before trusting** — run the actual command against real input rather than reading code
  and assuming it behaves as documented; a new rule needs an observed-failing planted-violation test
  before it needs anything else.
- **Never silently rewrite an Accepted decision** — a correction after `Accepted` gets a dated
  Amendment or a new narrowing/superseding record, never a silent edit to Decision/Consequences.
- **A record's Status: who sets it** — filing a record isn't deciding it; an ADR's or RFC's `Status`
  is the maintainer's. The PR that fully delivers a milestone sets its `Realized-by` and `Done`
  itself (`MILE-183`); covers any other field asserting a decision was reached on someone's behalf,
  not just the literal `Status` key.
- **Don't build speculative capability** — ship the smallest thing a real, evidenced case needs; the
  same discipline applies to *discovering* an existing shim whose justification no longer holds, not
  just to refraining from writing new ones.
- **Don't patch around a finding on your own new work** — a live finding (on code you wrote, code
  you're touching, or code you merely noticed) is signal to fix or file, never to quiet by widening a
  config list or ignore-list.
- **Before calling anything done** — `make ci` passes locally; a changeset exists for anything a
  person installing `urzua` would care about, per `ADR-29`.

## How it's maintained

Edited in place, like a spec — not append-only the way an ADR's Decision text is frozen. Unlike a
`spec`-type record, there's no mechanical enforcement of this (no `Version`/revision-log fields on
`AGENTS.md` itself to check); this spec's own revision log is where the "why this changed" history
actually lives, since the file being governed can't carry it.

**Every PR that changes `AGENTS.md` substantively adds a matching entry to this spec's own revision
log, in the same PR** — not as a follow-up. This is the mechanism for "track what's happening with
`AGENTS.md` over time" that the file itself can't provide on its own; skipping it here is exactly
the kind of gap this project's own history keeps finding elsewhere (a decision landing without the
spec describing its mechanism getting updated in the same pass — see `MILE-90`'s own `Why` for the
same pattern applied to a different feature area).

## The installable harness bundle (`ADR-89`, `MILE-198`)

The mechanisms above are this repository's own. `urzua harness` ships the same kind of mechanism to a
repository that adopts urzua, as one versioned bundle embedded in the binary, with the guardrail logic in
the engine and an adapter per agent that only translates. The contract is `SPEC-24`; the configuration is
`SPEC-3` ("The agent harness").

- **Commands.** `urzua harness install [--adapter NAME] [--dry-run]` writes the adapter's files, merges
  its hook entries into the agent's project settings file, writes the marked block into
  `harness.instructions_file` and writes `.urzua/harness.lock`. `status` compares the lock with the files,
  the merged entries, the block and the binary (missing, modified, out-of-date) and exits 1 on any drift,
  so a CI step can run it; it is a separate step and not part of `urzua check`. `update` rewrites an item
  whose hash still matches the lock and reports, without overwriting, one a person changed unless
  `--force`. `uninstall` removes what matches the lock, takes only urzua's hook entries and the marked
  block out of the shared files, reports and leaves what a person changed, and deletes the lock last.
  `adapters` lists the adapter directories of the bundle.
- **Shared files are merged, never replaced.** The settings merge adds the bundle's entries, keyed by
  event, matcher and handler, and leaves every other key and entry as written; a second install changes
  nothing. A file that is not valid JSON, or that holds an entry with a key of ours that a person edited,
  is refused (`not-run`, exit 2) and the entry named. A settings file that is not in the two-space JSON
  form is re-indented when written, and the install says so; its keys and entries are unchanged. The local
  settings file and the user-level settings are neither read nor written, and `status` says so.
- **The cores.** `urzua harness run edit` judges a proposed edit to a record as `ADR-69` and `ADR-78`
  describe (every enabled rule that can judge one record, `block_on`, the named override and `override_log`)
  and replaces the `jq`, `bash` and `python3` chain: no binary, no `jq` and an unreadable payload are
  verdicts the engine or the translator states. `urzua harness run session-start` reports, at the start of a
  session, a binary that is not on `PATH`, a bundle older or newer than the binary, a translator
  interpreter that is not on `PATH`, an unset project-directory variable the adapter expects, a hook entry
  that is not wired and a git hook that is not wired (`urzua doctor`, `MILE-179`).
- **The post-command check.** `urzua harness run post-command`, wired by the adapter as a `PostToolUse` hook on
  `Bash` and on only when `harness.guardrails.post-command` is named (`ADR-78` amendment, `MILE-202`), runs
  `check` on the records `git status` shows as changed after any Bash command, untracked ones included, and
  reports the unwaived findings to the agent as context. It reads no command, stages nothing, never blocks and
  always exits 0; pre-commit and CI remain the hard gate. With `acceptance` enabled it also reports, as an error of
  its own, a changed record whose status is one the `authority` table gates and that `HEAD` did not hold (a
  Bash-made acceptance), and, with `protected-paths` enabled, a changed, created or deleted path under a
  protected-paths gate (a Bash-made write to `.claude` or `.urzua/config.yaml`), as a report only. A command that commits its own
  change is covered too: the commit the last check saw is kept in `.urzua/cache/post-command-head` (seeded at session
  start) and the files changed since it are examined with the working tree's, a record compared with that commit
  rather than `HEAD`. A record failing at `HEAD` is listed as pre-existing,
  a record whose content hash was already reported is not checked again, a deleted record is not judged, and a
  `git` that is absent, a tree git does not know, a configuration that does not load or a `check` that cannot
  run is stated as NOT judged. The request, verdict and memory are in `SPEC-24`. This repository enables it in
  `.urzua/config.yaml` and has the bundle installed.
- **An unset project-directory variable is a stated verdict.** The adapter's command finds the translator
  from the repository root and the translator takes the project directory from the request's `cwd`, so the
  command no longer becomes `python3 -I "/scripts/hooks/..."` and exits 2, which blocks every edit with no
  reason.
- **Vendor neutrality.** The engine source names no agent vendor. A test scans every `src/` directory for
  vendor names; the bundle directory, where an adapter names its vendor by design, is outside `src/`.
- **Commands (`MILE-199`).** The install generates `.claude/commands/<type>.md` for every record type the
  configuration declares from one generic template (the type name and directory are the only things filled
  in; no command names a field, status or section), and installs `/amend`, `/repin` and `/done` and the
  helper `.claude/hooks/urzua-record.sh` they call (through `bash`, because the install does not set an executable bit). `/amend` only adds: it refuses any change to the Decision
  text, a removed list item or blank line included, and requires an added `## Amendment (date)` section and
  revision-log row. `/repin` refuses an empty reason, lists the stale pins, runs
  `urzua fix --rebaseline --apply --actor agent` and prints the `Re-pin:` line for the pull request body.
  `/done` runs `urzua fix --apply --actor agent` and never writes a state itself (`ADR-70`, amendment of
  2026-11-05); it stops when the engine computes nothing, and `fix` refuses a state an `authority` gate names.
  A type named like a lifecycle command is refused at install, and `urzua harness status` reports a command
  not yet installed as missing; `update` writes it. This repository runs entirely on the installed bundle
  (`MILE-204`): every command in `.claude/commands/` is generated, including `/adr`, `/rfc` and `/milestone`,
  and `urzua harness status` reports no drift. The guidance those three used to carry by hand lives where
  the engine already reads type-specific text: in the body of the type's template under `.urzua/templates/`
  (the `Decision` section of an ADR says a still-open direction is an RFC, the `Summary` of an RFC says a
  settled choice is an ADR, the `What` and `Done means` sections of a milestone say what a milestone is and
  how its criteria are written), so the generated text stays free of schema knowledge (`ADR-70`).
- **Nothing hand-wired remains.** `scripts/hooks/*` and `scripts/draft-record.sh` are deleted; the tests that
  pinned their verdicts (`record_edit_hook_integration.rs`, `block_on_integration.rs`) run the same planted
  cases against `urzua harness run edit` and the installed translator. The agent-instructions block states
  what an agent must not do and is advisory, only the verdicts bind.

## Who may accept a record: the acceptance guardrail (`ADR-89` section 4, `MILE-200`)

`authority` in `.urzua/config.yaml` (`SPEC-3`, "Who may accept a record") is one table read in two places,
and this section claims only what each place does.

- **Locally, the edit tools are refused.** With `harness.guardrails.acceptance` and `protected-paths`
  enabled, `urzua harness run edit` denies an agent's Write, Edit or MultiEdit that moves an ADR or RFC to
  `Accepted`, `Rejected` or `Superseded` (an ADR also `Retired`), or that writes `.claude`, `.git-hooks`, `.github/workflows`,
  `.urzua/config.yaml` or `.urzua/templates` (the templates shape every new record). Paths match ASCII case-insensitively, always, on every
  filesystem, so `.Claude/settings.json` is the protected file on a case-insensitive one; an edit through a symlink is judged as
  the file it resolves to; and the repository is the one that holds the file being edited, so an edit made from a `cwd` outside it is
  still judged. The deny cannot be overridden: the override of `ADR-78` is a name in an environment
  variable the agent controls, so it is not accepted for this verdict, and the output says so. A person
  edits outside the agent harness and is not affected; that is also the escape hatch when a denying
  guardrail cannot judge (a broken `authority` table, a missing binary), and `urzua doctor` and the verdict
  say so. An edit the adapter could not hand to the engine (an unreadable payload, an edit whose text the
  translator cannot place, an engine that cannot be run) is refused the same way when `acceptance` or
  `protected-paths` is on with `on_unjudged: deny`, and only then; where the configuration allows, it is allowed with a
  stated notice. This repository declares both gates and has the Claude Code bundle installed, so the refusal is
  wired into an agent session here.
- **In CI, the evidence is authorship.** `diff.decider-required` (`SPEC-2`), fed an `--actors` file the gate
  action (`.github/actions/urzua-gate`, input `actors: "true"`) writes from the GitHub API, requires the pull
  request's author, the default `pr-author` evidence, to be in the `maintainers` group for every gated move
  and path. With a single maintainer it proves authorship only: it cannot tell an agent acting under the
  maintainer's token from the maintainer. A local run has no file and reports `info`, so the maintainer,
  working by hand, can still accept a record and commit it.
- **Accepted risk.** The maintainer decided on 2026-11-05: "We just need to clearly document this for now
  and accept the risk." These get past the guardrail: (1) a Bash command that writes the status
  (`sed -i`, `python -c`, `cp`, `tee`, a heredoc), which the post-command check reports (a gated status move, or a change to a protected path; also when the same command commits it) and cannot stop;
  (2) editing the guardrail itself by shell or through a settings file the install does not read, and
  `git commit --no-verify`; (3) a pull request that changes its own workflow, since the gate runs from the
  head commit's workflow, which only `CODEOWNERS` and branch protection (the host's settings) could stop;
  (4) an agent acting as the maintainer, since CI cannot tell them apart. The required status check, branch
  protection and `CODEOWNERS` are the host's: `urzua doctor` says so and does not verify them. A distinct
  agent identity with `CODEOWNERS` on the protected paths is a future option and is not configured.
  `AGENTS.md` and the bundle's instructions block carry the same statement.

## What's deliberately not built

- **Making `AGENTS.md` itself a governed record type** — it would need a home directory, a header,
  and a `Status` that mean something. No single-instance record type exists elsewhere in this
  corpus, so that pattern isn't attempted here; this spec's own revision log is a
  sufficient history for a file this size, at this frequency of change, without inventing a new
  record-type instance for a single file.
- **A `check` rule verifying `AGENTS.md`'s content matches what this spec describes** — would require
  parsing prose for section presence, the same permanent content-scope ceiling SPEC-1 already names
  for structural-vs-content checking generally.

## References

- MILE-36 — wrote `AGENTS.md`; this spec's `Implements` pointer.
- ADR-41 — the "coherent feature area, decided editorially" principle this backfill follows, the
  same one that produced SPEC-6/9/10/16/17/18.
- RFC-4 — "RFC → ADR → Spec: the common path, not the only path" — the precedent for skipping
  the ADR stage when no real decision existed to record.
- SPEC-15 — `doctor`, the other feature area in this corpus with no ADR behind it, for the same
  reason.
- SPEC-1 — the source `AGENTS.md`'s "What urzua is" section restates; read that spec directly for
  the full success criteria and acceptance-suite bug classes this backfill only summarizes.
- MILE-89 — the display-number-race limitation `AGENTS.md` now names explicitly.
- BUG-9 — a real instance of the "flag a stale shim you find, don't just avoid writing new ones"
  guardrail `AGENTS.md` now states as a general rule.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial spec. **Why:** `AGENTS.md` was written directly by MILE-36 with no spec of its own — found live reviewing the dashboard, the same "implemented without any spec" gap ADR-41's review already found for `milestone`/`bug`/`waiver`/`adr`/`rfc`/`spec`, just for a file rather than a record type. | **structural** |
> | 2026-09-09 | `AGENTS.md` gained two sections: "A record's Status is a human decision, not yours to set" and "Don't patch around a finding on your own new work" (plus an extension to "Don't build speculative capability"). **Why:** two real, repeated agent mistakes this session — setting an RFC's `Status` without being asked, and widening `.urzua/config.yaml`'s `known_fields` to silence a warning on a newly-created file instead of fixing the actual gap. | **substantive** |
> | 2026-09-09 | `AGENTS.md` gained "What urzua is, and what it refuses to be" (grounding agent behavior in `SPEC-1`'s own constraints, not just process rules) and "Git workflow" (never commit directly to `main`), and had three existing rules broadened after an adversarial review found real loopholes: Status-ownership now covers decision-proxy fields, not just the literal key; "don't patch around a finding" now covers findings on touched-not-authored code; "don't build speculative capability" now covers discovering an existing stale shim, not just writing new ones. **Why:** the git-workflow gap is exactly how an agent came to edit files directly on `main` earlier the same session; the loopholes were found by deliberately construing the scenario where each rule's prior wording didn't actually stop a slightly-different-shaped version of the mistake it was written for. | **substantive** |
> | 2026-09-09 | A second, independent adversarial review of the previous entry's own changes found a real, self-defeating defect: `AGENTS.md` told agents to check `pointer_fields`/`narrative_fields` in `.urzua/config.yaml` as declared config, but those keys don't exist yet (`RFC-23`/`ADR-44` propose them; `MILE-90` hasn't built them), and `RecordTypeConfig` uses `deny_unknown_fields`, so following the instruction literally would break the build. Corrected to name only the two keys that exist today, with an explicit note on the proposed-not-built third axis. Also replaced an unfalsifiable "check for a PR number collision" instruction with two concrete manual commands, since no automated check exists yet (`MILE-89`). **Why:** this project's own "verify before trusting" discipline applies to `AGENTS.md`'s own claims about the codebase as much as to any other document — this backfill exists partly because that discipline had never been applied to `AGENTS.md` itself until an adversarial review did it after the fact, not before merge. | **substantive** |
> | 2026-09-09 | Backfilled this spec's "What belongs in `AGENTS.md`" section (stale since `Version: 0.1`, describing 3 sections against the file's actual 9) and added the standing rule that every substantive `AGENTS.md` change adds its own entry here, in the same PR — closing the exact gap that let the four rows above accumulate only after the fact, in one backfill, instead of one at a time as each change shipped. | **structural** |
> | 2026-09-09 | Reframed from "`AGENTS.md` agent instructions for this repo" to **"Repository agent guardrails"**: this spec governs this repo's agent-guardrail mechanisms in general, of which `AGENTS.md` is the sole one that exists today, not `AGENTS.md` specifically by name — leaves room for a second mechanism to join this same spec later instead of needing its own. File renamed to match. Added the new `Subject` field (`MILE-91`) and corrected `Parent` to `—` (`BUG-10`): this spec's real lineage is `Implements: MILE-36`, not a narrowing of `SPEC-1`'s v0-CLI scope. | **structural** |
> | 2026-09-09 | `AGENTS.md`'s "check declared config" bullet updated for MILE-90/ADR-44: `pointer_fields`/`narrative_fields` now exist and are checked alongside `header_shape`/`known_fields`, replacing the prior warning not to add those keys before they shipped. **Why:** this spec's own standing rule requires a matching revision-log entry in the same PR as any substantive `AGENTS.md` change. | **substantive** |
> | 2026-09-10 | Added a bullet to "Verify before trusting": when a change invalidates a claim in prose, grep the whole file or corpus for that claim's key phrase before calling the change done, not just the section being actively edited. | **substantive** |
> | 2026-09-10 | Added a bullet to "Use the tool on itself": before hand-writing a field the tool already computes (e.g. `Embodiment`), run `urzua fix --apply` rather than hand-asserting it. **Why:** found live -- `Embodiment: Verified` was hand-written on two ADRs this session (`ADR-44`, `ADR-45`) instead of using the command that exists to derive it correctly. | **substantive** |
> | 2026-09-17 | Config moves from `.urzua/config.toml` to `.urzua/config.yaml` (`ADR-52`, shipped in the same change). The described mechanism changes, not just its rendering. | **substantive** |
> | 2026-09-23 | Added "A narrower rule population is usually policy, not a regression", naming `ADR-53`/`ADR-57`/`ADR-54` by name as the settled decisions a reviewing agent should check before filing a coverage-narrowing or missing-diagnostic finding as a bug. **Why:** three consecutive `/code-review` rounds against this release independently re-flagged the exact same already-decided tradeoffs as defects, each requiring a fresh manual refutation; naming the pattern once, with the specific ADRs to check, is cheaper than refuting it a fourth time. | **substantive** |
> | 2026-09-23 | Removed the section added the same day, above. **Why:** the user rejected a prose fix for a code-review false-positive pattern as insufficient — a reviewer having to consult a policy note is weaker than the code making the distinction self-evident. `RFC-45`/`ADR-63` closed the specific `ADR-60` cross-record instance structurally (a dedicated rule, `relation.target-status-undeclared`, plus `Population`'s new `unreadable` disclosure); the prose stopgap is superseded, not left standing beside its structural replacement (`ADR-54`'s own reasoning, applied to this file). | **substantive** |
> | 2026-09-23 | Added two new sections: "The record-type workflow" (what each of the six record types is for, how they relate, and the pointer-vs-narrative field distinction) and "The review loop" (the fix/file/refute triage for a review finding, the repeating-shape escalation signal, and the branch/PR mechanics), plus a note in "Git workflow" to cut branches from an up-to-date `origin/main`. **Why:** the user asked directly whether `AGENTS.md` actually specified the operational workflow this session had been running, not just scattered pieces of it — it didn't; the record-type taxonomy and the review loop were both real, repeated, load-bearing parts of this project's own process that had never been written down in one place. | **substantive** |
> | 2026-09-23 | Corrected two inaccuracies CodeRabbit's review of the entry above found: the pointer-vs-narrative distinction wrongly implied `pointer.resolution` skips narrative fields (`SPEC-2` already states it covers `pointer_fields ∪ narrative_fields`; the real difference is `header.pointer-field-clean`'s format enforcement and `narrative-field.stale`'s extra check), and "Ship each round as one PR" made a changeset sound required for every PR, contradicting "Before calling anything done"'s own docs-only/CI-config/internal-refactor exemptions two sections later. **Why:** both verified real against `SPEC-2` and this file's own existing text before correcting. | **substantive** |
> | 2026-11-02 | `AGENTS.md`'s "`urzua new`'s assigned display number is provisional" bullet updated for `MILE-89`: `new` now also skips numbers on local branches and remote-tracking refs; the unfetched-remote limit and the manual PR check remain. **Why:** the bullet said the scan was filesystem-only and the check unbuilt. | **substantive** |
> | 2026-11-02 | Added two bullets to `AGENTS.md`. "Verify before trusting" gains "Verify against a real corpus, not only fixtures": changes to how records are read or identified must also run read-only against a real corpus, because an init header-shape fix passed every fixture test yet chose the wrong shape for 444 of 444 records on a real adopter corpus. "Use the tool on itself" gains "Every finding improves both our own workflow configuration and the engine": each gap is recorded as a milestone or bug for both the repo's config and the engine capability. **Why:** both were maintainer-approved after a week of findings showing fixture-only verification and engine-only fixes each left the real gap open. | **substantive** |
> | 2026-11-02 | The "every finding" bullet now asks both questions but records only the follow-ups a yes answer calls for. **Why:** review found the earlier wording required a milestone or bug even when no change is needed, which no record type represents. | **structural** |
> | 2026-11-04 | `AGENTS.md`'s "The review loop" no longer says an agent never merges with no standing exception: with the maintainer's authorization for the session, an agent may squash-merge (or enable squash auto-merge on) a PR once CI is green on the current head and an agent review found nothing blocking; force-push stays forbidden, and without that authorization the human merges as before. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | `AGENTS.md`'s "Use the tool on itself" gains the rule that an agent offers `/rfc`, `/adr` or `/milestone` when it notices a proposal, a made decision or decided unbuilt work, and drafts records through those commands. The commands in `.claude/commands/` are a second guardrail mechanism, governed by `ADR-70`: they scaffold through `urzua new`, hold no schema knowledge, and end in `urzua check`. **Why:** `MILE-177`; hand-written records were rejected by `check` for headers `urzua new` would have produced (`MILE-103`). | **substantive** |
> | 2026-11-04 | `AGENTS.md`'s Status rule now says who sets it: the PR that fully delivers a milestone sets its `Realized-by` and `Status: Done` in that PR (`status.delivery-agreement` fails when they disagree), while an ADR's or RFC's `Status` stays the maintainer's; `Version` `0.8` -> `0.9`. **Why:** maintainer decision of 2026-11-04, `MILE-183`: about thirty built milestones still read `Planned` because agents were told never to move a `Status`. | **substantive** |
> | 2026-11-05 | `AGENTS.md`'s "The review loop" now names the REST check-runs endpoint (`gh api repos/{owner}/{repo}/commits/<sha>/check-runs`) for polling CI instead of `gh pr checks`; `Version` `0.10` -> `0.11`. **Why:** `gh pr checks` uses GraphQL, which agent sandboxes can block. | **substantive** |
> | 2026-11-05 | Bumped to `0.12`. `AGENTS.md` catches up with the rules decided since it was last edited: seven record types with `condition` (`SPEC-23`); `Realized-by` as the one evidence field and the retired `Regression-test` (`ADR-84`); permanent waivers with a `Category` (`ADR-86`); the `info` level (`ADR-87`); the editor hook, `block_on`, the override log and never overriding it yourself (`ADR-78`); re-pins with an honest reason listed in the PR body (`SPEC-8`); amending an Accepted decision through a dated `## Amendment` section; and `make ci` runs `make records`, not `urzua check docs/`. **Why:** a review of the agent harness found the file silent on these, so an agent hitting a hook block or a re-pin had no instruction. | **substantive** |
> | 2026-11-06 | Bumped to `0.13`. Documents the installable harness bundle: `urzua harness install`, `status`, `update`, `uninstall` and `adapters`, the settings-merge and instructions-block rules, the `run edit` and `run session-start` cores, the stated verdict for an unset project-directory variable, and the vendor-neutral engine; this repository still runs `scripts/hooks` until `MILE-204`. **Why:** `MILE-198`, decided in `ADR-89`. | **substantive** |
> | 2026-11-06 | Bumped to `0.14`. Documents the post-command check: after any Bash command the harness runs `check` on the records git shows as changed (untracked included) and reports, never blocks; this repository enables it and has the Claude Code bundle installed. The editor hook is wired once, through the installed bundle; `scripts/hooks` stays on disk, unwired, until `MILE-204` deletes it, because both wired at once would block twice and write each override twice. **Why:** `MILE-202`, decided in the `ADR-78` amendment and `ADR-89`. | **substantive** |
> | 2026-11-06 | Bumped to `0.15`. Documents the acceptance and protected-path guardrails (`MILE-200`): the local refusal of an agent's edit-tool move to a gated status or write to a protected path (final, not liftable by the override), `diff.decider-required` fed by the gate action's actors file as authorship evidence only, and the "Accepted risk" statement (the Bash, shell-edited guardrail, self-modifying workflow and shared-token routes, accepted by the maintainer on 2026-11-05). The post-command check also reports a Bash-made move to a gated status, judged against `HEAD`. **Why:** `ADR-89` section 4, accepted by the maintainer. | **substantive** |
> | 2026-11-06 | Bumped to `0.16`. Documents the generated drafting command for every record type, `/amend`, `/repin` and `/done` and the helper they call (`MILE-199`): what each refuses, that `/done` sets a state only through `urzua fix --apply` and `fix` refuses a state an `authority` gate names, and that this repository commits the commands for the types it lacked while `/adr`, `/rfc` and `/milestone` stay hand-written until `MILE-204`. **Why:** `ADR-89` section 1, `ADR-70` amendment of 2026-11-05. | **substantive** |
> | 2026-11-06 | Bumped to `0.17`. This repository runs on the installed bundle: `/adr`, `/rfc` and `/milestone` are generated (their type-specific guidance moved into the type templates), `scripts/hooks` and `scripts/draft-record.sh` are deleted, the protected-path list no longer names `scripts/hooks`, and the commands run the helper through `bash` (a file the install writes carries no executable bit, so the direct call was denied). **Why:** `MILE-204`, `ADR-89` section 6. | **substantive** |
> | 2026-11-06 | Bumped to `0.18`. The post-command check also reports a changed protected path (report only) and covers a commit made inside the command, comparing with the commit the last check saw; the edit gates match paths ASCII case-insensitively, judge a symlink edit as its target, find the repository from the file edited when `cwd` is outside it, and `.urzua/templates` is a protected path here; an adapter failure is refused where `acceptance` or `protected-paths` is on with `on_unjudged: deny`. **Why:** the second-pass review of `MILE-198` to `MILE-204` found the shell route to the guardrails silent, a commit hiding the change, and four ways an edit went unjudged although the configuration said deny. | **substantive** |
