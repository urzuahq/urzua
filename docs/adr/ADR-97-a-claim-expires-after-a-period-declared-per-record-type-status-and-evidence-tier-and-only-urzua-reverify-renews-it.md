---
Stable-Id: 01M6V2J0H6WNAVSFWECX801SSC
Status: Accepted
Accepted-on: 2026-11-07
Date: 2026-11-07
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: null
---
# 97 — A claim expires after a period declared per record type, status and evidence tier, and only urzua reverify renews it

## Context

Line references are to `origin/main` at `0ad09609`. Measurements are from a whole-repository run of the
release build at that commit on 2026-11-07.

`MILE-129` was filed as the decision whether drifted evidence may carry a hash pin, a grace window, an
attributed re-baseline and a claim age. All four are built (`ADR-67`, `ADR-74`, `MILE-181`). It was
reopened because the fourth piece is built and not used: `claim_max_age_days` is not declared in
`.urzua/config.yaml`, and its last Done-means item, "`claim_max_age_days` is exercised in urzua's own
`.urzua/config.yaml`", has no evidence. The maintainer's direction on 2026-11-07 is to improve the engine
and make it configurable, and to give this use case an expiration date: a claim's verification expires,
and someone has to re-verify it and refresh the hashes. This is the model of an existing tool the
maintainer built.

Why declaring today's option does not work, measured here:

- **One number for the whole rule.** `claim_max_age_days` is a `u32` on `embodiment.consistency`
  (`urzua-core/src/config.rs:529`), read for every record the rule judges
  (`rules/embodiment.rs:476`, `:593-609`). One value cannot say that a `Done` milestone's evidence goes
  stale faster than an `Accepted` ADR's. With the value at 14 or 30 the rule reports 14 records (12 ADRs
  and 2 specs); at 45, 13; at 60 or more, none. Run on 2026-11-07 against a 64-day history, every
  value is either a wall of ADR warnings or silence.
- **It is blind to most claims.** The finding rides on `embodiment.consistency`, which judges a record
  that states `Embodiment`: 65 of the 119 eligible fields. 364 records carry a `Realized-by` locator
  (36 ADRs, 148 milestones, 159 bugs, 21 specs). A `Done` milestone or a `Fixed` bug claims delivery
  through `Status` (`ADR-84`, `ADR-96`), states no `Embodiment`, and so is never judged. These 307 claims
  (148 milestones, 159 bugs) of the 364 can never expire.
- **Refreshing is silent.** Age counts from the commit that last touched the `Realized-by` line
  (`urzua-cli/src/discovery.rs:949-1002`). Any edit to the line resets it: a reformat, a bulk rewrite, or
  `urzua fix --rebaseline`, which rewrites the line (`SPEC-8` notes that rewriting a pin touches it). The
  corpus shows it: the median age of a milestone's claim is 0 days and the maximum 3, which fits
  `MILE-215` having rewritten every Done-means section on 2026-11-07, though no one re-verified a claim. The
  rule's own table says why it offers no action for the finding (`rules/table.rs:644`): "touching the
  field to reset the age without checking would defeat the rule". Today the only way to clear the
  finding is the one the rule is meant to prevent.
- **There is no way to say "re-verified".** The record has no field for the date the evidence was
  last checked. The one that exists, `Last-audited` (`SPEC-18`), is declared by the `spec` type only and
  is read only to move the drift grace window (`drift_audit_field`, `MILE-166`). No spec carries it.

Measured ages (days since the `Realized-by` line was last touched, by `git blame`, on the records that
carry one):

| Type | Records | Median | Maximum |
|---|---|---|---|
| ADR | 36 | 2 | 60 |
| Bug | 159 | 2 | 52 |
| Milestone | 148 | 0 | 3 |
| Spec | 21 | 2 | 49 |

The repository is 64 days old (first commit 2026-09-05), so no claim here can exceed 64 days yet.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. A rule-wide `claim_max_age_days`, as today | Built; one number | One period for every type and status; reaches only records that state `Embodiment` (65 of 364); the level is the rule's, so an expiry cannot be advisory while drift is not; a refresh is any edit to the line, so it is silent; this repository cannot declare a value without a wall of ADR warnings (14 at 30 days) |
| B. `level: info` with `scope: changed` (`ADR-87`, `ADR-90`) | No new mechanism; nothing fails; the nudge appears when a record is touched | Expiry exists for the record nobody touches, and `scope: changed` judges only touched records, so a stale claim stays silent until it is edited. It also does not say how to refresh, or prove one |
| C. No expiry | Nothing to build or maintain | Evidence is trusted forever once pinned; a pin that still matches proves the file did not change, not that the claim still holds. `MILE-129` stays open on an item nobody will do |
| D. Extend the option on `embodiment.consistency` into a map of periods | Reuses the rule | The rule judges only records that state `Embodiment`; one level for drift and expiry; waivers, `info` and `scope` attach to the whole rule; the finding stays inside a rule whose purpose is agreement |
| E. A new rule with periods per type, status and tier, a declared verification date, and a `reverify` command (chosen) | Per-type and per-status periods; its own level, waivers and scope; reaches every record with a locator; a refresh is an attributed, recorded act; off until declared | A new rule, a new command, a new field on the types that opt in, and a recurring workload the repository accepts |

## Decision

In the context of a claim whose evidence is pinned and still matches but was last looked at weeks ago,
facing a rule that has one period for every record, cannot see most claims and lets any edit to the line
count as a refresh, we decided **to add an opt-in rule, `embodiment.claim-expired`, that expires a claim
after a period declared per record type, status and evidence tier, at the level the configuration
declares, and a command, `urzua reverify`, that is the one attributed way to renew it**, to achieve a
claim that must be re-verified on a schedule the repository chooses and a renewal that says who did it,
what the tool checked and what it could not, accepting a new rule and command, a date field on each
opting-in type, and a recurring review workload.

Maintainer direction, 2026-11-07: improve the engine, allow for configuration, and give this use case an
expiration date. This ADR is Proposed until the maintainer accepts it.

**"Refresh the hashes" is two acts, by design.** `reverify` never moves an existing pin: it re-checks
that each pin still matches and dates the claim. A changed pin is moved by `fix --rebaseline`, with its
own reason, and only then `reverify` renews. The one hash `reverify` does write is a **first** pin on a
locator that has none (section 4), because a claim cannot be renewed over evidence that nothing but git
history vouches for.

### 1. The rule and its configuration

`embodiment.claim-expired` is opt-in: a configuration that does not declare it runs no new rule, reports
nothing new, and loads and reports as before, byte for byte. The existing `claim_max_age_days` is
unchanged and stays supported. Declaring both is a load error that names both: they would report the same
claim twice.

```yaml
rules:
  embodiment.claim-expired:
    level: warn               # info | warn | error; the ceiling for every entry below
    anchor_field: Last-audited
    require_pin: [code, test] # evidence categories whose locators must be pinned to be renewed
    max_batch: 10             # most records one `reverify --apply` may name
    periods:
      - {type: milestone, status: [Done],             days: 45, spread_days: 45}
      - {type: bug,       status: [Fixed],            days: 60, spread_days: 60}
      - {type: spec,      status: [Shipped, Accepted], days: 60, spread_days: 60}
      - {type: adr,       status: [Accepted],         days: 90, spread_days: 90}
      # a narrower entry wins: spec-tier evidence only, a longer period, advisory
      - {type: adr, status: [Accepted], tier: Specified, days: 180, level: info}
```

- **`periods` is a list of maps.** No string mini-language (`ADR-64`). An entry has `type` (a declared
  record type), optional `status` (a list of that type's statuses), optional `tier` (a state of the
  `embodiment` declaration, that is `Verified`, `Implemented` or `Specified` here), `days` (whole days,
  1 or more), an optional `spread_days` (whole days, 0 or more, default 0; section 2) and an optional
  `level`.
- **`require_pin` and `max_batch` are rule-level.** `require_pin` is a list of evidence categories as the
  `embodiment` declaration names them (`code`, `test`, `spec` here); the engine knows no category, and an
  unknown one is a load error. It defaults to empty, which is today's behaviour. `max_batch` is a whole
  number, 1 or more, and defaults to 1.
- **The most specific entry wins.** An entry naming `type`, `status` and `tier` outranks one naming
  `type` and `status`, which outranks `type` alone. Two entries of equal specificity that can match the
  same record are a load error. A record of a listed type that no entry matches is eligible and not
  examined, and counted so: it never expires.
- **`level` per entry** is `info`, `warn` or `error` and defaults to the rule's. The rule's level is a
  ceiling, as `ADR-87` item 10 requires for a declared severity: an entry cannot raise a finding above
  it. `off` is refused.
- **Nothing is dropped silently.** An unknown type, status or tier, a `days` below 1, an unknown key, an
  `anchor_field` that a listed type does not declare, or a type with no locator field is a load error. An
  entry that matches no record in this repository is reported by `config.scope-matches-nothing`.
- **The tier is the computed one.** It is the tier the record's locators compute to under the
  `embodiment` declaration, not the stated `Embodiment`, so it applies to a milestone or a bug that states
  none.
- **Which records are judged.** A record of a listed type that has the type's evidence locator field
  (`Realized-by`) and whose status matches an entry. A record whose status is in the type's
  `claim_retained` (`Superseded`, `Retired`) is not judged: its claim is history. A record stating a
  `sticky_states` value (`Deprecated`, `Inactive`) is not judged. Both match `embodiment.consistency`.
  Unlike that rule, the record need not state `Embodiment`: the claim of a `Done` milestone is its
  `Status` (`ADR-84`, `ADR-96`).
- **The unit is the record.** The rule's population counts records eligible and examined, in the report
  as every rule's is.

### 2. How old a claim is

Age is whole UTC days from the claim's anchor to today (the comparison `SPEC-3` already uses for
`Expires`). The anchor is, in order:

1. the date in `anchor_field` (`YYYY-MM-DD`) when the record carries one: the day someone re-verified
   the claim, written only by `urzua reverify` (section 4);
2. otherwise the date of the commit that last touched the record's `Realized-by` line, the measure
   `claim_max_age_days` uses today.

A malformed date is reported and falls back to the second, as `drift_audit_field` does. **A date in the
future is malformed**: dating a field forward would be the cheapest way to extend a claim. The field is
declared in each opting-in type's `known_fields`, optional, and it is the same field `drift_audit_field`
reads (`Last-audited`): one date, two readers, so a record that has been audited has both its drift
window and its expiry counted from it. A repository may name a different field for each; it is not
required to name the same.

Because a record with no date falls back to the line's last commit, adopting the rule changes no age. It
has the weakness section 6 states: until a record carries the field, a rewrite of its line still counts
as a renewal.

**The first expiry is spread (`spread_days`).** The first cycle is a cliff when many records share an
anchor: a bulk rewrite touched every `Done-means` section of the milestones on one day, and the bugs on
another. Left alone, 127 milestones would expire together on 2026-12-22, and 131 bugs on 2027-01-04
(section 10). Starting every clock at enable time would only move the cliff, and one longer first period
would move it too, since everyone would still expire on one day. So a record with **no valid
`anchor_field` date** (the fallback case above) has its period lengthened by a deterministic offset of
`0` to `spread_days` days:

`offset = (the first 8 bytes of SHA-256 of the record's Stable-Id text, as a big-endian u64) mod (spread_days + 1)`

- **Deterministic and stateless.** Nothing is written to the record and no run date is stored. Every run,
  every machine and every merge agrees. `Stable-Id` is a record's permanent identifier (`ADR-3`, `ADR-21`), so a record keeps its slot.
- **First cycle only.** The offset ends when the record carries a valid `anchor_field` date, that is after
  its first `reverify`. From then on a claim expires `days` after its anchor, with no offset. A renewal
  that follows the `max_batch` cap is spread by the reviewers' own pace.
- **It delays, never hastens.** A record's first expiry falls between `days` and `days + spread_days`
  after its anchor. A longer period for some records is the price; the finding and `urzua rules` name the
  offset, so a reviewer sees why a claim expires on that day. A record without a `Stable-Id` gets offset 0
  and is counted in the report, not skipped silently.
- **Why not seed `Last-audited` with spread dates?** That writes a date no one verified into 300 records,
  which is a renewal that skipped the command (section 6). The offset is computed, so it records nothing
  false.
- **Chosen value.** `spread_days` equal to `days` for each entry here: a first expiry spread uniformly
  over one further period. Section 10 gives the distribution. A repository that declares no
  `spread_days` gets no offset, byte for byte as before.
- **Removing the field is not a way back to an early offset.** Deleting a present `anchor_field` is
  reported by the companion rule of section 6.

### 3. The finding

An expired claim is a finding of `embodiment.claim-expired` at the entry's level, on the record. The
message names the period, the age, the anchor (`Last-audited 2026-08-01` or `line last changed
2026-08-01`), the date it expired and the record's status and computed tier. Its `ADR-73`
`suggested_action` is `run-command` with `argv` `["urzua", "reverify", "--ids", "<record path>"]`: the
review report of section 4, not a write. A test asserts, as `ADR-73` requires for `fix`, that the command
the finding offers produces a report for that record. The existing `claim-too-old` variant of
`embodiment.consistency` keeps its exemption and its reason.

### 4. `urzua reverify`

`reverify` follows `fix --rebaseline` (`SPEC-8`): review first, then apply. It writes no file in
detect mode.

- **Detect** (`urzua reverify [--ids <record>...]`, read only). For each record it lists the finding it
  answers (or, for `--ids`, any record of a listed type) and, per locator, `locator`, `resolved`,
  `pinned`, `recorded_hash`, `current_hash`, and for a scoped pin the `span` with its text, exactly as
  `--rebaseline` shows them, so the reviewer reads the content they are attesting. It lists
  `refused` records with the reason.
- **Apply** (`--apply`): the gates of `fix --apply` hold unchanged. `--ids` or `--force`, never implied;
  an `--ids` entry matching nothing fails; **`--force` is refused when it selects more than one record**
  (a batch is named, never swept up); identity resolves under the `attestation` setting
  (`ADR-89` section 5, `MILE-201`): `--by`, the identity source, and `--actor agent|human`; a lock; a
  re-read before the write; a record with no revision log is refused. `--reason <text>` is required: one
  line, without `|` or a line break, naming what was checked. A refresh with no reason is a rubber stamp.
- **What is checked before anything is written.** For each chosen record, the tool re-resolves every
  locator against the working tree and **refuses the record, writing nothing, when**:
  a locator does not resolve (file missing, a scoped pin that no longer finds its symbol or range);
  a pin's current hash differs from the recorded one (run `fix --rebaseline` first, with its own reason);
  a locator has drifted by the git-history rule, whether or not the drift is still inside
  `drift_grace_days`; `embodiment.consistency` would report the record; the record's type declares no
  `anchor_field`; or **a locator of a `require_pin` category has no pin** and was not pinned by the step
  below. Refusing on drift inside the window matters: attesting over a file that has changed
  and not been re-read is the rubber stamp this command exists to prevent.
- **The pin step: a renewal requires a pin on every `require_pin` locator.** Here that is every `code:`
  and `test:` locator. `reverify --apply` first pins each such locator that has none, then renews. The
  pin is written in the grammar of `ADR-74` and no other: the locator's own spelling, file-level or scoped
  exactly as the record wrote it, then `@sha256:` and 64 lowercase hex of the content the tool just read.
  The step **invents no scope**: narrowing a file-level locator to `path#Symbol` is a hand edit the author
  makes before running the command, and the report names every pinned file-level locator so the author
  can decide (section 10 measures why). A scope given without a pin (`ADR-74` point 8) is pinned to the
  scope and so stops being "drift judged on the whole file". The step runs under the same gates as the
  renewal: the same identity, the same required `--reason`, one row of its own, and the detect report shows
  the file or span before the hash is written. It is refused, writing nothing, for a locator that does not
  resolve or whose scope no longer finds its item.
- **A drifted unpinned locator is pinned by `reverify --pin-only`, not renewed.** The git-history rule
  can say an unpinned locator changed after the `Realized-by` line was last touched; with no pin,
  `fix --rebaseline` has nothing to move and `SPEC-8` says only a hand edit clears it. `--pin-only
  --apply` is that act made attested: it pins the unpinned locators of the chosen records, allows the
  drift, writes the `Pinned` row and **does not set the `anchor_field`**, so nothing is renewed. Its
  reason must name the real delta, as a re-pin's does (the 2026-11-05 decision), and the detect report
  lists the commits since the line was touched. `--pin-only` refuses a pinned locator whose hash differs
  (that is `fix --rebaseline`). Then `reverify` renews in a second act with its own reason. `--pin-only`
  is also the one-off migration of a corpus (section 10), because pinning is not a claim that the claim
  holds today, and so does not start a clock that the first-cycle spread (section 2) would then not apply
  to.
- **Spec locators stay unpinned.** A `spec:` locator cites another record, not code. That record has its
  own lifecycle: a status, a supersession and a revision log, and `embodiment.consistency` already judges
  the cited record. A hash of it moves on every revision row it gains, which is the needless re-pin
  `ADR-74` was written to remove, and a re-pin trains the review into a rubber stamp. A `spec:` locator is
  still re-resolved (the record must exist) and is checked by the git-history rule, and the row counts it
  apart (`S` below) so nobody reads a history check as a content check. The category list is the
  repository's: `require_pin: [code, test]` is a choice, not a rule of the engine.
- **Who may renew, and in how many at once (decided).** An agent may renew and may pin: `--actor agent`
  is accepted, as the re-pin decision of 2026-11-05 accepts it, when an independent reviewer reads the
  PR's `Pinned:` and `Re-verified:` lines and confirms each reason matches what was read. The engine
  records the actor and cannot check the reviewer's independence. **A human actor is required** where the
  matching `periods` entry's level is `error`: `--actor agent` is refused there. `--apply` names at most
  `max_batch` records (10 here); a longer `--ids` list is refused with the cap, not truncated. **Every row
  of one batch carries the same `--reason`**, so a copy-pasted sentence is visible at once, and the
  report prints the reason once above the list of records it covers. Beyond the cap the reviewer runs
  another batch with its own reason. `--force` cannot batch, above.
- **What is written.** The `anchor_field` is set to today (UTC), added to the header when absent. The
  `Status` and `Embodiment` are not touched: the computed state stays computed, so a reverify cannot lift
  a claim, only date it. The `Realized-by` line changes only to add the first pins of the pin step.
  Revision-log rows, of the type's `tool_class` (`ADR-94`), in these forms:

  `Pinned (attested by NAME, identity: SOURCE, actor: ACTOR): N locators pinned, F file-level, Q scoped. Evidence: <locator>@sha256:<12 hex>, ... Reason: <reason> **Why:** a claim is renewed only over pinned code and test evidence.`

  `Re-verified (attested by NAME, identity: SOURCE, actor: ACTOR): N locators re-resolved, P hash-checked, S spec locators checked by git history only. Evidence: <locator>@sha256:<12 hex>, ... Reason: <reason> **Why:** the claim was <age> days old against a <days>-day period.`

  A record pinned and renewed in one `reverify` gets both rows, in that order, with the one reason. The
  row names every locator with a 12-character prefix of the hash of the content the tool saw (the file,
  or the resolved span for a scoped pin), so the row records what was attested. It names how many locators
  were hash-checked and how many only history-checked, so a reader sees how much the tool could prove.
  For a repository with no `require_pin`, `S` is replaced by `H` for every unpinned locator, as the
  first version of the row had it. Both formats are pinned by a test, as the `Re-baselined` row is.
- **Interaction with `ADR-74`.** The pin grammar, the hashed bytes and the resolver contract are `ADR-74`'s
  and are not changed. The new thing is who may write a first pin: before, only a hand edit; now
  `reverify` and `reverify --pin-only`, which call the same parser and the same hash. A pin the step writes
  is moved later only by `fix --rebaseline`, exactly as any pin. `ADR-74` keeps the re-pin edit's rule that
  the scope stays in the spelling as written.
- **Interaction with the harness `/repin` command.** `/repin` runs `urzua-record.sh repin`, which runs `fix
  --rebaseline` only (`.claude/commands/repin.md`). It is unchanged: it moves stale pins and has nothing
  to do for an unpinned locator, and it must not be used to mean "pin it". The build adds a sibling,
  `/reverify`, and a `reverify` verb in `urzua-record.sh`, built the same way: it takes a record and a
  reason, passes `--actor agent`, refuses an empty reason, lists before it applies, and tells the agent
  to put every `Pinned:` and `Re-verified:` line in the PR body. The script keeps holding no schema
  knowledge.
- **Renewing early is allowed.** `--ids` may name a record that has not expired.
- **The order for a drifted, expired claim** is `fix --rebaseline` for a pinned locator, or `reverify
  --pin-only` for an unpinned one (the attestation says the pin moved or was first made, and why), then
  `reverify` (its attestation says the claim holds). They are two acts with two reasons and two rows,
  because the first says what changed and the second says someone looked. This closes the gap in which
  drift on an unpinned locator could not be cleared by `rebaseline`.

### 5. What proves a renewal

| Fact | Proved by | Strength |
|---|---|---|
| Every locator exists and every scoped pin still finds its item | The tool, at apply time | Mechanical |
| Every pinned locator's content equals its recorded hash | The tool | Mechanical, and exact for a pin |
| Every `require_pin` (`code:`, `test:`) locator is pinned, and a pin created now equals the content read | The tool: it refuses a renewal otherwise, and writes the first pin itself | Mechanical, and exact |
| A `spec:` locator exists and has not changed in git since the `Realized-by` line was last touched | The tool, by the existing heuristic | Mechanical, and only as good as the heuristic; the row counts it apart as `S` |
| The record agrees with its computed `Embodiment` | The tool (`embodiment.consistency`) | Mechanical |
| Who attested, as which identity source, and as which actor kind | The row, under `attestation` | Attribution, not cryptographic attestation (`RFC-25`); the actor is the caller's claim |
| What was looked at | The row's locators and hash prefixes | A record of what the tool saw |
| Why the claim still holds | The `--reason` | The reviewer's judgment |
| That a person read the code | Nothing | Not provable |

The tool does not claim to prove the last row. What it does is make a renewal impossible without
re-resolving the evidence, impossible over changed evidence, impossible over code or tests that are not
pinned, impossible without a named identity and a reason, and visible in the log with the content it saw.
The one weak row left is the `spec:` locator, which is judged by history, and the row says so. A false attestation is attributable and
reviewable in the pull request that carries it, as a false `Re-baselined` row is today.

### 6. A renewal that skips the command

A date in a text file can be typed. Prevention is not claimed; detection is. A second opt-in rule,
`embodiment.claim-renewal-unattested`, takes `scope: changed` (`ADR-90`; its verdict depends on the record
and its merge-base copy alone) and, on `check --base`, reports a record whose `anchor_field` is newer than
in the merge-base copy and which gained no revision row of the `Re-verified` form dated that day, or
whose `anchor_field` the merge-base copy carried and the branch removed. It is
declared by the same configuration that declares `embodiment.claim-expired` and its level is the
repository's. It cannot see a record that never carried the field, which is why section 2's fallback is a
weakness: a record in that state is renewed by any rewrite of its line. `fix --rebaseline` keeps
resetting such a record's age as a side effect, documented in `SPEC-8`; it does not move an
`anchor_field`.

### 7. Interaction with drift and the grace window (`ADR-67`)

- **Two clocks.** Drift asks whether the evidence changed; expiry asks how long since someone looked.
  They are independent and report as separate findings with separate actions (`fix --rebaseline` and
  `reverify`). Both can report on one record. Neither suppresses the other: a drifted claim is also an
  unverified one, and hiding the second would hide a fact.
- **The grace window does not delay expiry.** `drift_grace_days` holds back drift only. A claim whose
  evidence is inside the window can still expire.
- **The window and the expiry share an anchor when the field is shared.** After a `reverify`, both count
  from the new date. `reverify` refuses over drift, including drift inside the window, so renewal cannot
  be used to extend a window over an unreviewed change.
- **A re-pin is not a renewal.** `--rebaseline` attests that a pin moved and why; it does not set the
  date.

### 8. Waivers

Expiry findings are waivable like any finding (`ADR-11`). A waiver suppresses the finding and does not
renew the claim: the age keeps counting, and once the claim is renewed `waiver.matches-nothing` reports
the waiver. A suppression with no `Expires` is a permanent "this claim never expires", so it is governed
by `ADR-86`: this repository's `permanent` declaration does not list `embodiment.claim-expired`, and
declaring `permanent` closes permanence for an unlisted rule, so a waiver of the rule here must carry an
`Expires`. A repository that wants a permanent exemption lists the rule with a category and so decides it
knowingly. `sticky_states` is the declared way to exempt a whole state.

### 9. Levels and scope

`info` (`ADR-87`) and `scope: changed` (`ADR-90`) are accepted by `embodiment.claim-expired` like any
rule's option. `scope: changed` is discouraged for it: expiry exists for records nobody touches, and the
option would judge only the touched ones, which is option B. The rule's population says what was
examined.

### 10. This repository's declaration and what the first run reports

This repository declares the rule at `level: warn` (`fail_on: warning` makes an expiry fail CI), with
`anchor_field: Last-audited`, declared on `adr`, `milestone` and `bug` as it is on `spec`,
`require_pin: [code, test]`, `max_batch: 10`, and these periods, each with `spread_days` equal to `days`:

| Type and status | Days | Why |
|---|---|---|
| Milestone `Done` | 45 | Code evidence is the fastest-moving; milestones cite most of it |
| Bug `Fixed` | 60 | A regression test pins a fix; the code beside it changes |
| Spec `Shipped`, `Accepted` | 60 | A spec describes behaviour the code changes under |
| ADR `Accepted` | 90 | A decision changes slowly; its evidence is usually a spec or the code that implements it |

These are starting values, judged and not derived: the repository has 64 days of history and no
measurement can tell 45 from 60. They are revisited after the first full cycle, and the milestone figure
first: it is the one the cliff below bears on, and the first cycle is the cheapest time to learn what a
review of 128 milestones costs. `max_batch: 10` is judged the same way: a day's steady-state expiries
(about 6) fit one batch, and a week's (about 40) take four, each with a reason of its own.

**Measured on this repository** (2026-11-07, `0ad09609`), using each record's `Realized-by` line age as
the anchor, which is all any record has since none carries the field yet:

- **Expired now: 0.** The oldest claim is 60 days old and the periods are 45 to 90 for the statuses
  that matter. Milestones and bugs are not examined by today's option at all.
- **Expiring within 14 days: 6** (five bugs, `BUG-27`, `28`, `33`, `34` and `35`, on 2026-11-15, and
  `SPEC-21`); **within 30 days: 7**, adding `SPEC-5`, and **6 more ADRs on day 30 exactly**
  (`ADR-16`, `17`, `19`, `23`, `31`, `37`, 60 days old, 2026-12-07), 13 if the day is counted. These are
  today's ages, before the pin run and the spread below change the anchors.
- **For comparison, today's option:** a rule-wide value of 14 or 30 reports 14 records (12 ADRs, 2
  specs) and fails CI; 60 reports none. Shorter periods (45 days for ADRs and specs, 30 for milestones and bugs) would report 18 records
  on the first run: 12 ADRs, 5 bugs and 1 spec.
- **Steady state is a workload.** Across 339 judged records and these periods, about 6 claims a day expire
  once the corpus is old enough (128 milestones over 45 days, 158 bugs over 60, 32 ADRs over 90, 21 specs
  over 60). That is a cost the repository chooses, not a side effect; longer periods lower it and weaken
  the guarantee.

**The first cycle is a cliff, and this is the decided answer.** Measured on this repository on
2026-11-07 by `git blame` of each `Realized-by` line, with the corpus pinned the same day (below):

| First expiry | No spread | `spread_days` = `days` (chosen) |
|---|---|---|
| Milestones, 128 | 127 on 2026-12-22 (1 on 12-20) | 2026-12-22 to 2027-02-05, at most 6 a day |
| Bugs, 158 | 131 on 2027-01-04, the rest by 01-06 | 2027-01-04 to 2027-03-07, at most 8 a day |
| Specs, 21 | 21 on 2027-01-06 | 2027-01-06 to 2027-03-07, at most 2 a day |
| ADRs, 32 | 18 on one day, the rest within 4 days of 2027-02-02 | 2027-02-09 to 2027-05-03, at most 2 a day |
| All, 339 | 131 on one day; 127 on another; 307 within 60 days | 10 at most on any day; 24 in December 2026, 163 in January, 110 in February, 30 in March, 12 later |

The chosen spread keeps the busiest week near 41 expiries (about the steady state of 6 a day) where the
unspread corpus has 127 and 131 on two single days. It costs a longer first period for part of the
corpus: no claim expires before its `days`, and one expires up to `days` later. Two alternatives were
measured and not chosen. A shorter spread of half the period leaves up to 20 expiries on one day and 101
in the first 60 days. A fixed 30-day spread leaves up to 15 on one day. The ADR tail to 2027-05-03 is the
cost of spreading the slowest-moving type over its own 90-day period. The distribution moves with the
adoption date; the shape does not, because the offset does not depend on it.

**Pinning the corpus is a one-off, and it is measured.** Of the 339 judged records with a `Realized-by`
line, **164 carry at least one unpinned `code:` or `test:` locator** (105 of 128 milestones, 25 of 158
bugs, 20 of 21 specs, 14 of 32 ADRs). They hold **502 such locators**, **223 distinct**, all file-level
(none has a scope), on **212 distinct files**. The other 1048 `code:` and `test:` locators are pinned
already. 124 `spec:` locators stay unpinned by design (section 4). Run as `reverify --pin-only` in batches
of at most 10, the pin run is **17 batches** and 164 reviewed records, each with a `Pinned` row. It is
the same human review cost as 164 re-pins and it happens once; steady state adds nothing, since a locator
pinned stays pinned. Two consequences are named, not hidden:

- **The pin run restarts the fallback age** of the 164 records, because it edits their `Realized-by`
  line; the table above already counts that. It does not set `Last-audited`, so the first-cycle spread
  still applies to them.
- **File-level pins on busy files will go stale.** 106 of the 212 files had 3 or more commits in the last
  14 days. `ADR-74` was written because file-level pins on such files go stale in hours. The author
  should narrow a locator to `path#Symbol` where a symbol is the claim, before the pin run, in the same
  PR. The report lists file-level pins so the choice is visible; the step does not make it.

`MILE-129` is reopened to implement this ADR (section 11).

### 11. What changes where

`SPEC-2` (the two rules and their findings), `SPEC-3` (the options, load errors, the anchor),
`SPEC-8` (or a new spec, for `reverify`: its report, refusals and row), `SPEC-18` (`Last-audited`
on the other types), the rule table and `urzua rules`, a minor changeset, and the
repository's own `.urzua/config.yaml` and `known_fields`. The implementation is `MILE-129`'s.

## Reversibility

Moderate. The engine change is additive and off until declared: removing the rule and the command
returns today's behaviour, and a configuration that omits them is unchanged byte for byte. What is hard
to take back is on the records: once a repository's types carry `Last-audited` and its logs carry
`Re-verified` rows, removing the feature leaves dates and rows that nothing reads. They are harmless,
and `Last-audited` is already read by `drift_audit_field`. The command's report and the row format become
a contract that agents parse; a row prefix is hard to rename after rows exist (`Re-baselined` shows the
same constraint). The workload section 10 measures is reversible only by lengthening the periods.

## Consequences

- A claim that has not been looked at in its period is a finding at the level the repository chose, for
  every record with a locator, not only those that state `Embodiment`.
- Renewal is a command with a review report, gates and an attributed row. No edit to the `Realized-by`
  line counts as a renewal for a record that carries the anchor field.
- A person or an agent can renew a claim cheaply when the evidence is unchanged: one command, one reason.
  That is also the risk: the cost is low enough to batch, and the reason is a sentence. The tool refuses
  changed evidence and unpinned code or tests, caps a batch, makes every reason in a batch identical so a
  paste shows, and records what it saw; it cannot make the reason true.
- A one-off pin run over 164 records (502 locators) precedes the first renewals, and the file-level pins it
  writes on busy files will need `fix --rebaseline` sooner than a scoped pin would.
- The first expiries spread over about four months, at most 10 on a day, instead of two days that hold
  258 of 339.
- A record without the anchor field keeps today's behaviour, including that a rewrite of its line
  resets its age. Adopting the field is a per-type step and the repository's migration path.
- Drift and expiry both report on a drifted, expired record. The reviewer sees two findings and two
  actions in the right order.
- A waiver does not renew, and a permanent one is closed here by `ADR-86`.
- Three more things to maintain: a rule, a companion rule and a command, plus documents. The
  attestation row format joins `Re-baselined` as one that agents parse.
- `SPEC-8`, or a new spec, gains the command's contract with test vectors at the build.

## Decided with this ADR

The open questions of the first draft are settled in this revision, after the independent review of
2026-11-07.

- **Who may renew.** An agent may renew and may pin, with `--actor agent`, when an independent reviewer
  reads the PR's lines and confirms each reason, the rule of the re-pin decision of 2026-11-05. A human
  actor is required where the matching entry's level is `error`. The engine records the actor and cannot
  verify the reviewer.
- **A renewal of unpinned evidence.** Code and test locators must be pinned before a claim is renewed.
  `reverify` pins any unpinned one first, with a real reason, and only then renews. There is no
  "history-checked only" renewal for them. Spec locators may stay unpinned (section 4).
- **Batch renewal.** `--ids` takes up to `max_batch` records (10 here), and every row of a batch carries
  the same `--reason`. A longer list is refused; `--force` cannot batch.
- **The first cycle.** Spread by a deterministic per-record offset (section 2), measured in section 10.

## Not decided here

- **A per-type actor requirement beyond the `error` level.** For instance a human reviewer for
  milestones only. The level of the entry is the only handle now.
- **Pinning a `spec:` locator.** Left unpinned for the reason in section 4. A repository that wants it
  adds `spec` to `require_pin`, and the cost of a pin that moves on every revision row is its own.

## References

- `ADR-67`, `ADR-74` -- pins, the grace window, the claim age, scoped pins and the audit date this extends.
- `ADR-73` -- the structured `suggested_action` this uses.
- `ADR-86` -- permanent waivers by category; `ADR-11` -- waivers.
- `ADR-87` -- the `info` level; `ADR-90` -- `scope: changed`; `ADR-89` and `MILE-201` -- the attestation identity.
- `ADR-84`, `ADR-96` -- a milestone's or bug's status is derived from its evidence.
- `ADR-94` -- the revision log's class vocabulary and the tool's class.
- `ADR-64` -- structured config in, structured output out.
- `RFC-2`, `RFC-25` -- the attributed refresh and the meaning of attribution.
- `MILE-129` -- reopened to implement this decision; `MILE-181` -- re-baselining; `MILE-166` -- the audit date.
- `SPEC-2`, `SPEC-3`, `SPEC-8`, `SPEC-18`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Filed as Proposed. **Why:** maintainer direction of 2026-11-07 on `MILE-129`: improve the engine, make claim expiry configurable, and make a refresh a recorded re-verification with refreshed pins. | **substantive** |
> | 2026-11-07 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-07. | **substantive** |
| 2026-11-07 | Amended after the independent review of PR 425 (still Proposed): a renewal requires a pin on every `code:` and `test:` locator (`require_pin`), written by `reverify` or `reverify --pin-only` under the attestation rules, with spec locators left unpinned; the first expiry is spread by a deterministic `spread_days` offset from the `Stable-Id`; agent renewal, pin-before-renewal and a capped batch with one shared reason are decided, no longer open; the corpus pin run (164 records, 502 locators) and the new first-cycle distribution are measured; the 30-day count and the two-act hash refresh are worded exactly. **Why:** the review found that unpinned locators made a renewal only as strong as the git-history heuristic and that `rebaseline` could not clear their drift (blocker), and that 258 of 339 first expiries fell on two days (should fix). | **substantive** |
