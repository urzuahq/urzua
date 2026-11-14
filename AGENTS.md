# Agent instructions

This file is for any AI agent working in this repository. `CONTRIBUTING.md` is the human-facing
equivalent; read both, but this one states things CONTRIBUTING.md doesn't need to.

`docs/specs/SPEC-19-repository-agent-guardrails.md` governs this file: what belongs in it and why,
and its revision log is where this file's own change history lives, since this prose carries none
itself. Any substantive edit here adds a matching entry there, in the same PR.

## What urzua is, and what it refuses to be

`urzua` is a governance engine for decision records (ADRs/RFCs/specs/milestones/bugs/waivers as
data, not prose convention), built on one founding claim: **one config-driven implementation
instead of every team hand-writing its own linter.** SPEC-1 states the constraints that follow from
that claim; read it before assuming a change is obviously fine.

- **Config-driven is the whole thesis, not a nice-to-have.** The same binary has to serve
  differently-shaped repos without forking. Anything that only works because of a hardcoded,
  type-specific assumption in Rust source (a field name, a record type, a relationship) is a
  regression against the founding claim, not a shortcut — this is what BUG-8/RFC-23/ADR-44 exist to
  fix, and the same question is worth asking of any new rule.
- **No silent no-op.** A check that finds nothing must be distinguishable from a check that ran on
  nothing — report what was examined, not just what was found. This is SPEC-1's own acceptance-test
  bug class, not a style preference.
- **Three distinct field states, not one.** Blank, placeholder, and pending are different things;
  collapsing them was the single most-repeated bug class across the tools this project replaced.
  Watch for the same collapse in any new field-state logic.
- **Structural presence is not content-scope correctness, permanently.** A required section can
  exist, have the right shape, and pass every check while still misrepresenting the actual decision
  — `check` cannot and must not claim to close that gap mechanically. Never design a rule as if it
  could verify a section's *meaning*, only its *presence and shape*.
- **The real bar is replacement, not coexistence.** SPEC-1's own success criteria: a tool that runs
  *alongside* the hand-written linter it was meant to replace has failed, whatever its test coverage
  says. Weigh new work against whether it moves toward an adopter deleting their own linter, not
  just toward more checks existing.

## The record-type workflow

Seven record types, each governing its own directory and each defined by its own `spec` (`SPEC-16`
adr, `SPEC-17` rfc, `SPEC-18` spec, `SPEC-6` milestone, `SPEC-9` bug, `SPEC-10` waiver, `SPEC-23`
condition) — read the
relevant one before hand-writing a record of that type, rather than inferring its schema from nearby
examples. What each is *for*, not just its fields:

- **`rfc`** — a proposal, not yet a decision. Lives in `Draft`/`Discussion` until an `adr`
  (`Derives-from: RFC-N`) accepts or rejects it. An RFC's own `Status` is independent of whether a
  governing ADR exists yet.
- **`adr`** — a point-in-time decision fact. Frozen once `Accepted`; never edited in place to match
  later understanding — amended via a dated `## Amendment` section instead (see "Never silently
  rewrite an Accepted decision" below).
- **`spec`** — a living, current-truth document, the opposite of an ADR: edited in place, a
  substantive change is a `Version` bump plus a revision-log entry, not a new record. `spec` is also
  what any *other* type's own schema is defined in (see above).
- **`milestone`** — this project's own backlog: decided-but-unbuilt work, explicitly deferred work,
  unrouted research findings. *Planned* work moving toward done.
- **`bug`** — a real defect already found, retrospective rather than planned, whose
  `Status` follows its `Realized-by` evidence (`ADR-84`; `Realized-by` is the one evidence
  field for every type, and the old `Regression-test` field is retired), so "nobody verified this is actually fixed" is
  a finding and not a judgement.
- **`waiver`** — a reviewed exception to a specific rule finding, never a config-level ignore
  list. A waived finding still appears in `check`'s output, only excluded from the blocking exit
  code. It carries an `Expires`, or is permanent (`ADR-86`): a waiver with no `Expires` takes effect
  only for a rule this repository's `permanent:` list names, with a `Category` from that entry's
  vocabulary (and a `Justified-by` record where the category requires one); otherwise it is
  ineffective and the finding is reported again.
- **`condition`** (`COND`, `SPEC-23`) — a fact nobody builds: an open question or an observed signal
  (an adopter asking for a feature). Filed `Open`; a milestone, ADR or RFC that waits on it points
  at it with `Depends-on`. It becomes `Met` only with `Met-by` naming the record that shows the
  signal, or `Dropped`; both are terminal.

The typical pipeline: an idea becomes an `rfc`; an `adr` derives from it to accept or reject; an
accepted decision that isn't built yet becomes a `milestone` (`Implements` pointing at the `rfc`/`adr`
it realizes); a defect found along the way becomes a `bug`; a `spec` is created or updated once the
resulting schema/behavior is real and needs documenting as current truth. Not every change visits
every stage — a same-turn bugfix still gets a `bug` record even though no RFC preceded it.

`Implements`/`Derives-from`/`Amends`/`Parent` are **pointer fields**; `Blocked-on` is a **narrative
field**. Both are resolved by `pointer.resolution` the same way — a stale or unresolvable reference
is a finding either way, not just a broken link. The real difference is format and what else runs on
top: a pointer field must hold clean, comma-separated references only (`header.pointer-field-clean`
enforces this; it never applies to narrative fields, which tolerate free prose around the reference),
and a narrative field gets one more check narrative fields don't: `narrative-field.stale` flags it
specifically once the record it names reaches a terminal status, a stronger, more specific signal
than `pointer.resolution`'s routine "resolves; target status = X". Use a pointer field for a
relationship that should read as a clean, enforced reference; narrative when it's prose that happens
to name a record, with staleness worth flagging once that record is done.

## Git workflow

Never commit directly to `main`. Create a feature branch and open a PR for every change, however
small — including a one-line docs fix. Check `git branch --show-current` before your first edit if
there's any doubt which branch you're on. Cut the branch from an up-to-date `origin/main`, not from
whatever a prior round left checked out — a branch cut before the last PR merged carries a stale base
and reopens whatever that PR just fixed.

## The review loop

This project's own defects are found by running reviews against real code, not by reading a diff and
trusting it — the discipline in "Verify before trusting" applies to a *reviewer's* claims exactly as
much as to a change's own author. The loop:

1. **Run a review.** This repository has `CodeRabbit` wired in at the GitHub level
   (`.coderabbit.yaml`) — trigger it by commenting `@coderabbitai review` on an open PR, regardless of
   which tool or agent is doing the triggering. If your own environment provides an additional review
   capability (a slash command, a separate reviewing agent), use it too — that's a capability of your
   tooling, not a project requirement, so don't assume the next agent working here has the same one.
2. **Verify every finding against the actual code before acting on it** — a review has produced
   fabricated findings before, and has also, this project's own history shows, missed a real defect a
   narrower earlier check should have caught. Neither "the reviewer said so" nor "I already checked
   this shape once" is verification; read the exact lines the finding names and confirm the failure
   scenario reproduces, or doesn't.
3. **Triage each verified finding into exactly one of three outcomes** — never a fourth "note it and
   move on":
   - **Fix it now** if it's mechanical and needs no design or schema decision. Ship it with a
     planted-violation test, observed failing before the fix and passing after (see "Verify before
     trusting").
   - **File it** (`bug` for an already-real defect, `milestone`/`rfc` for something requiring a design
     call) if fixing it means deciding something, not just correcting something. Don't rush a design
     decision to close a review round faster.
   - **Refute it** if it describes already-decided, deliberate design — `grep docs/adr/` for the
     field, rule, or behavior the finding names; an `Accepted` decision that already made this exact
     tradeoff means the finding is wrong, not the code.
4. **Watch for a repeating shape across findings**, not just each one individually. The same
   collapse-of-distinct-states defect (`BUG-125`) was independently found in three different rules,
   across two separate review passes, before it was fixed once in the shared mechanism instead of
   three times at the call sites. Two independent findings with the same underlying shape is a signal
   to fix the mechanism (or at least name the pattern to the user) rather than patch each instance and
   move on.
5. **Ship each round as one PR**: fresh branch off `origin/main`, a changeset (`.changeset/*.md`)
   per PR describing what an adopter would care about — skip it for the same cases "Before calling
   anything done" already exempts (CI config, internal refactors, docs-only changes) — then `gh pr
   comment <n> --body "@coderabbitai review"` after pushing, `gh api repos/{owner}/{repo}/commits/<sha>/check-runs` (REST; `gh pr checks` uses GraphQL, which can be blocked) polled until
   resolved. **Never force-push.** Merging
   belongs to the human running the session, with one exception: when the maintainer has authorized
   merging for the session, an agent may squash-merge (or enable squash auto-merge on) a PR once CI
   is green on the current head and an agent review found nothing blocking. Without that
   authorization, merging belongs to the human, every time.

## Use the tool on itself

This project's own backlog is tracked through `urzua`'s own record types, not scratch notes or
conversation history. If you're doing multi-step work worth remembering:

- **Track it as a `milestone`** (`urzua new milestone "..."`) if it's planned work — set `Phase`,
  `Track`, and `Implements` (comma-separated, pointing at whichever RFC/ADR it realizes) honestly.
  A milestone with no `Implements` yet is valid if the decision doesn't exist yet either.
- **Track it as a `bug`** (`urzua new bug "..."`) if you find a real defect — even one you fix in
  the same turn. Name the test that proves it in `Realized-by` as
  `test:path#name@sha256:<pin>` (`urzua explain path#name` prints the pin), and file the bug already
  `Fixed` in the PR that fixes it; a bug left `Open` while its `Realized-by` names a test is a
  `status.delivery-agreement` finding that `urzua fix --apply` repairs. A bug that is open on
  purpose with evidence is waived (a reason and an expiry, or a permanent waiver with a `Category`),
  not given a new status. Cross-link the milestone that plans the fix and the bug that describes the
  defect via a real `Implements` pointer, not just a prose mention — verify the edge actually
  resolves with `urzua graph` before treating the record as done.
- **Every finding is checked against both our own workflow configuration and the engine.** When a finding
  exposes a gap, ask two questions: does this repository's own `.urzua/config.yaml`, records or
  process need to change, and does the engine need a capability so the configuration can express
  the fix (the config-driven thesis)? Record each follow-up a yes answer calls for as a `milestone` or
  `bug`, not as prose; a no needs no record.
  Two past examples: lineage and dependency relations needed different field scopes (fixed by a
  `fields:` scope, generalized in RFC-47), and the dashboard's second parser drifted from the
  engine's (fixed by `urzua export`).
- Before assuming something isn't already tracked, run `urzua check docs/`, `urzua explain <path>`,
  or `urzua graph` against the real corpus rather than trusting memory of an earlier turn.
- **Draft records through the commands, and suggest them.** `/rfc`, `/adr` and `/milestone`
  (`.claude/commands/`, `ADR-70`) scaffold through `urzua new`, stage the file, run `urzua check`
  on it and report how many records it examined; they never set a decision status. When you
  notice a direction being proposed with real tradeoffs but not settled, offer `/rfc`; when you
  notice a choice actually being made ("let's go with X"), offer `/adr`; when decided work is
  left unbuilt, offer `/milestone`. Offer, don't draft unprompted, and let the user say no. No
  tool catches a missed suggestion; this is an instruction.
- **Before hand-writing a field the tool already computes, run the subcommand that computes it.**
  `urzua fix --apply` derives `Embodiment` from `Realized-by`'s actual tier — don't hand-assert
  `Verified`/`Drift detected`/etc. yourself and skip the command that exists to get it right.
- **`urzua new`'s assigned display number is provisional, not reserved.** It scans the working tree and
  every local branch and remote-tracking ref (MILE-89), skipping past numbers found there and naming
  them, with the reason, in the `skipped` list of its JSON (never on stderr). It is blind
  to refs never fetched into this clone, so `git fetch` first
  and, before opening a PR, manually check other open PRs for the same type/number — e.g. `gh
  pr list --search "in:title <TYPE>-<N>"` or `git log --all --oneline -- 'docs/<type>/<TYPE>-<N>-*'`
  — a collision found this way is expected and cheap to rename, not a bug in the tool.
- **Before hand-writing any record header or field, check `.urzua/config.yaml`'s
  `header_shape`/`known_fields`/`pointer_fields`/`narrative_fields` for that type.** Declared config
  is the source of truth for header shape, field vocabulary, and relationship-field behavior — never
  freehand an annotation convention (e.g. baking a status or explanation into a pointer field's own
  value) because it looks consistent with nearby records; check what's actually declared, and check
  it again against any rule you've just built or fixed in the same session, since matching an
  anti-pattern you just banned elsewhere is a real, observed failure mode. (`pointer_fields`/
  `narrative_fields`, MILE-90/ADR-44: a type declaring either must declare both explicitly, even as
  `[]` — omitting one is a `config.pointer-declaration-missing` finding, not read as "zero fields, on
  purpose." Every field named in either list must also be in that type's `required_fields`/
  `known_fields`.)

## Verify before trusting

This project's own history is full of defects found by actually running the tool against real
input, not by reading the code and assuming it does what its comment says. Before treating a claim
about behavior as true:

- Run the actual command against a real or disposable fixture. `check <path>` looking identical to
  `check docs/` was found this way — nothing about reading the code would have surfaced it.
- A new rule or fix needs a **planted-violation test, observed failing before the fix, then passing
  after** — not just a test that always passed. If you can't show it failing on the old code, it
  isn't verified yet.
- A test written **after** the code it covers has no failing state to observe, so **break the code
  and watch the test go red**, then restore. Same standard, opposite order. Three assertions in the
  `0.4.0` work passed against the bug they were written for — a population pinned to 100%, an
  `accepted || reported` pair that are exact complements, and a determinism check looping over one
  `HashSet` whose order is fixed within a process. Each derived its expected value from the same
  source as the thing under test, and reading them did not reveal it.
- Don't assume a design decision matches what's actually implemented. Grep the code, don't just
  read the ADR.
- When a change invalidates a claim in prose, grep the whole file (or corpus) for that claim's key
  phrase before calling the change done — not just the section being actively edited.
- **Verify against a real corpus, not only fixtures.** Any change to how records are read or
  identified (header parsing or shape inference, identity or filename parsing, discovery, config
  loading) must also be run read-only against at least one real corpus before it is accepted: this
  repository's own, plus any real adopter corpus available locally. A fixture-only pass is not
  verification. An init header-shape fix passed every fixture test and still chose the wrong shape
  for 444 of 444 records on a real adopter corpus, because a body blockquote outranked the real
  header. MILE-101 plans the automated version of this check.

## Never silently rewrite an Accepted decision

If understanding changes after an ADR/RFC is `Accepted`, don't edit its Decision/Consequences
content to match. Either add a dated `## Amendment (YYYY-MM-DD): <what changed>` section after the
Decision explaining the correction, leaving the Decision text as it was, or write a new record that
narrows/supersedes it and say so explicitly in both places. `Status` is never silently changed during an otherwise-structural edit.

## A record's Status: who sets it

Filing a record is not deciding it. Create records as `Draft`/`Open`/`Planned` and leave `Status`
there, with two exceptions. An ADR's or RFC's `Status` (`Accepted`, `Rejected`, `Superseded`) is the
maintainer's decision and is never yours to set, however finished the work looks. A `milestone` is
different: the PR that fully delivers it sets its `Realized-by` locators (the code and tests that
deliver it) and `Status: Done` in that same PR, with a structural revision row citing the PR, and the
maintainer reviewing the PR is the human decision. `status.delivery-agreement` fails when they
disagree, and `urzua fix --apply` writes the `Done` for you when the evidence is there. A milestone
the PR only partly delivers keeps its `Status` and gets a revision row saying what remains; put no
`Realized-by` locator on it until it is delivered, because the evidence is read as "delivered". A
bug follows the fixed-in-PR convention. The same rule covers any other field whose value asserts a
decision was reached on someone's behalf (`Supersedes / Superseded-by`, a vacated-number tombstone,
closing a cross-reference) and is not covered by the exceptions above. Distinct from the rule above
this one: that one guards an already-decided record's content; this one guards the decision itself.

## Who accepts a record, and the accepted risk

`authority` in `.urzua/config.yaml` (`ADR-89` section 4, `SPEC-3`) names the maintainer as the only
decider for an ADR or RFC moving to `Accepted`, `Rejected` or `Superseded` (an ADR also `Retired`) and for the protected paths (`.claude`,
`.git-hooks`, `.github/workflows`, `.urzua/config.yaml`, `.urzua/templates`; matched without regard to letter case, with Unicode case folding; and the files the harness keeps for itself: `.urzua/cache`, `.urzua/harness.lock`, the override log, `.git/hooks`, `.git/config`). Through `urzua harness run
edit` the engine refuses an agent's Write, Edit, MultiEdit or NotebookEdit, or a file-writing MCP tool, that does either (a
file hard-linked to a protected or gated one included, on Unix), and no override lifts it;
a person edits outside the agent harness. In CI `diff.decider-required` checks that the pull request's
author is the maintainer. That is **authorship, not who decided**: agents here run under the
maintainer's token, so it cannot tell an agent from the maintainer.

**Accepted risk (the maintainer, 2026-11-05: "We just need to clearly document this for now and accept
the risk"; narrowed 2026-11-06, `ADR-89`).** These routes still get past the guardrail, so the rule that you
never accept a record or touch a guardrail rests on this file and on the person reading the diff:

- a Bash command that writes a `Status` (`sed -i`, `python -c`, `cp`, `tee`, a heredoc) or a protected path.
  The post-command check reports it and cannot stop it. It reports a gated status move, a changed protected
  path (a gitignored one such as `.claude/settings.local.json` included, found by content hash from the
  baseline `session-start` records), a record hidden with `git update-index --assume-unchanged` and a commit
  the same command made on the checked-out branch (a commit on another branch or in a sibling worktree, and a
  push of another ref, are not seen). It reports a change once: the same
  change made again, or one that is still there at the next check, is silent. That is deliberate. An MCP tool
  that runs a shell command is not a Bash call: it is neither judged nor reported;
- skipping the commit hook with `git commit --no-verify`;
- a pull request that changes its own workflow, since the gate runs from the head commit's workflow;
- an agent acting under the maintainer's token: CI proves authorship, not who decided. A distinct agent
  identity with `CODEOWNERS` and branch protection is a future option, not configured;
- what the editor hook cannot read: a hard link is resolved on Unix only (elsewhere a write is judged by its
  own name), names are not Unicode-normalised, an MCP tool whose input has no string a path can be read from
  is not an edit it can see, an MCP tool that applies a patch or runs a shell command is an unparsed route, a change made before `session-start` took the protected baseline is not
  reported, the agent's user-level settings (outside the repository) are neither protected nor read, the
  project's `.claude/settings.local.json` is protected as a path under `.claude` but neither read nor wired by
  the install (a hook entry or `disableAllHooks` set only there is not seen by `status`), and a link swapped
  between the judgement and the write is not seen;
- a Bash command that writes what the edit tools cannot: `.urzua/cache`, `.urzua/harness.lock`, the override
  log, `.git/hooks` and `.git/config`. Nothing reports it;
- without `python3` or the translator, the editor hook's command reads `.urzua/config.yaml` as text and
  refuses every edit if it finds `acceptance` or `protected-paths` enabled without `on_unjudged: allow`; a
  configuration that scan misreads is read as not closed, and an invalid or empty `.urzua/config.yaml` (only
  a Bash write makes one, and it is reported) lets every edit through with a message;
- a repository the session did not start in can add restrictions, never remove them: an edit is judged by
  the session's repository and by every governed repository that holds the file, and a deny from any wins;
- the agent-instructions block in this file is not a protected path: an edit tool can remove it with no
  refusal, and only `urzua harness status` reports it modified;
- a waiver record is not gated, so an edit-tool waiver can silence the CI gate for a gated move made by a
  Bash command.

Do none of these.

## Findings, levels and the editor hook

- **Levels.** A rule is `off`, `info`, `warn` or `error` (`ADR-87`). An `info` finding is reported and
  counted but never trips `fail_on` or sets `blocking`; it is not a finding to silence.
- **The editor hook** (`.claude/settings.json` via the installed bundle, `ADR-69`, `ADR-78`) judges a
  `Write`, `Edit`, `MultiEdit` or `NotebookEdit`, or an MCP tool call that carries a file path and the file's
  content, of a governed record before it is written, by running `urzua check
  --stdin-path`. It blocks an unwaived error the edit introduced (`block_on: introduced`; `block_on:
  all` in `.urzua/config.yaml` blocks any). Warnings are shown, not blocking. It does not cover
  edits made through `Bash`, and it allows an edit it could not judge with a message saying so (except
  where `acceptance` or `protected-paths` is on with `on_unjudged: deny`, which refuses one), so
  run `urzua check` before committing either way.
- **When it blocks, fix the record.** Never set `URZUA_HOOK_OVERRIDE` or `--override` yourself. An
  override needs a named human, and every one is appended to `.urzua/override-log.md`, where
  `override.unreviewed` reports it until a person fills `Reviewed-by`. Don't fill `Reviewed-by`
  yourself either.
- **Re-pinning evidence.** A stale `@sha256:` pin on `Realized-by` is repaired with `urzua fix
  --rebaseline --apply --ids <record> --by <name> --actor agent --reason "<delta>"` (`SPEC-8`). `--by`
  is attribution, not authentication, so it names the human who authorised the re-pin and only with
  that person's authorisation. A verified `gh` login replaces `--by` (`attestation` in
  `.urzua/config.yaml`; the report's `identity.replaced_by` names what it replaced), and under the
  maintainer's token the row would otherwise name the maintainer as the attester of an agent's work,
  so always pass `--actor agent` (the row records the source and the actor kind; the label is your
  claim, not proof). The reason must say what actually changed in the pinned code, not
  that the pin was stale. Look at the diff of the pinned file before re-pinning, list every re-pin
  and its reason in the PR body so a reviewer can check it, and never re-pin to quiet drift you have
  not read.

## Don't build speculative capability

Ship the smallest thing that solves a real, evidenced case. This project has walked back more than
one over-eager build this way — after checking whether real demand existed and finding none yet.
When a bigger version of something is plausible but unproven, build the narrow version and name the
bigger one as a milestone, don't build it preemptively. The same applies to backward-compatibility
code: don't add a shim, tolerance, or default "for when someone needs it" without a real case in
front of you — this tool has no external adopters yet whose behavior needs preserving.
(Note, 2026-11-05: this holds through Phase 0 and expires at the Phase 1 switchover (`MILE-102`), when a real
adopter's corpus exists and `RFC-49`'s compatibility contract (proposed in `ADR-77`) takes over; `MILE-118`.)

If you find *existing* compatibility or tolerance code whose stated justification no longer holds
against this repo's own real, current state — a "for future adopters" comment where no adopter
exists, a "for a mixed corpus" rationale where the corpus already converged — that's a live finding
worth its own bug record, the same as any other defect. Don't leave it un-flagged just because you
didn't write it, and don't silently delete it either without a decision on the record.

## Don't patch around a finding on your own new work

A live finding is signal, not a bug in the checker — whether it's on code you just wrote, code
you're merely touching, or unrelated code you happened to notice. Fix the real gap it names, or
leave it firing while the decision gets made separately — don't widen a config list, an enum, or an
ignore-list just to make a diff come up clean, regardless of whose diff it is or how old the
underlying issue is. Same rule as the stale-shim case above, applied to a live checker finding
instead of something you noticed by reading code — in both cases, file the defect, don't quietly
absorb it.

## Before calling anything done

- `make ci` passes locally (fmt, clippy, build, test, `make records`, the rule-table check).
- A changeset exists in `.changeset/` for anything a person installing `urzua` would care about
  (ADR-29) — skip only for CI config, internal refactors, or docs-only changes. It carries `closes:`
  (a one-line list of the records it closes, `[]` for none) and the record in it is closed in the same
  PR (ADR-85).

## Reading records over MCP (optional)

`urzua mcp` is a read-only MCP server over stdio. An agent that registers it can list records, read one with its relations, ask which records govern a path, run `check` and read the graph without a shell. To see what was already rejected or superseded, call `urzua_list_records` with `{"type": "adr", "status": ["Rejected", "Superseded"]}`, then `urzua_show_record`. Register it in your own MCP configuration (`{ "mcpServers": { "urzua": { "command": "urzua", "args": ["mcp"] } } }`); `urzua harness install` does not write it. It has no tool that writes. See `docs/reference/mcp.md`.

<!-- urzua-harness:begin -->
## urzua harness

This section is written by `urzua harness install` and is rewritten between its markers on every install or update. Do not edit it by hand; change `harness` in `.urzua/config.yaml` instead.

The record-edit guardrails are installed in this repository. What an agent must not do:

- Do not accept or reject a record yourself. Moving a decision record to an accepting status is a person's act.
- Do not set a hook override to get a blocked edit through. A named override is recorded for review; it is the maintainer's to take.
- Do not edit the harness itself: the agent's hook settings, the hook files under `.claude/hooks`, `.git-hooks/`, `.urzua/config.yaml` and the CI workflows are protected paths, and so are the files the harness keeps for itself (`.urzua/cache`, `.urzua/harness.lock`, the override log, `.git/hooks` and `.git/config`).
- Write records with their drafting commands and `urzua new`, not by hand; run `urzua check` before committing.

Where `authority` is declared, the engine refuses an agent's Write, Edit, MultiEdit or NotebookEdit, or a file-writing MCP tool, that moves a record to a status a gate names or writes a protected path (a hard link to one included), and no override lifts it; a person edits outside the agent harness. That is the whole claim. Accepted risk, documented by the maintainer on 2026-11-05 and narrowed on 2026-11-06 and 2026-11-07: a shell command that writes a status or edits a guardrail or a file the harness keeps (the post-command check reports it after the command, a gitignored protected file, a record hidden with `git update-index --assume-unchanged` and a commit the same command makes included, when that guardrail is on; it reports a repeat of a change it has already reported only once; nothing stops it), `git commit --no-verify`, a pull request that changes its own workflow, and an agent acting under the maintainer's token (CI proves who authored the pull request, not who decided) all get past it. Do none of these.

The editor hook judges a Write, Edit, MultiEdit or NotebookEdit of a record before it lands. It cannot see a shell command that writes a record, so run `urzua check` after one. Only the verdicts of `urzua harness run` bind; this text is advisory.
<!-- urzua-harness:end -->
