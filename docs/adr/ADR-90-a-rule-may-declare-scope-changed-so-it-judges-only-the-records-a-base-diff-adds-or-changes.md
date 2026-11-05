---
Stable-Id: 01M6PYD2NGM8E41XYRCM7S22Q7
Status: Accepted
Accepted-on: 2026-11-05
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: ADR-87, ADR-78, ADR-73, ADR-64
---
# 90 — A rule may declare scope: changed so it judges only the records a --base diff adds or changes

## Context

Line references are to `origin/main` at `0b481385`. `commands/check.rs` and `diff_scope.rs` are under
`rust/crates/urzua-cli/src`; `config.rs`, `report.rs` and `rules/` are under `rust/crates/urzua-core/src`.

A rule judges every record it is handed, every run. For most rules that is right. For an advisory rule
it is not: a nudge says "while you are here, do this", and it is only useful at the moment someone is
touching the record. `MILE-189`'s rule, `embodiment.locator-on-back-pointer-path`, nudges a `Realized-by`
locator on a file under `back_pointer_paths` toward an `Implements:` or `Verifies:` comment. It is enabled
here at `level: info` (`.urzua/config.yaml:597-600`, `ADR-87`). Run over this repository at `0b481385`
it reports 398 findings on 158 records (the maintainer's decision cites about 367; the corpus has grown). PR #357 removes the 25 findings on `Fixed` records, giving 373; both counts are re-stated after it merges.
All 158 are records that are already closed: 116 `Done`, 15 `Fixed`, 14 `Shipped`, 12 `Accepted` and 1
`Retired`. Nobody is editing them, so the backlog cannot be burned down, and the findings carry no
information on a run that touches none of those records. The nudge is useful in exactly one place: a
diff that adds or changes the record.

What exists today, and why none of it is this:

- **A diff run narrows what is reported, not what is judged.** `check --base` runs every rule over the
  whole corpus and then drops findings: `narrow` keeps the records in `examine`'s set
  (`commands/check.rs:517-545`, the `retain` at `:535`) and `scope_findings` drops a finding whose file is
  outside it (`:901-917`, `rules/runner.rs:231-257`). A finding on a changed or dependent record is kept.
  The set is changed records plus *dependents* (`diff_scope.rs:376-380`, built at `:344-374`). A record is a
  dependent when it names a changed or deleted id (`:351-353`), shares an id (`:356-357`), cites a locator
  under any changed path (`:359-366`), shares a locator with a changed record (`:367-369`), or a touched
  waiver covers it (`:349-350`). The rule's population is the whole corpus either way: 494 eligible and 348
  examined on this repository, whatever the diff is. The unit is the `field` slot (a record's `Realized-by` locator), not the record.
- **Dependents defeat the purpose.** `evidence_changed` (`diff_scope.rs:359-366`) makes a record a
  dependent of any edit under a path it cites. The 158 records cite source files, so an edit to one source
  file reaches every record citing it. Narrowing the rule's findings to the diff-scoped set would still
  nudge closed records on routine source edits.
- **Some rules are already confined to the changed set, and say so differently.**
  `status.transition-allowed` makes every record eligible and examines only those with a committed copy,
  which `committed_statuses` builds for changed records alone (`diff_scope.rs:224-226`,
  `rules/status.rs:365-372`); without `--base` it reports `not-applicable` (`rules/status.rs:417`).
  `revision-log.version-bump-required` takes the changed records as its candidates
  (`rules/revision_log.rs:121-133`) and is `not-applicable` without a diff (`:115`). Both are hard-wired,
  not something a repository declares, and neither says why a record was not judged.
- **The population has a count for "not reached", with a meaning that does not fit.**
  `Outcome::OutOfScope` is "the record does not match the shape the configuration declared" and is the one
  state whose subject is the configuration (`report.rs:371-389`); `Absent` is "the slot is not there"
  and carries no reason. `config.scope-matches-nothing` reads it: it warns for any `ran` rule whose
  population has `out_of_scope() > 0` ("the rule cannot apply to them however the records are edited",
  `rules/type_scope.rs:294-320`), and a `Population` carries no cause. Reusing `out_of_scope` for a rule's
  own `scope` would put about 494 slots there on a plain `check`, warn against the locator rule and exit 1
  under `fail_on: warning`.
- **Options are per rule and refused where they do not apply.** `RuleSetting` is the one definition of the
  option set (`config.rs:188-208`); an option declared on a rule that does not list it is a load error,
  "rule 'x' does not take the option 'y'" (`config.rs:1233-1234`, enforced at `:2086-2093` through
  `rule_accepts_option`, `rules/table.rs:1640-1643`). `option_field` (`config.rs:1533-1553`) reads only
  sequence-valued options, so it never sees a scalar such as `scope`.
- **`--stdin-path` is a one-record diff.** The proposed record is made "the one changed record" against
  HEAD (`commands/check.rs:152-165`, wired at `:730-736`), which is how the two diff-aware rules above are
  judged on an editor edit (`SPEC-2`, "`stdin` scope").

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Leave the rule as it is; accept the backlog | Nothing to build | 398 findings that can never reach zero; the nudge is noise on every run that touches none of them |
| B. Waive the closed records | Uses an existing record | One broad waiver hides the finding instead of saying when it applies, and it would also silence the records the rule should nudge |
| C. Add a path or status filter to this one rule (`only_statuses`) | Small | Nothing says the problem is about the diff; a record being edited at `Done` is the case that matters; one-rule filter again for the next advisory rule |
| D. Run the nudge only under `--base` as a Scope::Diff rule | Existing mechanism | `Scope::Diff` rules judge changed *paths*, not records (`rules/table.rs:1472-1500`); the rule's logic is per-record and would be rewritten |
| E. Narrow every rule's findings to changed records on `--base` | No new option | Changes behaviour for every rule; dependents exist to catch breakage a change causes in an unchanged record (`diff_scope.rs:3-6`) |
| F. A general opt-in rule option `scope: changed` (chosen) | One declaration, any record-level rule; the population stays honest; unchanged configs unchanged | A second meaning of "scope" beside `Scope` and the report's `scope`; a rule can be configured to say nothing on a plain `check` |

## Decision

In the context of an advisory rule whose findings sit on closed records nobody edits, facing a backlog
that can never be burned down and a diff scope that includes dependents, we decided **to add a general,
opt-in rule option `scope: changed`, under which a rule judges only the records the diff adds or changes**,
to achieve a nudge that appears at the moment someone touches the record and nowhere else, accepting that
such a rule is silent on a plain `check`.

1. **The option.** `scope` is a rule option under `rules:`, in table form
   (`embodiment.locator-on-back-pointer-path: { level: info, scope: changed, ... }`). Its one value is
   `changed`; any other value is a load error naming `changed`. There is no `scope: all`: omitting the
   option is that. The option is a field of `RuleSetting` listed in the rule's `options` (`rules/table.rs`)
   (`config.rs:188-208`). Because `scope` is a scalar and `option_field` handles only sequence options
   (`config.rs:1533-1553`), `rule_accepts_option` (`:2086`) would never see it and a `scope` on any rule
   would be silently accepted. The build therefore adds a dedicated validation of `scope`: the value must
   be `changed`, and a rule that does not list `scope` in its `options` is a load error. A test covers it.
2. **What it does under `check --base`.** The rule's candidates are all still eligible; the rule judges
   (reaches a verdict on, and may report a finding for) only the records whose path is in the diff's changed
   set: added, modified, or the new name of a rename, working-tree edits included
   (`GitChanges.changed`, `rust/crates/urzua-io/src/lib.rs:133-144`; `diff_scope.rs:316-320`). A candidate that is not changed
   is eligible, not examined, and counted in the new `deferred_by_scope` counter (point 4). An unchanged
   candidate that is `Absent` anyway is counted as `Absent` first: the slot is not there, so there is
   nothing to defer. This is decided inside the rule's census, where
   the population is built, not by dropping findings afterward; the afterward-drop at
   `commands/check.rs:901-917` cannot make the population say what was not judged.
3. **What it does on a plain `check`.** No diff exists (`changed_paths` is `None`, `rules/runner.rs:77-78`,
   `commands/check.rs:769`), so the rule judges none of its candidates: all are eligible, none examined,
   none reported. The rule's execution is `ran`, not `not-applicable`: it ran and found its scope empty.
   That differs on purpose from the two diff-aware rules above, which have nothing to run without a base.
4. **A disclosed reason, in the population.** The unjudged candidates are counted in a new, separate,
   additive `Population` counter, `deferred_by_scope`. This is an additive output-contract change to
   `SPEC-2`: a consumer that does not know the field ignores it. `out_of_scope` keeps its meaning, a
   configuration that does not reach a record, and the rule `config.scope-matches-nothing` does not read
   `deferred_by_scope`, so a plain `check` raises no warning. The unit is the `field` slot, not the record.
   The report carries one `info` notice per such rule: the rule id, `scope: changed`, the number of
   slots not judged, and why ("no `--base`" or "unchanged since <base>"). `Outcome::OutOfScope`'s
   documentation is unchanged; the new counter has its own.
5. **Dependents do not count as changed.** Only records the diff touches are judged. `diff_scope.rs`'s
   dependents exist so a defect a change causes in an *unchanged* record is reported (a stranded reference,
   a half of a collision; `diff_scope.rs:3-6`); a nudge is not a defect the change caused. Counting
   dependents would re-create the backlog on every source edit (`:359-366`). A dependent is still
   *examined* by the rules that do not declare the option, exactly as today.
6. **Composition with diff-scoping.** The option's set (changed records) is a subset of the diff scope's
   (changed plus dependents), so the two intersect and the second step is the existing one: the rule judges
   changed records only, then `scope_findings` keeps the findings on examined records
   (`commands/check.rs:901-917`), which a changed record always is. A path argument narrows further, as
   for every rule (`:517-545`). When the configuration file itself changed, every record is in the diff
   scope (`diff_scope.rs:299-314`), but *changed* still means the record's own path: a config edit does
   not make a record changed. Declaring the option on a rule that already judges only the changed set
   (`status.transition-allowed`, `revision-log.version-bump-required`) is a load error, because it would
   do nothing (`config.rs:202-204`: a silently-ignored option is a check that never fires); the rule
   table marks such rules.
7. **Which rules may take it.** A record-level rule: `Scope::Records` (`rules/table.rs:5-17`) whose
   findings are anchored to a record (`Anchor::Record` or `Anchor::RecordOrDeclaredFile`,
   `:116-136`), less the two above. Of these, only a rule whose verdict on a record depends on that
   record alone may take it. A cross-record rule (id collision, reference resolution and similar) anchors
   a finding on a record other than the one it judged, so deferring the unchanged records would hide the
   defect; such a rule does not list `scope`. In the rule table, `Scope::Records` has 39 rules anchored to
   a record, 1 to `RecordOrDeclaredFile` and 1 to a directory; the build marks each rule per-record or
   cross-record, adds a table-consistency test that `scope` appears in `options` only for per-record
   rules, and cross-checks the `options` list against `rule_accepts_option`. A config-level rule (`Scope::Config`), a file-level one
   (`Scope::Files`), a `Scope::Diff` rule and a `Scope::Run` rule cannot: the declaration is refused at load
   with the existing error, "rule 'x' does not take the option 'scope'" (`config.rs:1233-1234`). The
   Directory-anchored `filename.identity-pattern` cannot either: its finding is on a directory, not a
   record. A rule's `options` list decides, so a future rule opts in by listing `scope`.
8. **A waiver on such a rule is not "matches nothing".** `waiver.matches-nothing` judges the unmatched case
   only on a whole-corpus run in which the target rule ran (`rules/waiver.rs:16-22`, `:118-121`). Under
   `scope: changed` a plain whole-corpus run is a run where the rule did not look at most records, so the
   absent finding proves nothing. A waiver whose rule declares the option is eligible and unexamined for
   that case, as under `--base` today; the check reads the rule's `scope` setting. The build tests it.
9. **`ADR-73` is unchanged.** A finding the rule does emit carries the `suggested_action` or exemption
   its variant declares, exactly as without the option, and the coverage test is unaffected. A record that
   is out of scope yields no finding and therefore no action. The option adds no kind, no `target` key and
   no variant.
10. **The editor hook treats the edited record as changed.** `check --stdin-path` already makes the proposed
    record the one changed record against HEAD (`commands/check.rs:152-165`, `:730-736`), a record `HEAD`
    lacks being added, so the rule judges it with no special case. The rule is still run, not skipped, under
    `record_only` because its findings are anchored to a record (`rules/runner.rs:185`). A `--stdin-path` run
    is the only plain-looking run where the rule fires without `--base`, which is the intent: the edit is the
    moment to nudge.
11. **Not declared, nothing changes.** A rule without the option runs as it does today: same candidates,
    population, findings and notices, byte for byte.
12. **This repository sets it** on `embodiment.locator-on-back-pointer-path` (`MILE-189`, at `level: info`).
    `MILE-203` builds the option and does that.

## Reversibility

Moderate. The option is additive: a configuration that omits it is unchanged, so the engine change can be
removed without touching a report. Once a repository declares it, removing the option brings back every
finding the rule suppresses, which for this repository is the 398-finding backlog at `info`. The shape that
is hard to reverse is the wire: the additive `deferred_by_scope` counter and a notice per rule become part
of what consumers read. A consumer ignores a field it does not know, so adding it is cheap, but removing it
later is not. `out_of_scope` is untouched. `scope` accepts one value now: a second value is a new decision.

## Consequences

- The nudge reaches an author at the moment of the edit: on `check --base` in CI, and in the editor through
  `--stdin-path`. Closed records stop generating findings nobody can act on.
- A plain `check` is silent on this rule by design. A repository that wants the full backlog visible runs
  without the option or reads the population: `eligible` 494 slots, `examined` 0, with the notice saying why. The
  backlog is disclosed, not hidden ("you get what you declare", `ADR-53`).
- `check <path>` on one closed record, with no `--base`, reports nothing from such a rule. An agent that
  wants the nudge for a record uses `--base` or `--stdin-path`.
- `out_of_scope` keeps one cause (a configuration that does not reach a record) and
  `config.scope-matches-nothing` keeps working on it. A rule's own `scope` is counted in
  `deferred_by_scope`, which that rule does not read. `SPEC-2` gains the field.
- A rule with the option cannot be used as a gate over the whole corpus. That is the trade, and why the
  option is opt-in and per rule, never a default.
- A touched-but-unchanged record is never nudged: a dependent whose evidence moved gets no nudge until it
  is itself edited. Accepted: it is the case that would make the backlog return.
- Untracked files are not in the changed set (`rust/crates/urzua-io/src/lib.rs:146-151`), so a new
  record is nudged once it is tracked, as every `--base` rule is.
- `SPEC-2` (the option, the population, the notice, `waiver.matches-nothing`) and `SPEC-3` (the option and
  which rules take it) change in `MILE-203`, with the rule table and `urzua rules` output.

## References

- `ADR-87` -- the `info` level at which this repository enables the rule.
- `ADR-78` -- item 4, the basis of the rule this option applies to.
- `ADR-73` -- suggested actions, unchanged.
- `ADR-64` -- structured config in, structured output out; no string mini-language.
- `ADR-53` -- a repository gets what it declares; a rule that read nothing says so.
- `MILE-189` -- the advisory rule; `MILE-123` -- the diff scope; `MILE-191` -- `--stdin-path`.
- `MILE-203` -- builds this decision.
- `SPEC-2`, `SPEC-3` -- the check output contract and the configuration.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** Maintainer decision, 2026-11-05, recorded here; the maintainer accepts it in a later change. | **substantive** |
> | 2026-11-05 | Amended while Proposed after independent review: a separate additive `deferred_by_scope` counter replaces reuse of `out_of_scope` (which `config.scope-matches-nothing` would have warned on); dedicated scalar validation for `scope`; units are `field` slots (494 eligible); cross-record rules named; ADR-78 added. **Why:** Review of PR #360 found that reuse would warn on every plain `check`. Status stays Proposed. | **substantive** |
> | 2026-11-05 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-05. | **substantive** |
