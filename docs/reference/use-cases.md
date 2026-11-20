# Use cases: a demo script

This page is a script for showing urzua to someone, on screen, talking to a coding agent in a repository that
uses it. It is a numbered list of scenes. Each one starts from a problem people actually have with decision
records, shows it happening without urzua, then shows urzua catching or preventing it. It is not a reference;
every scene links to the page that is.

How to read a scene:

- **Show it** mixes two kinds of step: a quoted prompt (what you type to the agent, in plain words) and a
  command (what you run, or what the agent runs).
- **What you'll see** names the visible result, using the real refusal text and JSON field names, abridged.
- **Configure it** names the config key that controls the behaviour.
- Every `urzua` command prints one JSON object on stdout; pipe it to `jq`. The examples below were run
  against the binary built from this repository's `main`, in a scratch repository, before they were written.
  Output is shortened and uses placeholder names (`alex` is the maintainer).
- A "scratch repository" below means any git repository with a `.urzua/config.yaml` that declares the record
  types in the scene. The config snippets give only the keys the scene needs; a complete working config is in
  [the config file](config.md) and [adopt an existing corpus](adopt.md).

One scene depends on work that is not on `main` yet and says so (scene 4). Everything else is on `main`.

## Why decision records need a linter now

Records used to be kept true by people. A significant decision happened rarely, a named person remembered
it, and the reviewers knew the conventions: which statuses exist, that "supersedes" must be answered by
"superseded by", who may accept a decision. Those conventions lived in prose and in reviewers' heads, and that
was enough.

Coding agents changed both ends. They read the records as instructions: ADRs, specs and `AGENTS.md` are the
context an agent builds from, so a stale or wrong record becomes wrong code, written confidently. And they
write the records: fast, in parallel, unattended, without the memory of the conventions. Decisions now arrive
at machine speed, and review still happens at human speed.

A prose convention does not survive that. No reviewer can hold every rule in their head at that rate, and an
agent follows what it reads in the nearby files, mistakes included. So the team turns its conventions into
code: a script in pre-commit and CI that checks the records. That is the moment the linter is born.

Then the linter becomes a second product to maintain. Every team writes the same few hundred lines (status
vocabulary, required fields, reciprocal links, who decided, does the code still match), each with its own bugs,
and the worst bug is a check that reports "0 errors" because it read nothing.

## What urzua is

urzua is that linter written once and driven by each repository's config instead of forked per team. It governs
decision records as data: ADRs, RFCs, specs, milestones, bugs, waivers and conditions are whatever record types
your `.urzua/config.yaml` declares, and every rule reads the declaration, not a hardcoded field name. The checks
report what they examined as well as what they found. On top of the checks it adds what a hand-written script
never gets to: guardrails in the agent's own editor hooks, evidence pins that tie a decision to the code that
realises it, and a board and roadmap computed from the records. It checks the shape of a record, never its
meaning, and says so (scene 18).

## Part 1: the problems

### 1. The decision says one thing, the code does another

Status answers "was this decided?", not "is this still true?". A decision is accepted, then the code it
describes changes, and nothing signals the record is now wrong. The next agent reads it and builds on it.

**Setup.** An ADR that names the code realising it, pinned to a content hash. The config declares the evidence
vocabulary and, for a symbol-level pin, a resolver for the language ([waivers and drift](waivers-and-drift.md#drift)):

```yaml
embodiment:
  drift: Drift detected
  tiers:
    - {state: Verified, evidence: test, claims_built: true}
    - {state: Implemented, evidence: code, claims_built: true}
  none: Not started
locator_resolvers:
  - name: rust
    glob: "**/*.rs"
    start: '^\s*(pub(\([^)]*\))?\s+)?((const|async|unsafe)\s+)*(fn|struct|enum|trait|union)\s+{symbol}\b'
    end: brace_balanced
record_types:
  adr:
    known_fields: [Embodiment, Realized-by]
    relation_fields: {embodiment_state: Embodiment, embodiment_locator: Realized-by}
    companion_fields: [[embodiment_state, embodiment_locator]]
    revision_log:
      marker: "**Revision log**"
      classes: [substantive, structural, evidence]
      class_column: Class
      date_column: Date
      tool_class: structural
      evidence_class: evidence
rules:
  embodiment.consistency: error
  embodiment.pin-stale: warn
```

`ADR-1` is `Accepted` ("retries stop after 30 seconds") with `Embodiment: Implemented` and
`Realized-by: "code:src/lib.rs#retry_ceiling_seconds"`, and `retry_ceiling_seconds` returns `30`.

The pin and re-pin commands write a row into the record's revision log, so the record needs a `**Revision log**`
table. The reason you pass to `fix --pin` or `--rebaseline` must be at least three words and 12 characters.

**Show it.**

1. Without urzua: prompt the agent, "Change the retry ceiling to 5 seconds for the new client." It edits
   `src/lib.rs`. The ADR still says 30 and nothing complains. Run `git checkout src/lib.rs` before the next step, so
   the pin is taken over the code the decision describes.
2. Pin the claim to the code it names. `urzua explain` prints which records govern a file; give it
   `'src/lib.rs#retry_ceiling_seconds'` (path, `#`, symbol) and it also prints the scoped pin to write. The
   command below writes the pin and a revision-log row:
   ```sh
   urzua explain src/lib.rs
   urzua fix --pin --apply --ids docs/adr/ADR-1-use-a-30s-retry-ceiling.md --actor agent \
     --reason "the constant is the ceiling the decision describes"
   ```
3. Repeat the prompt from step 1, then run `urzua check`.
4. List the stale pin: `urzua fix --rebaseline`.
5. Re-pin it, with a reason that says what changed. The reason is required and recorded:
   ```sh
   urzua fix --rebaseline --apply --ids docs/adr/ADR-1-use-a-30s-retry-ceiling.md --actor agent \
     --reason "ceiling lowered to 5s for the new client; ADR text updated separately"
   ```
   In an agent session the agent can run `/repin <record path>`, which runs this with a reason it must write
   after reading the diff.

**What you'll see.** Step 2's `explain` prints `governing_records` and, for the symbol form, a `scope.pin`
(`src/lib.rs#retry_ceiling_seconds@sha256:...`). Step 3 reports two findings: `embodiment.consistency`
("stated Embodiment 'Implemented' disagrees with 'Drift detected', computed from Realized-by") and
`embodiment.pin-stale` ("1 pinned evidence changed since it was pinned ... pinned a416a972afcb, now
ffc8c6bbb226"), each with a `suggested_action` that names the listing command (`urzua fix --rebaseline --ids <record>`); the
re-pin command itself is in the `message`. Step 4 lists `rebaselines` with each
pin's `recorded_hash` and `current_hash` and the changed `span`. Step 5 appends a row to the
record's revision log (class `evidence` where the type declares `revision_log.evidence_class: evidence`, otherwise `structural`), "Re-baselined (attested by ..., actor: agent) ... Reason: ...". A pin changed or moved by
hand, with no such row, fails `check --base` as `diff.pin-changed-unattested`.

**Configure it.** `embodiment` (states and which category lifts a record to which), `locator_resolvers`
(symbol-level pins; a line range `path#L10-L40` needs none) and the `embodiment.*` rule levels. To make a claim
lapse after a period, declare `embodiment.claim-expired`; `urzua reverify --apply --ids <record> --reason "..."`
renews it, and refuses to renew over changed evidence ([waivers and drift](waivers-and-drift.md#expiring-claims)).
`urzua explain <path>` is also the answer to "which decisions govern this file?" and the same question over MCP
(scene 27).

### 2. An agent moves a status that only a person may move

Someone, or something, changes `Status: Proposed` to `Accepted` in a later commit. No reviewer signed off and
no decider is recorded, yet every consistency check still passes. If the only gate is the record's own status
field, it works only for people who already mean to follow it.

**Setup.** Install the harness and declare who may decide ([the agent harness](agent-harness.md)). The
`authority` table below is what makes acceptance a person's act; `protected-paths` guards the config itself
(scene 5):

```yaml
authority:
  decider_table: base
  waivable: never
  deciders:
    maintainers: {logins: [alex]}
  gates:
    - {type: adr, to: [Accepted, Rejected, Superseded], by: maintainers}
    - paths: [".urzua/config.yaml", ".claude", ".github/workflows"]
      by: maintainers
harness:
  adapters: [claude-code]
  instructions_file: AGENTS.md
  outside_writes: shared
  guardrails:
    editor-hook:     {enabled: true, on_unjudged: allow, pending_pair: allow}
    session-start:   {enabled: true}
    post-command:    {enabled: true}
    acceptance:      {enabled: true}
    protected-paths: {enabled: true}
```

```sh
urzua harness install --dry-run
urzua harness install
urzua harness status           # "status": "ok"
```

`install` writes one drafting command per declared record type plus `/amend`, `/done`, `/repin` and `/reverify`
into `.claude/commands/`, the hook entries into `.claude/settings.json`, a block in `AGENTS.md` telling the
agent what it must not do, and `.urzua/harness.lock`. Commit all of it.

**Show it.**

1. Without urzua: with a `Proposed` ADR open, prompt "Implement ADR-2 and mark it accepted." The agent writes
   the code and edits `Status: Accepted` itself. `git checkout` that edit before the next step.
2. With the harness: repeat the prompt. The agent's edit of `Status` is refused before it is written.
3. Show the code side. A source comment `// Implements: ADR-2` while ADR-2 is `Proposed` is an error under
   `embodiment.back-pointer-decision-accepted`, and an `Embodiment` that claims "built" on a `Proposed` record is
   `embodiment.requires-accepted-status`.
4. In CI the same table is enforced on the pull request by `diff.decider-required` (scene 26).

**What you'll see.** The agent's tool call is denied, and the agent reads (abridged): `urzua harness
(acceptance): refused. this edit moves docs/adr/ADR-2-use-a-queue.md from Status 'Proposed' to 'Accepted', a
move the authority table reserves for a decider (gate: type adr to Accepted, Rejected, Superseded, by
'maintainers'); an agent cannot make it through the edit tools. A person makes this change by editing outside
the agent harness ...`. No override lifts it.

**Configure it.** `authority.gates` (which type, which target statuses, which group), `authority.deciders`
(logins), and `harness.guardrails.acceptance`. Never set `URZUA_HOOK_OVERRIDE` to get past a refusal: an
override needs a named human and is logged in `.urzua/override-log.md`.

### 3. Approving a decision in the session

Refusing the agent outright is safe but slow. If the person is at the keyboard, they can approve the one move
they mean, with a receipt.

**Setup.** As scene 2, with the mode switched on, and the session's `gh` login (or `git config user.name`) a
login in the gate's decider group:

```yaml
harness:
  guardrails:
    acceptance: {enabled: true, in_session: ask}   # refuse (default) | ask
```

Claude Code must be in its Manual (`default`) permission mode, and your own `gh` login must be listed in the
decider group (`logins: [...]`): otherwise the person is not asked and the edit is simply refused.

**Show it.**

1. Prompt: "Accept ADR-2." The agent edits `Status`.
2. Claude Code shows its permission prompt, with urzua's sentence as the reason. Read it aloud: it says what is
   approved (the whole file as shown, body included) and who may decide.
3. Approve once. The edit is written with a revision-log row recording the approval. Decline, and nothing is
   written.
4. Switch Claude Code to a mode that never prompts (for example `bypassPermissions`) and repeat step 1.

**What you'll see.** Step 2: the prompt's reason opens "urzua: approve a gated move? ADR-2 moves from Status
'Proposed' to Status 'Accepted'. gate: ... only the decider group 'maintainers' may decide. Approving writes the
whole file as shown, including any change to its body ...", and the edit shown includes the added row: `Recorded
as approved in an agent session, not verified: ADR-2 Status moved from 'Proposed' to 'Accepted' (gate: ...); by
<name>, as the host session reported them (name read from gh); the pull request check does not rely on it.`
Step 4 is refused with the same text as scene 2 plus "the person cannot be asked here. Ask the person to switch to
the Manual (default) permission mode and retry".

**Configure it.** `harness.guardrails.acceptance.in_session`. Details and the limits (a permission prompt can be
answered by something other than a person, and the name is whatever `gh` reports) are in
[asking the person about a gated edit](agent-harness.md#asking-the-person-about-a-gated-edit). The pull request
check, with the gate action's actors file, is the control; the in-session row is a record, not proof.

### 4. Approving a decision where the host shows no prompt

Available once PR #612 (ADR-105, currently proposed) lands. It is not in a release or on `main` yet, so do not
demo it from a released binary. What follows is from that pull request's text and tests; re-check it against the
page `agent-harness.md` after it merges.

In the modes where Claude Code shows no prompt (`bypassPermissions`, `acceptEdits`, `auto`, `dontAsk`, `plan`),
scene 3's step 4 changes: the refusal of the gated edit carries the exact input of one question for the agent
to ask with Claude Code's question tool.

**Show it.**

1. In `bypassPermissions` mode, prompt "Accept ADR-2."
2. The edit is refused and the agent asks the question. It begins `urzua approval <nonce>: approve ADR-2 moving
   Status from 'Proposed' to 'Accepted'? ...`, names the gate and the group, and offers two choices:
   `Approve <nonce>` and `Decline`.
3. Choose `Approve <nonce>`. The agent retries the same edit, with the same content, in the same session, and it
   is written without a further prompt, with the receipt row above naming the question and the host.

**What you'll see.** A one-use approval kept for five minutes under the git directory. A decline, free text, an
idle timeout, or a question urzua's hook did not see first approves nothing. The hook refuses a question whose
input already carries answers or whose wording or choices were changed.

**Configure it.** Same key as scene 3 (`in_session: ask`); the two extra hooks come with `urzua harness install`,
so run it again after upgrading. Say the limit on camera: this is a guard for the normal case, not a security
boundary. Another hook or host program that answers the question for the person, a shell command that writes an
approval file, or a person who approves without reading all get past it.

### 5. What the guardrails do and do not promise

Say this part plainly: the harness guards the normal case; it is not a sandbox and not a trust anchor.

This scene uses the harness config of scene 2 and the declared fields and `Deciders` requirement of scenes 8 and
14 (`known_fields`, `statuses.require`), so run it in the one scratch repository that has all three.

**Show it.**

1. Protected paths. Prompt: "Set `fail_on` to `warning` in `.urzua/config.yaml`." The edit tool is refused.
2. The editor hook. Prompt: "Set the Deciders on ADR-1 to TBD, and add an `Approved-on` date." The edit is
   blocked before it lands.
3. The post-command check. Have the agent run a shell command that writes a status, for example
   `sed -i 's/Proposed/Accepted/' docs/adr/ADR-2-use-a-queue.md`. It is not stopped. It is reported afterwards.
4. Show where the limits are written down: the "Accepted risk" section of `AGENTS.md` and
   [what it can and cannot promise](agent-harness.md#what-it-can-and-cannot-promise).

**What you'll see.**

- Step 1: `urzua harness (protected-paths): refused. .urzua/config.yaml is a protected path (gate: paths ...,
  by 'maintainers'); only that decider group may change it, and an agent's edit through the edit tools is
  refused.`
- Step 2: `urzua harness (editor-hook): blocked. urzua reports errors this edit introduced:` followed by one line
  per finding: `field.quality` (Deciders is Placeholder), `header.field-set-consistency` (`Approved-on` is not
  declared), `status.required-field-missing`. Fix the record; do not override.
- Step 3: after the command runs, a message to the person: `urzua harness (post-command): the command changed
  1 path(s) the authority table reserves for a decider ... It had already run, so nothing was prevented; CI's
  diff.decider-required checks who made it.`, with an `[acceptance] error` line per gated change, plus a
  finding line for anything else it broke (here an index row, scene 13). The agent gets a shorter line.

**The honest framing.** The hooks bind an agent that goes through its edit tools. They do not see a shell command
that writes a record (reported after, never blocked), `git commit --no-verify`, a pull request that changes its
own workflow, or an agent acting under a maintainer's own token: CI proves who authored the pull request, not who
decided. CI is the guarantee; the harness shortens the loop. The full accepted-risk list is in `AGENTS.md`.

**Configure it.** `harness.guardrails.editor-hook` (`on_unjudged: allow|deny`), `post-command`, `protected-paths`,
and `authority.gates` entries with `paths:`. `urzua doctor` and `urzua harness status` report whether it is
installed and in sync; run `urzua harness update` after upgrading urzua.

### 6. Accepted records get rewritten, and parallel edits clobber each other

People and agents edit decision records in place. An accepted decision is quietly reworded to match later
understanding, or one agent's edit undoes another's. The file still looks well formed.

**Setup.** A type with a declared revision log and status transitions:

```yaml
record_types:
  adr:
    statuses:
      values: [Proposed, Accepted, Rejected, Superseded]
      initial: [Proposed]
      transitions: {Proposed: [Accepted, Rejected], Accepted: [Superseded]}
    revision_log:
      marker: "**Revision log**"
      classes: [substantive, structural, evidence]
      class_column: Class
      date_column: Date
      tool_class: structural
rules:
  status.transition-allowed: error
  revision-log.change-class-required: error
  header.required-fields: error
```

**Show it.**

1. Prompt the agent to delete the `Deciders` line of an accepted ADR, or to move it from `Accepted` back to
   `Proposed`. The editor hook blocks the first (an introduced error) and the acceptance gate the second.
2. Add a revision-log row by hand with no `Class`, and run `urzua check`.
3. Move a status along an edge the type does not declare, then run `urzua check --base origin/main`.

**What you'll see.** The editor hook's `blocked` message (scene 5); `revision-log.change-class-required` on the row
with no class; `status.transition-allowed`, judged against the base of a `--base` run.

**The gap, stated plainly.** Nothing yet reports a rewrite of an `Accepted` record's Decision text. That is planned:
`MILE-350` (an opt-in rule for a change to a decided record's body other than an appended amendment) and
`MILE-88` (amendments are dated and keep the original Decision text). Until then the discipline is the `/amend`
command, which adds a dated amendment and a revision-log row and leaves what was decided unchanged, and your
review. A merge that silently drops a field value which is still valid on its own is not detected either.

### 7. Merge conflicts on evidence pins

Two branches that each touch a pinned file and re-pin the same record conflict at the second merge, in the pin,
in the version and in the revision-log rows. Redoing the re-pin by hand discards what was attested.

**Setup.** A milestone whose `Realized-by` pins a whole file, `code:src/client.rs@sha256:...`, and git.

**Show it.** In a scratch repository:

1. Branch `narrow`: narrow the pin to one function, which writes an attested row:
   ```sh
   urzua fix --pin --apply --ids docs/milestones/MILE-1-retry-client.md --actor agent \
     --reason "narrowed client pin to the send function only" \
     --replace "src/client.rs=src/client.rs#send"
   ```
2. Branch `repin` from the same commit: add a function to `src/client.rs`, then
   `urzua fix --rebaseline --apply --ids docs/milestones/MILE-1-retry-client.md --actor agent --reason "..."`.
3. On `repin`, `git merge narrow`. The record conflicts.
4. List before writing: `urzua fix --conflicts`.
5. Resolve, stage and continue:
   ```sh
   urzua fix --conflicts --apply --ids docs/milestones/MILE-1-retry-client.md --actor agent
   git add docs/milestones/MILE-1-retry-client.md && git commit
   ```

**What you'll see.** Step 4 lists the record with `kind: "pin"`, `outcome: "resolvable"` and, per pin, `base`, `ours`,
`theirs`, `merged` and whether it was `recomputed`; the narrowed locator carries `moved_from` and `moved_from_pin`.
Step 5 reports `"status": "resolved"` and keeps both sides' revision rows, adding one: "Pin recomputed on merge ...
No person wrote a reason for the merged state ... a reviewer of the pull request should confirm the merged pin."
`check --base` lists that pin for the reviewer. A conflict anywhere else in the record, or a pinned file that was
hand-merged, is `refused` with its reason and left alone; `next_action` names the first step that works. It never
stages. Detail: [when your re-pins conflict](waivers-and-drift.md#when-your-re-pins-conflict).

### 8. Blank, placeholder and pending collapse into one state

A `Deciders` field exists but nothing checks it, so records reach `Accepted` with it blank or holding "TBD". The
reverse also happens: a cleanup treats "awaiting review by X" as empty and overwrites live information. A field
never filled, one filled with a placeholder, and one deliberately marked pending are three facts.

**Setup.**

```yaml
record_types:
  adr:
    required_fields: [Status, Date, Author, Deciders, Stable-Id]
    statuses:
      pending: [pending, {pattern: '(?i)^(tbd|to be (decided|determined))\b'}]
      require:
        - {when: [Accepted], fields: [Deciders]}
rules:
  field.quality: error
  field.pending: warn
  status.required-field-missing: error
```

**Show it.** Three ADRs: `Accepted` with `Deciders: TBD`, `Proposed` with `Deciders: pending`, `Accepted` with
`Deciders:` empty. Run `urzua check`. Plain markdown lint passes all three.

**What you'll see.** `field.quality` reports the `TBD` as a placeholder and the empty value as empty, as separate
messages ("Placeholder -- not a real, present value", "empty -- not a real, present value"); `field.pending`
reports the `Proposed` one as a warning, "marked pending -- work declared unfinished", which is allowed to stand;
`status.required-field-missing` fires on the two `Accepted` ones only, because the requirement binds at that
status.

**Configure it.** `statuses.pending` and `statuses.placeholders` per type, `statuses.require` and `forbid`,
[config: `statuses`](config.md#statuses).

### 9. The linter says "0 errors" because it looked at nothing

A check reports success because a pattern matched no files, a header parser stopped finding the field, or a new
record was not in the set it reads. "Nothing wrong" is indistinguishable from "nothing read".

**Setup.** A repository with ADRs governed by a type that declares `dir: docs/adr`. A hand-written grep over
`docs/adr/*.md` for comparison.

**Show it.**

1. Without urzua: prompt "Move the ADRs to `docs/decisions/`." The grep script prints "0 errors".
2. Run `urzua check`.
3. Prompt "Rename `ADR-2-use-a-queue.md` to `0002-queue.md`", then `urzua check`.
4. Open the report's `rules_executed` for any rule.

**What you'll see.** Step 2: `files_examined: 0` and `records_read_by_any_rule: 0`, and `type.dir-matches-nothing`
errors ("record type 'adr' declares dir 'docs/adr', which does not exist -- every rule for this type examines
nothing"). Step 3: `filename.identity-pattern` reports the file instead of dropping it ("file name does not match
the 'TYPE-NNNN-slug.md' convention ... rename: 0002-queue.md -> ADR-0002-queue.md"). Step 4: every rule has a
`status` (`ran`, `not-enabled`, `not-applicable`) and a `population` with `eligible`, `examined`, `out_of_scope`
and `unreadable`; `eligible` greater than `examined` is not a pass. A new record that is untracked is not read, and
`check` says so in a `discovery.untracked-skipped` notice. Also in this family: `type.record-outside-declared-dir`
(a record-shaped file no type claims) and `config.scope-matches-nothing` (a rule that reached nothing).

**Configure it.** `type.dir-matches-nothing`, `filename.identity-pattern`, `type.record-outside-declared-dir`,
`discovery.include_untracked`. The report fields are in [`check` and its JSON output](check.md).

### 10. Records disagree about how they relate

ADR-3 says it supersedes ADR-1, but ADR-1 still reads as current and points nowhere.

**Setup.**

```yaml
record_types:
  adr:
    known_fields: [Supersedes, Superseded-by]
    pointer_fields: [Supersedes, Superseded-by]
    narrative_fields: []
relation_pairs:
  - {name: supersession, forward: Supersedes, inverse: Superseded-by, reciprocity: required, target_types: [adr]}
rules:
  relation.pair-reciprocity: error
  header.pointer-field-clean: error
  pointer.resolution: error
```

**Show it.**

1. Without urzua: prompt "Write ADR-3 replacing ADR-1." The agent adds `Supersedes: ADR-1` to ADR-3 and leaves
   ADR-1 alone.
2. `urzua check`, then `urzua graph`.
3. `urzua fix`, then `urzua fix --apply --ids docs/adr/ADR-1-use-a-30s-retry-ceiling.md --actor agent`.

**What you'll see.** `relation.pair-reciprocity`: "Supersedes claims a relation with ADR-1 (pair 'supersession'),
but ADR-1's Superseded-by does not reciprocally name ADR-3". `graph` prints `{from, relation, to, kind,
dangling}` edges, `kind` being `pointer` or `narrative`. `fix` lists the missing half as a Tier 2 repair; applying
it writes `Superseded-by: ADR-3` on ADR-1 and appends a structural revision-log row, and says "Status is not
changed here": moving ADR-1 to `Superseded` is a separate, gated step. `header.pointer-field-clean` rejects prose
in a pointer field, which is the claim-versus-mention trap. When the agent writes only one half through its edit
tool, the editor hook allows it and names the command that writes the other (`editor-hook.pending_pair`).

**Configure it.** `relation_pairs`, `pointer_fields`, `editor-hook.pending_pair: allow|refuse`.

### 11. References rot

A record or a code comment cites a decision that was renumbered, deleted or superseded, or a milestone is
"blocked on" something finished long ago. The link is well formed and nothing resolves it.

**Setup.** `pointer_fields` for fields that must resolve, `narrative_fields: [Blocked-on]` for prose references,
and `pointer.resolution`, `narrative-field.stale` and, optionally, `pointer.target-status` and
`citation.resolution` in `rules`.

**Show it.** A milestone with `Implements: ADR-9` (which does not exist) and `Blocked-on: MILE-1` where MILE-1 is
`Done`. Run `urzua check`.

**What you'll see.** `pointer.resolution`: "Implements: ADR-9 does not resolve to any discovered record".
`narrative-field.stale` (a warning): "Blocked-on: MILE-1 has reached a terminal status (Done) -- re-examine whether
this record's Status/Blocked-on should update". `pointer.target-status` reports a dependency on a rejected or
superseded record; `citation.resolution` checks ids cited in body prose, and `code.dangling-bug-reference` ids
cited in source comments. A pointer field holds clean, comma-separated references; a narrative field tolerates
prose and gets the staleness check.

### 12. Two branches take the same record number

Numbers are picked as "highest plus one" on a branch. Two sessions open at once pick the same number, and the loser
renumbers at merge time and leaves dangling references.

**Show it.** Two sessions, two branches, each prompted "Draft an ADR for X" (or "for Y"). Without urzua both write
`ADR-4`. With it, each goes through `urzua new adr "..."` (the `/adr` command does this):

```sh
git checkout -b a && urzua new adr "Branch a"      # ADR-4
git add -A && git commit -m a && git checkout main
urzua new adr "Branch b"                           # ADR-5, "skipped": [{"number": 4, "reason": "local"}]
```

**What you'll see.** `new` reports `path`, `display_number`, a fresh `stable_id` and a `skipped` list of
`{number, reason}` with `reason` `local`, `worktree <path>` or `open-pr <ref>`. `identity.collision` reports two
records with the same identifier, and `identity.open-pr-collision` (on a `--base` run) a number also used on a
fetched pull request head.

**Be honest.** The number is provisional, not reserved. `new` scans the working tree, local branches and refs
already fetched; a ref never fetched is invisible. Run `git fetch` first, and check open pull requests for the
same number before opening yours.

**Configure it.** `new.check_open_prs: true` fetches pull request heads so numbers taken there are skipped. The
`Stable-Id` never changes even if a record is renumbered.

### 13. The index page lies

A README table lists every decision with its status, maintained by hand, and says `Accepted` for records since
superseded. It is the table people and agents read.

**Setup.**

```yaml
record_types:
  adr:
    exclude_files: [README.md]   # the index is not itself an ADR
rules:
  index.freshness:
    level: error
    indexes:
      - path: docs/adr/README.md
        type: adr
        format: "- {id}: {title} ({Status})"
        begin: "<!-- urzua:index:begin -->"
        end: "<!-- urzua:index:end -->"
```

with those two marker comments in `docs/adr/README.md`.

**Show it.**

1. Without urzua: prompt "Supersede ADR-2 with ADR-6." The agent edits the ADR and forgets the table.
2. `urzua check`.
3. `urzua index` (lists what differs), then `urzua index --apply`.
4. `urzua board` and `urzua roadmap` (scenes 23 and 24): they are computed from the records on every run, so they
   have no copy that can go stale.

**What you'll see.** `index.freshness`: "entry ADR-2 lists Status 'Proposed', but the record's Status is
'Accepted'", and for a new record "ADR-3 has no entry in 'docs/adr/README.md'". `index --apply` writes the
artifacts and reports `outcome: written` with an `entries` count. The `/adr` and `/milestone` commands run
`urzua index --apply` as part of finishing a record.

### 14. Agents invent fields and vocabulary

An agent adds a plausible header field that exists nowhere else, or a status outside the agreed set. The parser
ignores it, and it looks authoritative to the next reader.

**Setup.** `known_fields` declared on the type (even as `[]`), `statuses.values`, `field_values`, and the rules
`header.field-set-consistency`, `status.undeclared-value`, `field.value-not-declared`.

**Show it.** Prompt "Accept ADR-5 and record the acceptance date." Unprevented, the agent may write `Accepted:` or
`Approved-on:`. With the harness the edit hook shows it at write time; without it, `urzua check` does.

**What you'll see.** `header.field-set-consistency`: "field 'Approved-on' is not declared (required_fields or
known_fields) for record type 'adr'". Related rules: `header.case-variant-key` and `header.field-case-mismatch`
(`status` against `Status`), `date.field-valid` and `date.field-order` (an acceptance date earlier than the
record's own date). Field-set governance runs only for a type that declares `known_fields`;
`config.known-fields-declaration-missing` makes that choice explicit.

### 15. The checks exist but nothing runs them

A linter lives in the repository, but no CI job or hook runs it, or a hook prints "passed" while ignoring a
failure.

**Show it.**

1. Prompt: "Is record linting enforced in this repository?" Without urzua the agent has to grep CI config.
2. `urzua doctor`.
3. `urzua hooks install --dry-run`, then `urzua hooks install`, and `urzua hooks verify --run` to prove the hook
   reaches urzua by running it.
4. `urzua harness status`.

**What you'll see.** `doctor` reports named checks with `status` `ok`, `info`, `warn` or `error`: `config-exists`,
`config-parses`, `record-type-dir`, `ci-wired` ("no workflow ... invokes `urzua check` or the urzua-gate action"),
`hook-wired` (with a `fix`: `urzua hooks install`), `harness-installed`, `harness-registration`, and the authority
declarations. `hooks install` reports `items` with a `state` per hook (`would-create` in the dry run). A hook that
finds blocking findings stops the commit and prints each one's rule, file and message; if urzua is not installed it
says the records were NOT checked and lets the commit through, because CI is the guarantee (`hooks.on_not_run:
block` stops it instead). `harness status` reports a modified block or a removed hook.

### 16. The tool knows the right value and leaves the record wrong

An audit computes that a record's stated progress is out of date, prints a warning and stops. Someone must
hand-edit the field to match what the machine just derived.

**Setup.** A milestone type whose status is derived from its evidence:

```yaml
record_types:
  milestone:
    known_fields: [Realized-by, Embodiment, Parent]
    statuses:
      values: [Planned, InProgress, Done]
      success: [Done]
      delivery: {delivered_tiers: [Implemented, Verified], unevidenced_tiers: [Not started], not_done: [Planned, InProgress], set_status: Done}
rules:
  status.delivery-agreement: error
  embodiment.consistency: error
```

A milestone needs a `**Revision log**` table (declare `revision_log` as in scene 6), or `fix --apply` returns
`partial-failure` rather than write a field it cannot record.

**Show it.** `MILE-1` is `Status: Planned`, `Embodiment: Not started`, with `Realized-by: "code:src/lib.rs,
test:tests/retry_test.rs"`, and both files exist.

1. Without urzua: prompt "What's done?" The agent reads `Status` and says MILE-1 has not started.
2. `urzua check`, then `urzua fix` (read-only).
3. `urzua fix --apply --ids docs/milestones/MILE-1-retry-client.md --actor agent`. In a session the agent does this
   with `/done <record path>`, which computes rather than lets the agent choose the state.

**What you'll see.** `status.delivery-agreement`: "Status is 'Planned', but the evidence computes to 'Verified', from
Realized-by: the work reads as delivered; set Status to 'Done' (`urzua fix --apply` does)". `fix` lists `repairs`
(`record`, `field`, `current_value`, `computed_value`); applying writes `Embodiment: Verified` and `Status: Done`
and one structural revision row per field, and touches no prose. `status.children-agreement` does the same for a
parent and its parts, and `sequence.status-agreement` for a record that claims to be ready while a dependency is
open. A state an `authority` gate reserves is never written by `fix`.

### 17. Warnings pile up, and exceptions outlive their reason

A warn-only check fires on cases everyone already knows, the list grows, and the one live warning sits unread.
Exceptions are added to quiet the noise and nobody can tell whether the reason still holds.

**Setup.** A waiver type, per [waivers and drift](waivers-and-drift.md#waivers):

```yaml
record_types:
  waiver:
    dir: docs/waiver
    prefix: WAIVER
    role: suppression
    required_fields: [Rule, Scope, Reason, Stable-Id]
    known_fields: [Expires, Variant, Key-path]
    effective_requires: [Reason, Expires]
rules:
  waiver.matches-nothing: warn
fail_on: warning
```

Without `Variant` and `Key-path` in `known_fields`, `waive --from-report` prints a `waive.narrowing` warning on
stderr; warn the audience or list them as above.

**Show it.**

1. `urzua waive --from-report` previews one draft waiver per rule and file for the current findings;
   `urzua waive --from-report --apply` writes them. Each carries `Reason: Pending` and no `Expires`, so it
   suppresses nothing yet.
2. Fill one in (`Reason: ...`, `Expires: 2026-12-01`), `git add` it, run `urzua check`.
3. Jump past the expiry: `urzua check --as-of 2026-12-05`.

**What you'll see.** After step 2 the finding stays in `findings` with a `waived` field naming the waiver record,
and only leaves the exit code; the draft you did not fill in is itself reported (`field.pending` on its Reason).
After step 3 the same finding is unwaived again, and `waiver.matches-nothing` reports the expired waiver. A
waiver with no `Expires` takes effect only for a rule the type's `permanent:` list names, with a `Category` from
that entry's vocabulary (and a `Justified-by` record where the category requires one); otherwise it is ineffective
and the finding is reported again. Levels `off`, `info`, `warn` and `error` keep the noise sorted: `info` is
counted and never blocks.

**Configure it.** `rules`, `fail_on`, the waiver type's `effective_requires` and `permanent`
([waivers and drift](waivers-and-drift.md#declare-the-waiver-type)).

### 18. Content disappears during a "structural" cleanup

An agent or script moves a set of documents onto a new template. Every required section is present and every check
passes, but a named dependency or a caveat that did not fit was dropped.

**Show it.** Prompt "Retrofit these three specs onto the new template", then diff them. Something specific is
usually reworded or dropped, and `urzua check` passes.

**This is not addressed, on purpose.** urzua checks shape, never meaning: a required section can exist, have the
right shape and pass every check while misrepresenting the decision, and `check` does not claim otherwise. The
honest line on screen is "urzua checks shape; the diff review is still yours." The nearest help is the
revision-log class requirement (scene 6), which forces a change to be declared substantive or structural, and the
planned body-change rule `MILE-350`.

## Part 2: features and configuration

### 19. Adopting an existing repository

**Show it.**

1. `urzua init --dry-run` proposes one record type per directory of numbered records and prints the config it would
   write; it moves and rewrites nothing.
2. `urzua init` writes `.urzua/config.yaml`. It never overwrites one.
3. `urzua check`. Expect findings about naming before content.
4. `urzua migrate filenames` (a dry run until `--apply`) renames to the type's convention, and
   `urzua migrate ids` backfills a `Stable-Id` into every record that lacks one, keeping the display number.
5. `urzua doctor`, then `urzua hooks install` (scene 15). For the agent side: `urzua harness install`.

**What you'll see.** `init` prints `proposed` (`name`, `dir`, `record_count`, `prefix`, `header_shape`),
`written`, `config_yaml` and `notices`. Read the notices: on a corpus whose records have no header block `init`
proposes `header_shape: none` and says no field is read, so a `Status:` line in the body is not judged. A rule
`init` withheld is named in a `rule-applicability` notice with what to declare first, so a rule is never on while
it examines nothing. `migrate ids` reports `missing`; `doctor` reports what is unwired. For findings you cannot fix
today, file a waiver with an expiry (scene 17) rather than turning a rule off. Detail: [adopt an existing
corpus](adopt.md).

### 20. Drafting records with the agent

The agent can draft a record without knowing the schema, and cannot decide it.

**Setup.** `urzua harness install` (scene 2). One drafting command is generated per declared record type, so
`/adr`, `/rfc`, `/milestone`, `/bug` and the like exist for the types your config declares: a repository that
declares no `bug` type has no `/bug`. `/amend`, `/done`, `/repin` and `/reverify` are always installed.

**Show it.**

1. Prompt, after a design discussion: "/rfc Use a queue for retries." The command runs `urzua harness record new rfc
   "<title>"`, which scaffolds through `urzua new` and prints `path`, `to_fill` and the starting status.
2. The agent fills the empty header fields and sections from the conversation only, then the command finishes:
   stages the record, regenerates declared indexes, runs `urzua check` on it and prints how many records it
   examined.
3. Ask it to accept the RFC. It will not: a `Status` of `Accepted`, `Rejected` or `Superseded` is a person's act
   (scenes 2 to 4).

**What you'll see.** A record filed `Proposed`/`Open`/`Planned` (the type's declared initial status), a `check`
result with its examined count (zero examined is a failure, never reported as success), and a refusal if the agent
changed a value the scaffold wrote. The assigned number is provisional (scene 12).

**Configure it.** `record_types.<type>.statuses.initial`, `.urzua/templates/<type>.md` (the body `new` appends),
`harness.adapters`.

### 21. `check`: findings, levels and what was examined

**Show it.**

1. `urzua check`, then read `status`, `blocking`, `files_examined` and `rules_executed`.
2. Edit the config so one rule is `info` and another `error`. Run again, and run `urzua check --fail-on warning`.
3. On a branch, change one record and run `urzua check --base origin/main`.
4. Run `urzua check docs/adr` and `urzua check --rules field.quality`.

**What you'll see.** Exit 0 when nothing blocks, 1 when it ran and something blocks, 2 when it could not run (the
report then has `"status": "not-run"` and an `error`). A gate treats 2 as a failure, never a pass. `info` findings
are reported and counted and never block; `fail_on` (`error` by default) sets which unwaived findings do, and
`--fail-on` overrides it for one run. `--base` reports only the records changed since the merge base, plus the
records that refer to them: the report's `scope` has `source: "git-diff"`, `base` and `changed_paths`. This is what a
pull request runs, so a legacy backlog never blocks a change that did not touch it. Diff rules
(`status.transition-allowed`, `diff.decider-required`, `diff.pin-changed-unattested`) are `not-applicable` on a
plain `check` and run under `--base`. Every report also carries a `next_action`: one command to run next.
Detail: [`check`](check.md).

### 22. Configuring rules

**Setup and show it.** In `.urzua/config.yaml`, a rule is a bare level or a table:

```yaml
rules:
  header.required-fields: error          # a bare level: off | info | warn | error
  embodiment.claim-expired:
    level: warn
    scope: changed                       # judge only what a `check --base` diff changes
  status.children-agreement:
    level: error
    types: [milestone]                   # judge only these record types
```

1. `urzua rules` lists every rule this build ships (`id`, `description`, `scope`, `judges`, `population_unit`,
   `spec`, `actions`); it reads no config.
2. `urzua explain --rule status.children-agreement` prints the rule, your `level`, its `options` and the declared
   `types`, all under the `.rule` key (`jq .rule`).
3. Run `urzua check` and read that rule's `population` in `rules_executed`.

**What you'll see.** `types` (ADR-104) narrows a rule to the record types it judges. A record of a type not listed
is not judged by that rule, its candidates are counted in the population as `out_of_scope` and `out_of_types`
(for example `eligible: 8, examined: 1, out_of_scope: 5, out_of_types: 5`; the numbers depend on your corpus), and the rule still reads every record as
context. A type name that is not declared, or an empty list, is a load error, and so is `types` on a rule that does
not judge loaded records. `scope: changed` is accepted by only four rules, listed in [config: rules](config.md#rules);
on any other it is a load error. A rule not named under `rules` does not run, and `check` lists it as
`not-enabled`.

### 23. The board

**Show it.**

1. `urzua board` writes `.urzua/cache/board.html`; open it from disk. It prints `file`, `bytes`, `record_count`
   and a `read` stamp (commit, branch, whether records or config have uncommitted changes).
2. `urzua board --serve` prints one JSON line with `listening`, `address` and `port`, then serves on 127.0.0.1.
   Open it in the browser, change a record, and watch it update itself (it polls `GET /stamp`).
3. Click through the page, then open *About this board* (the `i` button) and *Build your own view*.
4. `urzua board --prompt` prints the same prompt without the page.

**What you'll see.** One page in one file, no network request, laid out only from the configuration. A top bar with
search and the read stamp; tabs (All, Needs you, Under way, Ready, and the states records are in) and, on a wide
screen, a sidebar with one entry per declared record type; sections *Needs you* (a record whose current status has
a declared transition to a status an `authority` gate reserves, with the group), *Under way*, *Ready*, *Work* (a
tree where a type declares a `part_of` field), one list per other type, and records whose header could not be read.
*About this board* says in plain sentences which declaration produced each part, so nothing is taken on trust.
*Build your own view* holds a prompt, written from this repository's config, to paste into a coding agent so it
builds a custom view from `urzua export --format json`. The server writes nothing, answers `GET` only, and listens
on loopback; to reach it from another device put a proxy in front and pass `--allowed-host`. Detail: [the
board](board.md).

### 24. The roadmap, and milestones as epics

**Setup.**

```yaml
record_types:
  milestone:
    pointer_fields:
      - {name: Parent, targets: [milestone], part_of: true}
      - Depends-on
    statuses:
      values: [Planned, InProgress, Done]
      success: [Done]
      progress: [InProgress, Done]   # without a progress set the rule cannot say a parent has not started
relation_pairs:
  - {name: sequencing, forward: Depends-on, inverse: Blocks, reciprocity: none, cross_type: true, acyclic: true}
roadmap:
  pairs: [sequencing]
  work_types: [milestone]
rules:
  status.children-agreement: warn
```

**Show it.** `urzua roadmap`, then show an epic: a milestone with `Parent: MILE-3` on its children, with the parent
`Planned` while a child is finished.

**What you'll see.** `roadmap` (under the `.roadmap` key) has `waves`, `ready`, `critical_path`, `records`, `groups`, `needs_replan`, `cycles` and
`notes`; a repository declaring no `roadmap.pairs` gets a `no-sequencing-pair` note rather than an empty roadmap.
`status.children-agreement` reports the epic: "MILE-3 is Status 'Planned', not started, while a child has begun: 1
finished, 1 not-started". On the board the epic is a parent row with an "N of M done" count and *disagrees* where
its status and its children's do not agree. The same fact is `state_disagrees_with_children` in the export. See
[config: containment](config.md#containment-part_of).

### 25. Seeing the model: `explain`, `rules`, `graph`, `export`

**Show it.**

1. `urzua explain --types` is one compact entry per declared record type; `urzua explain --type adr` is every
   declaration with its line in the config.
2. `urzua explain --rule header.required-fields` says what a rule judges and its current level.
3. `urzua explain src/lib.rs` lists the records whose `Realized-by` names it (`governing_records`, each with
   `record`, `record_type`, `via`).
4. `urzua graph`, `urzua rules`.
5. `urzua export --format json --no-text`.

**What you'll see.** `--types` entries carry `name`, `at`, `prefix`, `dirs`, `required`, `optional`, `states` and
`references`; `--type` adds `header_fields`, `gates`, `conditions` and the template path. `explain --rule` returns (under `.rule`)
`id`, `description`, `scope`, `judges`, `population_unit`, `spec`, `level`, `waivable`, `options`, `types` and
`actions`. `export` returns `record_count`, `types`, `type_roles`, `read` and `records`, each with `id`, `type`,
`path`, `header`, `title`, `status`, `state`, `waiting_on`, `ready`, `parent`, `children`, `refers_to`,
`referred_by` and more; this is the stable input the board is built from, and so can your own view be. Detail:
[command reference](commands.md#explain).

### 26. The CI gate

**Setup.** The composite action, pinned to a commit sha, with full history (drift reads it). The workflow needs
`permissions: {contents: read, pull-requests: write}`, or the comment is skipped with a warning:

```yaml
- uses: actions/checkout@<sha>
  with: {fetch-depth: 0}
- uses: urzuahq/urzua/.github/actions/urzua-gate@<sha>
  with:
    version: v0.4.12
    base-ref: ${{ github.base_ref }}
    actors: true          # only for a repository that declares `authority`
```

**Show it.** Open a pull request that breaks a record and one that moves an ADR to `Accepted`.

**What you'll see.** One summary comment headed "urzua gate", created and then updated in place, with the counts from
`urzua check` and from `urzua check --base origin/<base>`, and an inline annotation per open finding. Both runs
happen and both are reported before the verdict; waived findings are counted, not listed. A run that could not run
fails the job and is shown as NOT RUN with its reason: a gate that did not run is never a pass. With `actors: true`
the action writes the pull request's author and approvers to a file for `diff.decider-required`.

**Say the limit.** `diff.decider-required` checks the pull request's author or a current approver, whichever the
authority table's declared `evidence` names (the author by default), against the gate's decider group. That is
authorship or approval, not who actually decided; agents under the maintainer's token look like the maintainer. A distinct
agent identity with `CODEOWNERS` and branch protection is not configured here. Detail: [CI](ci.md).

### 27. MCP: reading the records without a shell

**Setup.** Register the server in your agent's configuration; `urzua harness install` does not write it:

```json
{ "mcpServers": { "urzua": { "command": "urzua", "args": ["mcp"] } } }
```

or `claude mcp add urzua -- urzua mcp`.

**Show it.** Prompt: "Which ADRs were already rejected or superseded? Read why, so we stop proposing them again."
The agent calls `urzua_list_records` with `{"type": "adr", "status": ["Rejected", "Superseded"]}`, then
`urzua_show_record` with one of the ids. Then: "Which decisions govern `src/lib.rs`?", which calls
`urzua_get_findings_for_path`.

**What you'll see.** Five tools, all advertised read-only: `urzua_check`, `urzua_list_records`,
`urzua_show_record`, `urzua_get_findings_for_path`, `urzua_get_graph`. Each returns the report its command prints,
as `structuredContent` and as one text block. A superseded record's `edges` name the record that replaced it. No
tool writes, it listens on no port (stdin and stdout only), and the status names are your own: the server knows
none. Detail: [the MCP server](mcp.md).
