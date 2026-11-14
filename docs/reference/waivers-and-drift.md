# Waivers and drift

Two ways a record and the repository around it fall out of step are handled explicitly: a finding you
have decided to live with for now (a waiver), and evidence that changed after a record claimed it
(drift).

## Waivers

A waiver is a reviewed exception to one rule for one path, written as an ordinary record so it shows
up in the same diff as everything else. It is not an ignore list: a waived finding stays in `check`'s
report with a `waived` field naming the waiver, and only stops blocking.

### Declare the waiver type

One record type declares `role: suppression`. A minimal declaration:

```yaml
record_types:
  waiver:
    dir: docs/waiver
    role: suppression
    prefix: WAIVER
    required_fields: [Rule, Scope, Reason]
    known_fields: [Expires, Stable-Id]
    effective_requires: [Reason, Expires]
    header_shape: yaml-frontmatter
rules:
  waiver.matches-nothing: warn
```

If no type declares the role, a type named `waiver` is read as the suppression type.

### Write one

```sh
urzua new waiver "Legacy ADRs lack Deciders"
```

then fill in the header:

```markdown
---
Stable-Id: 01M7A82E3M861JBW5YJPSEB776
Rule: header.required-fields
Scope: docs/adr/ADR-0002-use-queue.md
Reason: Predates the Deciders field; back-filled in the Q1 cleanup.
Expires: 2026-12-31
---
# WAIVER-1 - Legacy ADRs lack Deciders
```

| Field | Meaning |
|---|---|
| `Rule` | The rule id the waiver covers, matched exactly. |
| `Scope` | A file path, matched exactly against the finding's file, or `*` for every file the rule examines. |
| `Variant` | Optional. The finding's action variant (`waive --from-report` writes it): the waiver covers findings of that variant only. Required, or `Key-path`, when `Scope` is `.urzua/config.yaml`. |
| `Key-path` | Optional. The keys of the finding's `suggested_action.target.key_path`, such as `[record_types, milestone, dir]`: the waiver covers the finding at that configuration key only. |
| `Reason` | Why the exception is accepted. Free text. |
| `Expires` | `YYYY-MM-DD`, a real calendar date, compared in UTC. A waiver whose date has passed is treated as if it did not exist, and the finding blocks again. A waiver whose `Expires` is today's date is still in force. |

Notes that save time:

- **The waiver must be tracked.** An untracked waiver file is not read (`check` adds an info notice
  about untracked records), so a finding stays unwaived until you `git add` it.
- **A waiver scoped to the configuration file must name its finding.** `Scope: .urzua/config.yaml`
  with neither `Variant` nor `Key-path` covers every finding of the rule in the config, present and
  future, so it keeps working but `waiver.matches-nothing` reports it. List `Variant` and `Key-path` in
  the type's `known_fields` to write them.
- **A waiver that does nothing is reported.** With `waiver.matches-nothing` enabled, a waiver that has
  expired, is not in effect, or no longer matches a finding is a finding on the waiver record, naming
  the rule and scope it claimed.
- **`effective_requires: [Reason, Expires]`** makes a waiver take effect only once every listed field
  holds a real value: not blank, not a placeholder such as `TBD`, and not a value the type declares
  `pending`. With `Expires` listed, a waiver with no date takes no effect, which is what you want
  unless you declare `permanent`. `waive --from-report` refuses to run unless the type lists `Expires`
  here or declares `permanent`, because a draft given only a `Reason` would otherwise become a
  permanent waiver.
- **A waiver with no `Expires` is permanent, where the type allows it.** Declare `permanent` on the
  waiver type to say, per rule, which categories of reason may be permanent and whether each must cite
  a record; once declared, a permanent waiver for any other rule has no effect. See `SPEC-10`.
- **Some rules cannot be waived.** `urzua explain --rule <id>` shows `waivable`. A repository that
  declares `authority` with `waivable: never` cannot waive `diff.decider-required`.

### Draft waivers for an existing backlog

When you adopt urzua on a corpus that already has findings, `waive --from-report` drafts one
waiver per rule and file. Run it first as a preview, then with `--apply` to write the files:

```sh
urzua waive --from-report                          # preview: every unwaived finding at or above fail_on
urzua waive --from-report --rule header.required-fields --path docs/adr
urzua waive --from-report --apply                  # write the drafts as waiver records
git add docs/waiver                                # untracked waivers are not read
```

Each draft has `Rule` and `Scope` filled in (and `Variant`, plus `Key-path` for a configuration
finding, when the type lists them) and a pending `Reason` with no `Expires`, so it
suppresses nothing until a person writes the reason and the date. The preview lists the files it would
write with `"written": false`; `--apply` writes them (`"written": true`), each with a fresh
`Stable-Id`. Nothing is hidden by the tool: until you fill a draft in, `check` still reports the
original finding. A second `--apply` writes nothing. A finding whose rule and file already have a
waiver that is not yet filled in, or has expired, is listed under `skipped` (`already-waived` or
`expired`); one that a filled-in waiver covers is not listed at all.

## Drift

A record can claim it was built, and name the evidence. Urzua checks the claim against the evidence
and reports when they disagree.

### Declare the evidence vocabulary

```yaml
embodiment:
  drift: Drift detected      # the state a record computes to when its evidence changed
  tiers:                     # highest first; `evidence` is the locator category that lifts a record to it
    - {state: Verified,    evidence: test, claims_built: true}
    - {state: Implemented, evidence: code, claims_built: true}
    - {state: Specified,   evidence: spec}
  none: Not started          # the state with no evidence
record_types:
  adr:
    dir: docs/adr
    known_fields: [Embodiment, Realized-by]
    companion_fields:
      - [embodiment_state, embodiment_locator]
rules:
  embodiment.consistency: error
  embodiment.locator-exists: error
```

`embodiment_state` and `embodiment_locator` are the roles of the `Embodiment` and `Realized-by`
fields; if your fields are named differently, map them under `relation_fields`.

### Claim it

```yaml
Embodiment: Verified
Realized-by: "code:src/lib.rs, test:tests/db.rs"
```

A `Realized-by` value is a comma separated list of `category:path` locators. A comma separates two
locators only when a category and a colon follow it, so a path may contain commas. The strongest
category present decides the computed state: here `test:` lifts the record to `Verified`.
`embodiment.locator-exists` reports a locator whose path is not both tracked by git and present on disk.
`embodiment.consistency` reports a record whose stated `Embodiment` is not the computed one:

```text
stated Embodiment 'Not started' disagrees with 'Verified', computed from Realized-by
```

### Repair the state

```sh
urzua fix                                                    # list repairs; writes nothing
urzua fix --apply --ids docs/adr/ADR-0001-use-postgres.md --by alex --actor human
```

`fix --apply` rewrites only that field's line and appends a structural row to the record's revision
log. See [`fix`](commands.md#fix) for the identity rules.

### Pin the evidence

Without a pin, a locator drifts when git history shows its file changed since the record's
`Realized-by` line was last touched. That is a heuristic and an unrelated commit to the file can set
it off. A pin makes it exact: add `@sha256:<hash>` to a `code:` or `test:` locator and it drifts
exactly when the file's content hash differs, with git history not consulted.

```yaml
Realized-by: "code:src/lib.rs@sha256:8c72b9bc...ac8, test:tests/db.rs@sha256:76e95a3b...6823"
```

You rarely write hashes by hand. `reverify --pin-only` pins every unpinned `code:` and `test:`
locator and records who did it and why. It needs `embodiment.claim-expired` declared (next section).

```sh
urzua reverify --pin-only --apply --ids docs/adr/ADR-0001-use-postgres.md \
  --reason "pinned the evidence for the first time" --actor human
```

A locator can also carry a scope, `path#Symbol` or a line range `path#L10-L40`, to pin one item of a
file; a scope must be pinned, as in `code:src/lib.rs#Config@sha256:<hash>`. Symbols need a
`locator_resolvers` entry per language; a line range needs none. See `SPEC-3`, "Scoped evidence
pins".

### When the evidence changes

Change a pinned file and `check` fails the record:

```text
stated Embodiment 'Verified' disagrees with 'Drift detected', computed from Realized-by
```

Read the diff of the changed file, decide whether the claim still holds, and if it does re-pin:

```sh
urzua fix --rebaseline                       # lists each stale pin: recorded_hash and current_hash
urzua fix --rebaseline --apply --ids docs/adr/ADR-0001-use-postgres.md \
  --by alex --reason "added a helper function; the claim still holds"
```

`--reason` is required and is written into the record's revision log. A re-pin is an attestation
that you checked, so write what you actually checked. If the claim no longer holds, fix the record
instead.

To tolerate a short lag between a code change and its record, set `drift_grace_days` on
`embodiment.consistency`: drift is not reported until the file's last change is older than that many
days.

### Expiring claims

A claim can also go stale without any file changing. The opt-in `embodiment.claim-expired` rule
expires a claim after a period you declare per record type and status:

```yaml
rules:
  embodiment.claim-expired:
    level: warn
    anchor_field: Last-audited   # a date field in each listed type; written only by `reverify`
    require_pin: [code, test]    # categories whose locators must be pinned before a renewal
    periods:
      - {type: adr, status: [Accepted], days: 90}
```

When a claim has expired, `check` reports it and `urzua reverify` renews it:

```sh
urzua reverify                               # the records whose claim has expired
urzua reverify --apply --ids docs/adr/ADR-0001-use-postgres.md --reason "re-read the code"
```

`reverify` re-resolves and re-hashes every locator, refuses to renew over changed evidence (that is
`fix --rebaseline`'s), writes today's date into `anchor_field` and appends a `Re-verified` row with
your reason. `drift:` on the rule sets, per evidence category, whether drifted evidence blocks a
renewal (`block`, the default), is acknowledged in the row (`acknowledge`) or is ignored (`ignore`).
All dates are UTC. See `SPEC-3`, "Claim expiry".

### Drift needs history

Drift of an unpinned locator, and the age of a claim, are read from git. A shallow clone cannot
answer, so the [CI gate](ci.md) refuses one rather than reporting clean.
