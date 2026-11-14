# Changelog

All notable changes to `urzua` are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/). Written for the person installing it — what changed
for you — not a commit dump.

## 0.4.2 (2026-11-14)

### Fixes

- 0.4.1's macOS binaries could not be built because the CI runner image it used was retired, so no binaries were attached. 0.4.2 publishes the same changes with Linux and Apple Silicon macOS binaries; install 0.4.2. Intel Macs build from source with `cargo install`.

## 0.4.1 (2026-11-14)

### Fixes

- The 0.4.0 tag was created but its GitHub Release and binaries were not published, because the release notes exceeded GitHub's size limit. 0.4.1 publishes the same changes, so install 0.4.1 rather than 0.4.0. See the 0.4.0 entry in CHANGELOG.md for everything that changed since 0.3.0.

## 0.4.0 (2026-11-14)

### Breaking Changes

#### **Breaking: header field names are compared exactly against a type's declared fields.** They were

previously compared case-insensitively, so a record writing `status` matched a config declaring
`Status`. A corpus relying on that now gets a finding, and the finding names the declared spelling:
`field 'status' is not declared for record type 'adr' -- the type declares 'Status', differing only in
case`.

Fixes BUG-97, where the comparison was ASCII-only and reported a declared non-ASCII field name as
undeclared with no spelling an adopter could use to fix it. ADR-57 records why the comparison was
removed rather than corrected: every correct case-insensitive comparison makes `Maße` and `Masse` one
field, and accepting an undeclared name is a missing finding.

#### **Breaking: `FixReport.records_examined` and `MigrateSchemaReport.records_examined` are replaced by

`population`.** Both were the bare, hand-maintained `usize` the rest of the report contract moved away
from earlier in this release. `fix`'s population reports the `Embodiment`/`Realized-by` pair as a
declared slot (`Absent` when either is missing, matching `embodiment.consistency`'s own pair-slot
handling); `migrate schema --report`'s population reports a record whose header did not parse as
`Unreadable`.

Fixes a real false diagnosis: `migrate schema --report` classified a record with an unparsed header
as the candidate field being `Blank`, telling the operator the field would fail there specifically —
when the true defect is that nothing in the header is readable at all. Such a record is now excluded
from `would_fail` and disclosed instead as a `Notice`.

#### **Breaking: `rules_executed` entries no longer carry `records_examined` or `scope`.** Every rule now

reports a `population` instead — what it was eligible to examine and what it judged, in a named unit
(`record`, `field`, `record-type`, `path`, `claim`). The old number held three different denominators
under one name: `field.quality` reported 1041 over a 315-record corpus, correct all along and
labelled as something it was not. A consumer reading `records_examined` should read
`population.examined` and check `population.unit` before comparing it to a record count.

The report also gains a top-level `records_read_by_any_rule`: how many distinct records some rule
actually reached a verdict about, as a union rather than a sum across units. `files_examined` says
what was read off disk; this says what was judged.

`check` and `audit` no longer report `not-run` on the grounds that no record-scoped rule ran. What
gets checked is what the repository declares, so a thin configuration earns a clean exit — and says
`records_read_by_any_rule: 0` where a reader sees it, rather than reading as a checked corpus.

#### Governance is configuration. Every rule is declared and opt-in, and the configuration is YAML.

**Breaking: the config is now `.urzua/config.yaml` at `schema_version: 2`.** A `0.3.0` config does
not load. Every rule a repository wants must be named in the `rules` table -- an absent rule is off,
not on -- and `urzua init` writes a starter table for an adopted corpus. A rule taking options
(`pointer.target-status`, `claim.status-agreement`, `narrative-field.stale`) fails to load without
them rather than running inert. Unknown rule names and misplaced options are load-time errors.

**A rule's severity is declared, not compiled in.** `level: warn` or `level: error` per rule, so a
repository decides what blocks its own CI.

**`dir` means that directory, not that subtree** (`RFC-35`). A type's records sit directly in its
`dir`; nested directories belong to no type, and the new `type.record-outside-declared-dir` reports a
record-shaped file that falls between types rather than dropping it silently.

New rules, all opt-in: `type.dir-matches-nothing`, `type.record-outside-declared-dir`,
`identity.collision`, `embodiment.locator-exists`, `field.pending`, `claim.status-agreement`, and
`pointer.target-status` split out of `pointer.resolution` so a dangling reference and a reference to
a superseded record can carry different severities.

**The engine no longer knows this repository's vocabulary.** `narrative-field.stale` reads a declared
`terminal_statuses` instead of a table keyed on record-type names, so the rule applies to a corpus
whose types are named anything at all.

**`urzua new` finds a record's number rather than assuming its position**, so a hyphenated type
prefix parses and the command stops reissuing the same number.

**`urzua init` proposes one type per directory holding records**, writes the type prefix that
directory's filenames actually use, and declines to propose rules the corpus gives it no way to
satisfy.

**A run that establishes nothing is reported as `not-run`, not `ok`.** A configuration with no rules
declared, or only rules that read the configuration or the path inventory, no longer exits 0 as
though the corpus had been checked.

Discovery reads git's output NUL-separated, so a record whose filename contains a non-ASCII
character is part of the corpus rather than silently dropped.

**A record the tool cannot read stops the run.** An unreadable or missing tracked record previously
dropped out of the corpus, leaving a smaller `files_examined` and a clean verdict. A deletion staged
in git is still a legitimate absence.

**A path argument narrows what is reported on, not what a reference resolves against.** `check
docs/adr/` no longer reports every reference that points outside the requested path as dangling, and
no longer discards findings about files git does not track.

`claim_paths` is read to any depth, follows symlinks that stay inside the repository, bounds the walk
against cycles, and fails the run rather than skipping a claim it could not read.

Fixes BUG-38, BUG-39, BUG-42, BUG-43, BUG-44, BUG-45, BUG-46, BUG-48, BUG-49, BUG-53, BUG-54, BUG-55, BUG-56, BUG-58, BUG-59, BUG-60, BUG-61, BUG-62, BUG-63, BUG-64, BUG-67, BUG-68, BUG-69, BUG-70, BUG-71, BUG-72, BUG-75, BUG-76, BUG-77, BUG-78, BUG-79, BUG-80, BUG-81, BUG-82, BUG-83, BUG-84, BUG-85 and BUG-86.

#### **Breaking: the `blockquote` and `bold-list` header shapes are removed.**

The `blockquote` (`> Key: value`) and `bold-list` (`- **Key:** value`) header shapes no longer have a reader. YAML
frontmatter is the one header shape with a reader (alongside `none` for a type with no header), and
`header_shape` now defaults to `yaml-frontmatter` when omitted.

A config that declares `header_shape: blockquote` or `bold-list` (or the same under `fields.from`)
fails to load with an error naming the removed shape. A record still written in either form is
reported by `header.required-fields` as unreadable, with the form named and the fix stated, rather
than as a record with no header; `urzua init` counts such records in a notice, and `urzua migrate ids`
names the form when it skips one. `urzua new` always writes YAML frontmatter.

The `header.layout-consistency` and `header.deprecated-shape` rules and the `header_layout` option are
removed with the shapes they judged; naming either rule under `rules:` is now an unknown-rule error.
Rewrite an un-migrated corpus's headers as YAML frontmatter before upgrading.

#### Fixes two opposite-direction defects in how a declared field's value is read when its exact-cased key

is missing (`ADR-57`): `claim.status-agreement` and `pointer.target-status` used to fabricate a
sentinel that turned every claim or pointer citing a mis-cased `Status` field into a false blocking
finding; the embodiment rules used to silently drop a mis-cased `Realized-by`/`Embodiment`, hiding a
real inconsistency. Both now skip the field (unjudged, not judged-and-wrong), and a new rule,
`header.field-case-mismatch`, is the one place that reports a declared field written under a different
case (`ADR-58`). Also fixes a duplicate record-number bug in `urzua new` (`0013-01-15-slug.md` used to
parse as a date and skip record 13, which could then be reissued), consolidates three previously
independently-computed "record-shaped file" definitions into one, and consolidates the `(record,
field)` candidate-slot construction duplicated across five rules into two shared helpers.

Implements `RFC-39`/`ADR-59`: `FieldName` and `RecordId` are now real types whose `Eq`/`Hash` encode
their comparison rule (exact for field names per `ADR-57`, numeric-normalized for record identifiers
per `BUG-2`), replacing bare `String` at the six field-name comparison sites and the shared record
index. `RecordTypeConfig`'s own fields stay `Vec<String>` — the type boundary sits at
`declared_fields()` and the header's comparison methods, not at config deserialization, since that is
where every comparison bug in this family actually lived. The remaining ten rule functions that took a
bare `HashMap` projection of configuration now take `&Config` directly, matching the fifteen that
already did. Two enums that predate `ADR-53`'s move of status vocabulary to the adopter
(`urzua_core::Status`, `urzua_core::Embodiment`) are removed; neither was ever constructed.

Implements `ADR-50`, previously Accepted and unbuilt: `header_shape: "none"` declares a type whose
records carry no header block at all. `header.required-fields` skips such a type instead of reporting
every record as missing a header region; a new rule, `config.header-none-has-no-required-fields`,
reports the self-contradiction of declaring one anyway; `header.deprecated-shape` is now an explicit
enumeration of the two actually-deprecated shapes instead of a negative test that would have flagged
`none` on sight; `urzua new` refuses to generate a record for a `none`-shaped type rather than writing
one with no identity. Fixes `BUG-98`.

`init` now warns when its proposed config leaves `urzua audit`'s entire rule set undeclared — the
ordinary outcome for a corpus with no filename type prefix, since both of `audit`'s rules are
identity-dependent. `audit`'s own exit-code behavior is unchanged (a run in which nothing ran still
reports `not-run`); this only tells the adopter why before they hit it. Fixes the disclosure half of
`BUG-99`; the root cause (`audit` owning a hardcoded rule subset) stays tracked by `RFC-38`.

**Breaking: `claim.status-agreement`, `pointer.target-status`, and `narrative-field.stale` now only
read a target record's `Status` field when that record's own type declares `Status` in
`required_fields` or `known_fields`** — the same rule `header.field-case-mismatch` already applies to
every other field. A type that never declares `Status` previously had it checked anyway, hardcoded,
which meant a mis-cased `status:` on such a type went both unenforced and undiagnosed; it also means a
type relying on the old unconditional check now needs to declare `Status` explicitly to keep these
three rules' coverage. Fixes `BUG-109`, decided by `ADR-60`.

`read_claim_files` now reads through `urzua_io::read_to_string`, the same chokepoint every other record
read already uses, instead of calling `std::fs::read_to_string` directly.

Three independent, hand-maintained lists describing which rules take which config options (whether an
option is declared on the wrong rule, whether a rule requires options to load at all, and what a
missing required option's consequence is) collapse into one table in `urzua-core`, so a future
option-taking rule is one entry instead of three edits that could disagree. `init`'s notion of which
rules depend on filename identity moves from `urzua-cli` into the same crate as the rules themselves.

No adopter-facing behavior changes beyond what's described above: verified with the full test suite,
clippy, `make ci`, and a real-corpus `check` run reporting the same findings before and after each
phase of this work landed.

#### **Breaking: each `population` in `rules_executed` gains `out_of_scope`.** It counts candidates the

*configuration* cannot reach, as distinct from ones the corpus has not written yet — two states that
previously arrived as one number despite having opposite fixes. Always present, including as `0`.

Adds `config.scope-matches-nothing` (MILE-106), an opt-in rule that reports a declared rule whose
candidates the configuration does not reach. On a corpus whose filenames carry no type prefix,
`identity.collision` is reported and `revision-log.change-class-required` is not, though both examined
zero of two candidates: the first is a configuration that cannot match, the second is a corpus that
has not written a revision log yet.

### Features

- Add an opt-in `ignore_marker` to `embodiment.back-pointer-resolution` and `embodiment.back-pointer-decision-accepted`, the same option `boundary.published-path-reference` already takes. Declare the text once and a line carrying it, such as a record-shaped line inside a multi-line string in a test file, is no longer read as a back-pointer: it is not reported as citing a missing record, not judged for the cited record's status, and not counted as evidence. A real citation on any other line of the file still counts. The count of excused lines is shown in the rule's population (`excused_lines`) and in an `info` notice, and a marker on a line with no back-pointer is reported. The marker is shared by both rules and the evidence `fix` and `reverify` read (two different values are a load error), excuses the whole line, and is refused at load when it is empty or part of `Implements:` or `Verifies:`; `fix` and `reverify` report the excused count too. Configurations that declare no marker are unchanged.
- `urzua check --stdin-path <path>` reads the proposed content of one record from stdin and judges it against every enabled rule that can judge one record without writing, comparing a status change or a new revision-log row with the committed copy of the record, reporting on that record alone (`scope.source` is `stdin`). A rule that cannot report on a record is listed `not-applicable`, and the hook's message names those rules instead of claiming every rule ran. The editor hook now relays it: an unwaived error the edit introduced blocks, a warning is shown, an error already on the file is listed and does not block, and the hook says plainly when it did not judge.
- Adds a `verb_claims` option to `claim.status-agreement`. Set to `false`, it stops closing verbs in prose from being read as claims, so a change's `closes` field is its only claim; `check` states that the reading is off instead of counting verb claims. It is refused when `closes` is not declared, with either value, and on any other rule. Without `strict`, a notice says that a fragment lacking the key carries no claim. Left unset, the rule behaves as before.
- `urzua harness install` now writes a drafting command for every record type the configuration declares (for example `/bug`, `/condition`, `/waiver` and `/spec` beside `/adr`), generated from `.urzua/config.yaml` with only the type name and directory filled in, and three lifecycle commands: `/amend` adds a dated amendment section and a revision-log row to a decided record and refuses any change to existing lines; `/repin` runs `urzua fix --rebaseline --actor agent` only with a non-empty reason and prints the line for the pull request body; `/done` records finished work by running `urzua fix --apply`, so the state written is the one the engine computes from the evidence and the command never writes a state itself. Adding a record type to the configuration and running `urzua harness update` adds its command, and `urzua harness status` reports a command not yet installed as missing. A record type named like a lifecycle command is refused at install. `urzua fix --apply` no longer writes a state that an `authority` gate names; it reports the refusal.
- Add `date.field-order`, an opt-in rule that checks one date field of a record is not later than another. List entries as `date_order: [{earlier: Date, later: Accepted-on}]`; a record whose first date is later than its second is an error, equal dates pass, and a value that is absent, a placeholder or not a real date is left to `date.field-valid` and the field-quality rules.
- Add `date.field-valid`, an opt-in rule for date-valued header fields. List the fields in `date_fields`; a value that is not a well-formed `YYYY-MM-DD` date, or names a day that does not exist such as `2026-02-30`, is an error, and the report states how many values were examined. Requiring an acceptance date on accepted-type statuses and forbidding it elsewhere is declared on the record type with `statuses.require` and `statuses.forbid` (see the configuration spec).
- A record type can now declare its status vocabulary in `.urzua/config.yaml` under `statuses`: the allowed `values`, which are terminal `success` or `failure`, the fields a status `require`s or `forbid`s, and the type's own `pending` values. Three new opt-in rules report against it: `status.undeclared-value`, `status.required-field-missing` (for example `Deciders` on an `Accepted` record) and `status.forbidden-field-present`. `field.pending` uses a declared `pending` list, matched as whole values, instead of treating any value containing the word as pending; `narrative-field.stale` and `claim.status-agreement` use a declared type's terminal statuses in place of their hand-written lists. A type that declares nothing behaves as before.
- The path options of the diff rules now accept globs as well as prefixes, and `diff.source-requires-record` accepts `exclude_paths`. A set such as every source file except tests is written `source_paths: ["src"]` with `exclude_paths: ["**/*.test.*", "src/tests"]`; `*` stays inside one path component, `**` crosses them, and an entry with no glob character is a prefix exactly as before. A diff that touches only excluded files is not a trigger, and the report keeps those paths in the population as unexamined. A glob that cannot compile is refused at load.
- `urzua doctor` gains the check `template-sections`: for each record type whose configuration requires sections, it reports each required section the type's template (`.urzua/templates/<type>.md`) has no heading for, as a warning that names them, and reports clean when the template has them all. A type with no template, or none requiring sections, has nothing to compare.
- Add `/rfc`, `/adr` and `/milestone` drafting commands for AI agents working in this repository. Each scaffolds a record with `urzua new`, stages it, regenerates the declared indexes with `urzua index --apply`, runs `urzua check` on it and reports how many records were examined; none sets a decision status or changes a value `urzua new` wrote. `AGENTS.md` asks an agent to offer the matching command when it notices a proposal or a made decision.
- `embodiment.consistency` accepts `drift_audit_field`, the name of a record field holding the date its evidence was last audited. With `drift_grace_days` set, a record carrying a `YYYY-MM-DD` date in that field has its grace window counted from that date instead of from the cited file's last change; a record without it counts from the file as before, and a malformed date is reported as a warning. The option needs `drift_grace_days` and a field that a record type declares, else the config is refused at load.
- Adds `drift_baseline` to `embodiment.consistency`, a git ref that separates drift that predates it from drift introduced since: `drift_baseline: {ref: v1.0.0}`. A record that already had the same drifted locators (same pin) at the ref, whose cited files were last changed at or before it, is pre-existing, so it draws no finding and does not block, and it is listed in an `embodiment-drift-baselined` info notice and a new `baseline` object in the `check` report. Drift introduced since the ref is reported and blocks as before. A ref that names no commit stops the run with exit 2 rather than reading as no pre-existing drift, and a whole-repository run whose baseline holds nothing out says so, so it can be removed. Without the option nothing changes.
- `citation.resolution` reports the ids a record declares in its `non_reference_field` as a population of their own: a second `rules_executed` entry for the rule with the new unit `field-entry`, counting each declared entry, so a run over no declarations is told apart from a clean one. The uncited-declaration and malformed-entry findings are counted there and are not read as a misconfiguration.
- Add the opt-in `filename.identity-pattern` rule. A file directly in a record type's directory whose name does not match the type's declared `identity.pattern` (or the `TYPE-NNNN-slug.md` convention when none is declared) is now reported instead of silently dropping out of every identity-reading rule. When every file of a type is off-pattern, as in a bare-number corpus (`0001-slug.md`), `check` and `audit` report it once for the type with the expected file name for each file, so the corpus can be renamed. `urzua init` proposes the rule at `warn`.
- Add `urzua harness`, which installs an agent harness bundle into a repository and runs its guardrails. `harness install --adapter NAME` writes the adapter's hook entries, a translator for the agent and a lock file, merges the entries into the agent's project settings without touching anything else in them, and keeps one marked block of instructions in the file `harness.instructions_file` names; `--dry-run` lists what would change. `harness status` compares the lock with the files and exits 1 on drift, `harness update` rewrites what has not been changed by hand, `harness uninstall` removes only what it installed, and `harness adapters` lists what is embedded in the binary. The new `harness` configuration key turns each guardrail on by name, and `on_unjudged: deny` makes a guardrail that cannot judge refuse an agent's edit while a person's goes through. `harness run edit` judges a proposed edit to a record, with the verdicts the editor hook had, and says so loudly when it cannot judge; `harness run session-start` reports at the start of a session what would make a guardrail fail open. `urzua doctor` reports the harness when it is declared.
- Adds `block_on: introduced | all`, a top-level configuration key that sets which errors the editor hook blocks on. `introduced`, the default and what a configuration without the key gets, blocks only on an error the edit introduces, as before. `all` blocks on any error in the edited record, including one it already has, and skips the second `check` run. The named override and its log work under both values. Any other value is a configuration load error that names the key and the value.
- `embodiment.locator-exists` takes two options, `locator_kinds` and `pinnable`, each a list of `file`, `directory` and `glob` that defaults to `[file]`. A repository that opts in may cite a directory or glob in `Realized-by`: it is legal when it covers at least one tracked file, its drift uses the newest commit among those files, a pin on it hashes the sorted list of each file's path and content hash, and `urzua explain` and `urzua fix --rebaseline` show the files a pin covers. A locator of a kind the repository has not declared is reported, naming the option.
- The opt-in rule `embodiment.locator-on-back-pointer-path` warns on a `Realized-by` locator that names a file the back-pointer scan already reads, where an `Implements:` or `Verifies:` comment would say the same thing and is found automatically. It flags a `code:` locator under `back_pointer_paths` and a `test:` locator under `test_paths`, leaves what a comment cannot say (a `spec:` locator, a `#scope` or an `@sha256` pin), and offers no edit action, since the comment has to exist before the locator can go.
- `diff.source-requires-record` can now be a decision gate. Declare `decision_types` (each record type that holds decisions, with the statuses at which one counts as decided), and optionally `fix_types` (record types, such as `bug`, whose records exempt a change) and `link_fields` (pointer fields, such as a milestone's `Implements`, through which a record links a decision). A diff that changes declared source paths must then also change a decision at a decided status, a record linking one, or a fix record; otherwise each changed source path is a finding, with an `add-record` suggested action, and a changed decision still at an undecided status is named in the message. The rule is unchanged unless `decision_types` is declared, and a declaration that could never apply (an unknown type, a status the type does not list, `fix_types` without `decision_types`) is refused at load.
- `header.field-set-consistency` takes the optional `extension_prefixes` option, a map from a record type to the prefix its extension header keys begin with, in place of the fixed `x-`, so a type can admit `ext-owner` while an unprefixed unknown key still fails. A type the map does not name keeps `allow_extension_prefix` exactly as before. A prefix that is empty, holds whitespace or starts a field the type declares is refused at load, as is a type that is not declared or declares no `known_fields`, and the option is off unless declared.
- The editor hook records a named override in a committed log. A top-level `override_log` declares the file, a Markdown table of date, name, record, field, rule, reason and a blank `Reviewed-by` that a person fills in. The new opt-in rule `override.unreviewed` warns on every entry whose `Reviewed-by` does not name a person (a blank cell, a placeholder such as `TBD` or `-`, and a declared pending value are not a review), so an override surfaces in the pull request that carries it, and reports a row that is not an entry and a declared log that does not exist. With a log declared the hook needs a reason (`--reason`, `URZUA_HOOK_REASON`) and refuses an override it cannot record.
- Adds `pending` to `statuses.delivery.items`, an optional list of item texts that mean a Done means item is not yet written, in the shape of `statuses.pending` (a whole value or `{pattern: "..."}`). An item that matches is an unfilled placeholder: it never resolves, cannot be waived, and `status.done-items-shape` reports it on its line, so a milestone scaffolded from a template no longer passes `urzua check` with its placeholder items unflagged. A configuration without `pending` behaves as before.
- Adds an optional `fields` option to `pointer.target-status`: a list of declared pointer/narrative field names the rule checks. A dependency field such as `Implements` can now be judged separately from a lineage field such as `Derives-from`, whose Superseded target is permanent history rather than a stale reference. Omitting `fields` checks every declared field, as before. Fields left out are not counted in the rule's population, and a `fields` entry that names no declared pointer or narrative field is a config error rather than a silently ignored setting.
- Add `urzua harness run post-command`, which runs after any shell command an agent runs and checks the records git shows as changed, untracked ones included, in one `check` call, then reports the unwaived findings back to the agent. It never reads the command and never blocks: a record that already failed at `HEAD` is listed as pre-existing, a record whose content has already been reported is not checked again, a deleted record is not judged, and a missing `git`, an unreadable configuration or a `check` that cannot run is stated as not judged. Turn it on with `post-command` under `harness.guardrails`; the Claude Code adapter wires it as a hook on its Bash tool.
- Add `provisional` to a `relation_pairs` entry: `provisional: {statuses: [Draft, Proposed], level: warn}`. `relation.pair-reciprocity` then reports a claim from a record at one of those statuses that its peer does not echo at the provisional level (`warn` by default), while a claim from any other status keeps the rule's own level, so it becomes an error once the claimant is accepted. Off unless written; a status that no record type declares is refused at load.
- Add `types` to a `relation_pairs` entry: `types: [rfc]` binds the pair to the record types that carry it. `relation.pair-reciprocity` then treats a claim to a type outside the list as eligible and unexamined, counted in the report without a `config.scope-matches-nothing` warning, instead of an unechoed claim from a type that cannot hold the inverse field. A pair without `types` stays global. An unknown or duplicated type, an empty list, or a type declaring neither of the pair's fields is refused at load.
- A symbol resolver can now address a struct field. The `member` of a `locator_resolvers` entry may be a list of entries, each with its own `start` pattern and `end` kind, and a name is looked for with all of them, counted together (a field and a method of one name are reported as ambiguous). Declare a method entry (`brace_balanced`) and a field entry (`indent`), let the `container` match a `struct` as well as an `impl`, and `Type::field` pins one field, so a new field elsewhere in the type no longer stales the pin. A single `member` mapping is read as before.
- Adds `roadmap.work_types`, an optional list of the record types that are work. A record of any other type, such as a fact that is met or dropped and never built, still blocks the records that depend on it but no longer appears in `urzua roadmap`'s waves, ready set, critical path, records or groups. Waves and the critical path count work only and follow dependencies through those records, so work behind one is ordered after the work it waits on. A new top-level `waiting_on` list names each unfinished non-work record with its type, status and the work it blocks. Without the setting the output is unchanged. An unknown, repeated or blank type name, or an empty list, is refused when the configuration loads.
- A rule can declare `scope: changed` so it judges only the records a `--base` diff adds or changes, and none on a plain `check`. It is meant for advisory rules, whose findings are useful at the moment someone touches a record and nowhere else: `check --stdin-path` treats the proposed record as changed, a record that merely depends on a changed file is not nudged, and the report says what was left unjudged. A rule's population gains a `deferred_by_scope` counter, present only when it is non-zero, and the report carries one `info` notice for each such rule naming how many slots were not judged and why; `config.scope-matches-nothing` does not warn about them, and `waiver.matches-nothing` leaves a waiver on such a rule alone. `scope` takes one value, `changed`; any other value, or a rule that does not take the option, fails to load. `urzua rules` now prints each rule's `options` and whether it can take `scope` (`judges`). `embodiment.locator-on-back-pointer-path` takes the option.
- `sequence.cycle` reads the pairs `roadmap.pairs` names as one graph, so `urzua check` reports a loop that uses an edge of each of two declared sequencing pairs, on every record in the loop, with the cycle's path and the pairs it crosses. This is the same graph `urzua roadmap` reads, so the two now agree. A new finding variant, `sequence-cycle-cross-pair`, marks it. A loop inside one pair is reported as before, an `acyclic` pair that `roadmap.pairs` does not name is still checked on its own, and a repository with no `roadmap.pairs` is unchanged.
- `statuses.require` now treats the values in a type's declared `statuses.pending` list (for example `Unknown`, `none recorded` or a dash) as missing, so an accepted record whose required field holds a placeholder is reported by `status.required-field-missing`. The list is the same one `field.pending` reads and is declared once; a type that declares none behaves as before.
- `test_paths` of `embodiment.back-pointer-resolution` accepts globs as well as prefixes, so test files that sit beside the code they test and are named by suffix can be declared with `test_paths: ["**/*.test.*"]`. An entry with no glob character is a prefix exactly as before; `*` stays inside one path component and `**` crosses them. A glob is read among the files under `back_pointer_paths`, a glob matching none of them is reported, and one that cannot compile is refused at load.
- `embodiment.consistency` now reports a record that states `Verified` or `Implemented` while having no `Realized-by` field and no source or test back-pointer citing it: a claim with no evidence of either kind is reported against the computed `Not started` instead of passing silently. A record that states any other value with no evidence is still not judged and is counted as unexamined.
- Adds `urzua roadmap`, which computes the order of work from the records alone: waves, the ready set, the critical path, what each record blocks and a grouping, as JSON. It reads one graph over the sequencing pairs a new `roadmap.pairs` configuration key names, so a loop that crosses two pairs is reported as a cycle. A record that waits on a finished-unsuccessfully record, or on a cycle, is listed under `needs_replan` with a reason instead of appearing ready. A repository that declares no pair gets a note saying there is no roadmap rather than an empty one. `roadmap.group_by` names the header fields to group by, and `export --format json` carries the same `roadmap` object when `roadmap.pairs` is declared.
- Add `waiver.matches-nothing`, an opt-in rule that reports a waiver that does nothing: one that is expired, one naming a rule this build does not have, and one whose rule ran over the whole corpus and found nothing in its scope. Each finding is on the waiver record and names the rule and scope it claimed. A path-scoped or `--base` run, or a waiver whose rule is off, does not judge a live waiver, so a partial run produces no false stale report.

#### Fixes `BUG-24`: an explicitly-named `check <file>` path that git doesn't track was silently dropped

(`"status": "not-run", "files_examined": 0"`, no findings, no notice) instead of being read, exactly
as SPEC-2's Discovery contract already said it should be ("explicit paths on argv override discovery
and are used as given"). A directly-named file now overrides the tracked-only filter; a directory
argument still respects the tracked-only sweep unchanged (`ADR-6`'s "never a raw directory walk") --
only naming one file precisely bypasses it. Additive to the tracked sweep, never a replacement: the
corpus a pointer resolves against still doesn't shrink just because a path was requested (`BUG-60`).

New `ScopeSource`/`DiscoverySource::Argv` report which mode actually ran, distinct from
`tracked-sweep`.

Also adds `docs/specs/SPEC-22-rust-coding-standards.md` (Draft), surveying and backlinking this
codebase's already-in-force Rust conventions (domain-value newtypes, the declared-population pattern,
shared candidate-selection helpers, `#![forbid(unsafe_code)]`, the pure-core/impure-io split, and
more) to where each was decided or landed -- none of it was written down anywhere before.

No adopter-facing behavior change for a `check` invocation naming only tracked files or directories:
verified with the full test suite, clippy, `make ci`, and a real-corpus `check` run reporting the
same 70 findings before and after.

#### Adds `attestation`, a top-level configuration key that sets where `urzua fix --apply` and `urzua fix --rebaseline --apply` take the identity they attest under. `identity: gh-verified`, the default and what a configuration without the key gets, lets a verified `gh` login win over `--by`, as before, and the report now names the `--by` it replaced in an `identity` field. With `require_verified: true` the command exits 2 with `not-run` and writes nothing when `gh` cannot verify a login. `identity: asserted` takes `--by` as given and records the verified login beside it; combining it with `require_verified` is refused. An unknown `identity` value is a load error naming the allowed ones.

Adds `--actor agent|human` to `urzua fix`. Every tool-authored revision row now records the identity source (`gh-verified`, `asserted` or `git-config`) and the actor kind (`agent`, `human`, or `unspecified` when the flag is absent), for example `Re-baselined (attested by NAME, identity: gh-verified, actor: agent)`. The actor kind is the caller's claim, not proof. Existing rows are untouched.

#### `urzua check --rules <id>[,<id>...]` runs only the named rules of your configuration, and every report gains a top-level `rules_selected` (the ids, or `null` when not restricted). A rule left out is omitted from `rules_executed` and is not run; naming a rule your configuration does not declare, or one that does not exist, exits 2.

`urzua audit` is deprecated: it is now an alias of `urzua check` over your configured rules with no rule set of its own, so it reports exactly what `check` reports (`urzua --help` and `urzua audit --help` now say so) and adds one deprecation notice that never changes the exit code. Because it now runs every rule your configuration declares, a CI step running `urzua audit` fails on a blocking finding from any of them, not only supersession reciprocity and dangling references, and takes about as long as `check`. It no longer exits 2 on a configuration `check` accepts, such as the one `urzua init` writes for a corpus with no type prefix. Use `urzua check` (or `urzua check --rules pointer.resolution,relation.supersession-reciprocity` for the narrower report); `audit` is removed in the next release. `urzua init` no longer warns that `audit` has nothing declared.

#### A source comment of the form `Implements: ADR-12` now counts as realization evidence for the record

it cites. Declare the new opt-in rule `embodiment.back-pointer-resolution` with a `back_pointer_paths`
list of path prefixes; `urzua check` and `urzua fix` then compute the cited record's `Embodiment`
as at least `Implemented`, and the rule reports a back-pointer that cites a record that does not
exist. Only whole-line comments are read, and a comment never outranks a `test:` locator or drift.

#### New opt-in rule `embodiment.back-pointer-decision-accepted` reports a source `Implements: ADR-12`

comment whose cited record is not yet in an accepted status, so code cannot claim to implement a
decision that has not been made. Accepted means the cited type's declared `statuses.success`; a
type without one uses the rule's optional `accepted_statuses`, and with neither the status is
reported as unjudgeable. It takes `back_pointer_paths` like `embodiment.back-pointer-resolution`,
and on a `--base` run judges only the source files the diff changed.

#### A suppression type can declare `effective_requires`, the fields a waiver must carry before it takes effect, for

example `effective_requires: [Reason]`. A waiver missing one, or holding a blank, placeholder or pending value, no
longer suppresses its finding; the finding is reported again, and `waiver.matches-nothing` reports the waiver as
`waiver-ineffective`. Without the declaration nothing changes.

#### New opt-in rule `field.leading-reserved-indicator`: flags any declared field whose value starts with a

reserved YAML indicator character (`@`, `*`, `&`, `!`, `%`, `|`, `>`), which needs quoting to parse at
all. `BUG-18` found this happening to every hand-typed `Author`/`Deciders` value in this corpus (a
purely decorative `@`, forcing avoidable quoting) via a one-off backfill with no guard against it
recurring -- which it did, twice, the same day as the backfill itself. This rule generalizes the check
past those two fields to any declared field, so the next instance is caught by `check` rather than found
by hand again.

#### `field.quality` takes the optional `template_placeholders` option. Declared `true`, a required field whose value is

identical (after trimming) to its record type's template text for that field, from `.urzua/templates/<type>.md`, is
reported as a placeholder, so a record left with `Found-in: how/where this was actually discovered` no longer passes.
The compiled-in placeholder tokens do not apply while it is declared, and a type with no template is judged by its
declared `pending` vocabulary alone. The option is off unless declared, and declaring it with no template for any
record type is reported as a notice.

#### Add the `revision-log.version-bump-required` rule and the `revision_log.version_bump` declaration. A record type declares

a version field and the revision-log change classes that require it to change; on a `--base` run, a changed record that
adds a row of one of those classes while its version equals the merge base's copy is reported. The merge base's copy of
each changed record is read by the CLI. No type has it by default, and enabling the rule with no type declaring it is a
configuration error.

#### Fixes `BUG-52`: `SPEC-2`'s rule table claimed to be "the complete, current set" while nothing checked

that against `ALL_RULES`, and it had drifted twice already. The table is now generated, not
hand-maintained — `urzua_core::rules::RULE_METADATA` (checked against `ALL_RULES` by a compiled test)
is the single source, rendered into `SPEC-2` by `scripts/generate-rule-table.py` between marker
comments. `make rule-table-check` (wired into `make ci`) fails the build if the committed table is
stale.

New: `urzua rules` — lists the complete, current rule set this build ships, with each rule's
description, reading no config or corpus. What the binary supports, not what any one repository
enabled.

No adopter-facing behavior change to `check`/`audit`/etc.: verified with the full test suite, clippy,
`make ci`, and a real-corpus `check` run reporting the same 70 findings before and after.

#### A record's `Status` can follow any evidence field its type declares. `statuses.delivery` already held a

type's `Status` to its `Realized-by` locators; it now applies to any record type, and takes an optional
`back_pointer_evidence` key (default `true`) so a type can say that only its declared evidence field
counts, not a source back-pointer that cites the record. `unevidenced_tiers` may be an empty list for a
type whose done records legitimately have no evidence. A record that is deliberately open with evidence
is waived with a `waiver` record scoped to its path, with a reason.

#### Adds `urzua check --base <ref>`: report findings, and `files_examined`, for the records that changed

since the merge base of `<ref>` and `HEAD` (staged and unstaged edits included) and for the records a
change reaches: any record that names a changed or deleted record, or cites changed or deleted
`Realized-by` evidence. Intended for a pull-request gate over a large corpus, where deleting a target
still reports the record that points at it. References resolve against the whole tracked corpus, so an
unchanged target is never reported missing, and a change to the configuration file examines every
record.

The report says what was asked for: `scope.source` is `git-diff`, `scope.base` names the ref
(`null` on every other run), and `scope.changed_paths`, `scope.dependents` and
`scope.whole_corpus_reason` account for the examined set. A diff with no changed record reports `ok`
with `files_examined: 0` and a notice, an unknown ref or one with no merge base exits 2, and `--base`
combined with a named file git does not track is refused rather than silently skipping the file.

#### `urzua check` now reports an unknown key in the `harness` configuration (a misspelled guardrail

setting such as `enabeld`, or an unknown top-level `harness` key) as a warning notice with a structured
`detail` holding the `key`, its `nearest` valid key and the `valid` keys, instead of staying silent. It
never moves the exit code. A freshly scaffolded milestone no longer fails `status.done-items-shape`
before it is filled, and `urzua --help` now carries the one-line description the README and `RFC-27`
state.

#### `citation.resolution` accepts an optional `external_ids` list of prefixed ids that are not records of

the repository, so a public standard number that shares a record prefix (`RFC-4122` against an `RFC`
prefix) is no longer reported as a dangling citation. Each entry is an exact id or an inclusive
numeric range (`RFC-4000..4999`) under a declared prefix; a malformed entry or one under no declared
prefix is a config error. A matching citation is counted as out of scope in the rule's population, and
any other dangling id is still reported.

#### Add the opt-in `citation.issue-exists` rule and the `urzua refs refresh` command. An issue or pull

request cited in a declared reference section, as `#123` or as a link to the declared repository,
must be in a snapshot file committed to the repository; `issue_refs` names the snapshot, the
repository and the citation forms to read (`number`, `url`). `check` only reads the file and never
uses the network. If the snapshot is absent, unreadable or describes another repository the rule
reports that instead of passing, and every finding and an info notice name the date the snapshot
was generated. `urzua refs refresh` rewrites the snapshot from the GitHub REST API: it reads a
token from `GITHUB_TOKEN` or `GH_TOKEN` when one is set, pages through the listing, writes sorted
output, and on any failure (no network, a rate limit, an unknown repository) exits 2 and leaves the
existing file untouched.

#### Add the opt-in `citation.path-exists` rule. A path cited in backticks inside a declared reference

section (`reference_sections`, for example `["References"]`) must exist in the working tree: a file
git tracks and that is on disk, or a directory with a tracked file below it. Only plain paths are
read (a directory component and a file extension, or a trailing `/`); URLs, globs, placeholders and
bare file names are not. A trailing `:line` or `#anchor` is ignored and `./` or `../` resolves from
the record's directory. The check is offline and deterministic, and its population counts the paths
cited.

#### `citation.resolution` accepts the optional `skip_revision_log` flag that `citation.path-exists`

already has: when `true`, each record's revision log (the table under its type's declared
`revision_log` identity) is not read, so append-only history that cites an id as it was is not
reported as a dangling citation. It is `false` by default, so behaviour is unchanged unless it is
declared.

#### A release fragment can now declare the records it closes in a one-line frontmatter key, and

`claim.status-agreement` checks that data instead of reading a sentence. Declare the key's name with
the new `closes` option (`closes: closes`) and write `closes: [BUG-60, BUG-61]`, a single id, or
`closes: []` for a change that closes nothing. Each listed id must name a record and that record must
be at a terminal status its own type declares; an open record, an id that names no record
(`claim.id-unresolved`) and a value that is not a one-line list of ids (`claim.field-malformed`, which
includes a block list and a list over several lines) are errors. The optional `strict: true` also
asks every fragment that has frontmatter for the key (`claim.field-missing`), which catches a
fragment that names a record without saying whether it closes it. A file with no frontmatter is not a
fragment: it is counted out of scope and named in a notice. A reference in the text is never read as
a claim by the new option, and the existing closing-verb reading keeps working unchanged, with a
notice that says how many claims it still found. Declaring neither option changes nothing.

#### Add per-type claim expiry and `urzua reverify`. The opt-in rule `embodiment.claim-expired` expires

a claim after a period declared per record type, status and evidence tier (`periods`), at its own
level, counting from a recorded audit date (`anchor_field`) or the commit that last touched the
`Realized-by` line; `spread_days` lengthens the first expiry by a deterministic offset from the
record's `Stable-Id` so a corpus rewritten on one day does not expire on one day. The companion
rule `embodiment.claim-renewal-unattested` reports an audit date that moved with no `Re-verified`
revision row. `urzua reverify` is the one attributed way to renew a claim: it re-resolves and
re-hashes every locator, refuses changed evidence, pins `code:` and `test:` locators that have no
pin (`--pin-only` pins them and renews nothing), caps a batch at `max_batch` records under one
shared reason, and records who renewed the claim and why. A configuration that declares neither
rule loads and reports as before. The harness bundle gains a `/reverify` command and a `reverify` verb in its helper script,
which refuse an empty reason, list the review and print the `Pinned:` and `Re-verified:` lines for the pull request body.

#### Add `code.dangling-bug-reference`, an opt-in rule that reports a source comment citing a record id

that resolves to no record, the mirror of `embodiment.locator-exists`. It reads the tracked files
under its `back_pointer_paths` that have one of its declared `extensions`, takes the record prefixes
from the record types, judges existence only, leaves a back-pointer that
`embodiment.back-pointer-resolution` already judges to that rule, and takes `external_ids` and
`ignore_marker` as `citation.resolution` and the boundary rule do.

#### A fact that nobody builds can now be a dependency. This repository declares a `condition` record type

(`COND-NNNN-slug.md`, statuses `Open`, `Met` and `Dropped`, with `Met-by` required once `Met`) entirely in
`.urzua/config.yaml`, and a `Depends-on` from a milestone or any other record that declares the field
resolves to one like any other record. `urzua roadmap` and the derived `blocked` value read it: a
dependent of an `Open` condition is blocked and out of the ready set, a dependent of a `Met` one is
not, and a dependent of a `Dropped` one stays out. Any repository can declare the same shape under its
own type name; the engine has no special case for the name. `SPEC-23` documents the record.

#### Introduce a support window for the config `schema_version`, and `urzua migrate config` to bring a config from the previous version up to date.

Each release reads the current `schema_version` and the one before it. A config at the previous
version is upgraded in memory before any rule runs, so `check` keeps working through an upgrade; the
only difference is one new warning, `config.schema-version-outdated`, on the config file, naming the
version found and offering `urzua migrate config` as its suggested action. Under `fail_on: warning`
that warning fails a strict repository until the file is migrated.

`urzua migrate config` rewrites a config at the previous version to the current one, in place, keeping
its comments. It lists every key it added, changed, renamed, moved or dropped, makes no change on a
second run, and leaves a file it cannot make valid exactly as it was. A version-1 record type that
omits `header_shape` is refused with an error naming the type: version 1 read it as `blockquote`,
which is no longer read, and the upgrade will not quietly change it to `yaml-frontmatter`.

A config outside that window is no longer a bare "unrecognized `schema_version`" error. The run exits
2 with a `schema_version_error` object (`found`, `supported`, `current`, `migration`) beside the usual
`error` sentence, where `migration` names the oldest release that still reads the version and the
command to run with it. `SPEC-3` gains a "Supported schema versions" table and the rule for when
`schema_version` must change.

#### Add `authority.decider_table` (`base`, `head` or `both`), which says whose table `diff.decider-required`

reads. With `base`, who may decide (the groups, their logins and the gates) comes from the configuration at
the pull request's base revision, so a pull request can no longer add its author to a decider group, or
delete a gate, in the same change that moves a record to a gated status. `head`, which an undeclared key
means, keeps reading the checkout's table. `urzua init` now proposes `decider_table: base`, and `urzua
doctor` warns when gates are declared without the key.
Under `base` and `both`, a base with no `authority` is judged against the head table instead of passing
unjudged, and the rule's level and `fail_on` are the stricter of the base's and the head's, so a pull request
cannot switch the rule off or lower it. `urzua doctor` says an `authority` with no gates gates nothing.

#### Adds declared boundaries: an opt-in rule, `boundary.published-path-reference`, that reads the paths your repository publishes, vendors or generates, and enforces what follows from each kind. A boundary is a set of path prefixes or globs with a `kind` (`published`, `vendored` or `generated`), a required `reason` and a `severity`:

```yaml
rules:
  boundary.published-path-reference:
    level: error
    ignore_marker: "record-id-ok"
    boundaries:
      - paths: ["packages/*/src"]
        kind: published
        reason: "Published to the registry; readers cannot resolve our record ids."
        severity: error
        exclude: ["packages/*/src/fixtures"]
```

- A record identifier cited inside a `published` boundary is reported, in prose as well as in an `Implements:` marker, whether or not it resolves to a record.
- A back-pointer (`Implements:` or `Verifies:`) inside a `generated` boundary is reported; a plain record identifier there is not.
- A `vendored` boundary is declared and prohibits nothing.
- A boundary's `severity` (`error` or `warn`) is the level of its findings; the rule's `level` only turns the rule on.
- A boundary's `exclude` globs remove paths from its check, and the `ignore_marker` text you declare excuses the one line it is on. An `exclude` entry, a boundary path or a marker that matches or suppresses nothing is itself reported, as is a file that is not text.
- A back-pointer inside a `published` boundary never counts as realization evidence. The same holds for a file under `published_paths`, so a repository that declared one will see its Embodiment drop where records were realized only by back-pointers in those files.
- `published_paths`, a list of path prefixes or globs, is read as one published boundary with no reason required.

Overlapping published boundaries report a citation once, at the highest severity. Every finding carries a `suggested_action` where an edit to the config can fix it; an uncovered path offers a `set` with no value, for you to supply the corrected path. The rule's `population` is counted in a `file` unit.

#### A record type can now declare how its records are identified and read. `identity` takes a

`pattern` over the file name, with the type prefix and the record number as `(?P<prefix>...)` and
`(?P<number>...)` groups; the prefix also declares that titles open `PREFIX-N`. A pattern without a
prefix group fails the load, and a file name without the type prefix gets no identity. `fields.from` chooses how header
fields are read: `yaml-frontmatter`, `blockquote`, `bold-list`, `none`, or the new `prefix-lines`
for bare `Key: value` lines before the first `##` heading. A record type that declares neither
loads exactly as before, and a pattern that does not compile fails the load.

#### A record type can declare its lifecycle: `statuses.initial` names where a new record may start and

`statuses.transitions` maps each status to the statuses that may follow it. The new opt-in rule
`status.transition-allowed` compares a changed record's `Status` at the merge base with the change
under check (`urzua check --base`) and reports an edge the graph does not list, or a new record that
starts outside `initial`. `status.unreachable-state` reports a declared status that no path from
`initial` reaches. The `embodiment` declaration must now mark at least one tier `claims_built`.

#### `blocked` is a derived value. A record type's `statuses` can declare `progress`, the statuses that

claim work has advanced (a type lists `Done` there explicitly), and a top-level `roadmap.pairs`
names the sequencing pairs that count as dependencies, so a containment pair declared `acyclic`
blocks nothing. The new opt-in rule `sequence.status-agreement` reports a blocked record that states
a `progress` status. A record is blocked when any record it depends on is not in a status its own
type declares `success`, which includes a dependency in a `failure` status, with no status, or of a
type that declares no `success`. A type can still declare `statuses.blocked`, the written statuses
that mean waiting, to have a written one checked both ways. A blocked value that cannot be computed
(a dependency that resolves to nothing, or a record whose header cannot be read) is counted
unreadable and not guessed. Existing configs are unchanged.

#### Two opt-in rules judge the paths a `check --base <ref>` run changed. `diff.source-requires-record`

reports each changed path under a declared `source_paths` prefix when no governed record changed in the
same diff. `diff.source-and-records-mixed` warns when one diff changes both a governed record and a path
under `application_paths` (less `exclude_paths`). On a run with no `--base` either rule reports
`not-applicable` in `rules_executed` instead of passing.

#### A record type can declare `statuses.delivery.items`, so that its Done means is a list whose items each

name their own evidence, and its `Status` is computed from the items instead of from one record-level
locator. An item ends in an evidence clause in the `Realized-by` grammar, or in an inline waiver with a
declared category, a cite and a reason. A record with every item resolved computes delivered, one with
some resolved computes partial, and `urzua fix --apply` writes the computed status (`InProgress` for
partial work, with the item counts in the revision row) and never downgrades a finished record.

A finished record with an unresolved item is a new error of `status.delivery-agreement`,
`DoneWithUnresolvedItems`. It names each unresolved item and its cause, fails `urzua check` under every
`fail_on` and every rule level, and cannot be cleared by a `WAIVER` record; only evidence or an item
waiver clears it. Two new rules, `status.done-items-shape` and `status.waived-item-stale`, hold the item
grammar and the waivers; item locators are checked by `embodiment.locator-exists`, read by drift and
re-pinned by `urzua fix --rebaseline`. `urzua roadmap` reports `items` counts for such a record, and
`urzua rules` prints a variant's `severity_floor` and `waivable`. A configuration that declares no
`items` is unchanged.

#### New opt-in rule `embodiment.requires-accepted-status`: a record whose `Status` is not an accepted status

cannot state an `Embodiment` of `Implemented` or `Verified`. Accepted statuses are the type's declared
`statuses.success`; a type that declares none can list them in the rule's `accepted_statuses` option, and a
type with neither is not examined. `Specified`, `Not started` and `Inactive` are not flagged, and the rule's
census says how many records it judged.

#### The `Embodiment` states are now declared in config. A top-level `embodiment` key names the drift

state, the tiers from highest to lowest with the kind of evidence (`test`, `code` or `spec`) that
lifts a record to each, and the state computed from no evidence; `claims_built` marks the states
that say the work was built. `urzua check` and `urzua fix` compute and repair against those names.
Omitted, the built-in `Drift detected`, `Verified`, `Implemented`, `Specified` and `Not started`
apply, so existing output is unchanged.

#### Add a global `--as-of YYYY-MM-DD` option that fixes the date every date comparison reads: waiver

`Expires`, claim age (including ages taken from git history), status expiry and the audit grace.
`urzua check --as-of <date>` previews which waivers and claims will have lapsed by then, and a verdict
can be reproduced on another day. The reports of `check`, `audit`, `fix` and `reverify` carry a
top-level `as_of` object, `{"date": "YYYY-MM-DD", "source": "flag" | "clock"}`, naming the date used and
where it came from. Without the option the date is the machine's UTC date, as before. A malformed date
exits 2 with a structured error. The option is refused with `--apply`, where a revision row would
carry a false date, and by commands that compare no dates.

#### `embodiment.consistency` gains three opt-in refinements of drift, all off by default. A `Realized-by`

locator written `code:path@sha256:<hash>` is pinned to that content hash and drifts only when the file's
hash differs. The `drift_grace_days` rule option holds back drift whose cited file changed fewer than
that many days ago. The `claim_max_age_days` option warns when a record's `Realized-by` line was last
touched longer ago than that. A repository that sets none of them sees no change.

#### Add a per-type `exclude_files` declaration to `.urzua/config.yaml`: bare file names directly in a

type's `dir` that are not its records, such as an index `README.md`. A named file is never loaded as
a record, so no rule examines it and `filename.identity-pattern` no longer suggests renaming it or
counts it among the type's files; any other stray file is still reported. An entry that is a path, a
glob or empty is refused when the configuration loads. Findings about the configuration, such as
`config.scope-matches-nothing`, now name `.urzua/config.yaml` relative to the repository root
instead of an absolute path, so a report compares across checkouts.

#### `urzua explain --rule <rule-id>` prints what one rule is, as JSON: its scope, the level this

repository sets for it, its severity floor and whether a waiver can clear it, its options with their
current values, the suggested actions its findings offer, its population unit and the spec that
documents it. A bare argument to `explain` is still always a path, so a rule id and a path are never
confused. An unknown rule id is an error that suggests the nearest valid id. `urzua rules` also prints
each rule's `scope`, `population_unit` and `spec`.

#### `urzua export --format json` now emits each record's sections. For a record type that declares

`sections`, every record carries `sections` (each with `name`, `heading`, `line`, `end_line` and
`content`, an alias resolving to the declared name) and `preamble`, the text before the first
section. A type that declares no `sections` gets `null` for both, so "no declared document model"
is distinct from "declared, none present" (`[]`).

#### Add a `fail_on` setting (`error` or `warning`, default `error`) to `.urzua/config.yaml`, and a

matching `urzua check --fail-on` flag that overrides it. With `fail_on: warning`, `urzua check` and
`urzua audit` exit 1 on any unwaived warning, so a repository can make warnings fail its CI. The JSON
report is identical either way; only the exit code changes.

#### Set severity per field within one rule. `field.quality`, `field.pending` and

`status.required-field-missing` take an optional `field_levels` map from a field name to `warn` or
`error`, so a missing `Deciders` on an accepted record can block while a missing `Reviewers` only
warns. A field the map does not name keeps the rule's `level`, and a config without the option
behaves as before.

#### `urzua check` findings can now carry a `suggested_action`: what to do about the finding, as data an

agent can act on without reading `message`. It names a `kind` (`edit-field`, `edit-section`,
`edit-config`, `add-record` or `run-command`), an `op` where the kind has several forms (`set`,
`remove`, `append`, `rename`, `add`), a `target` (the file, field, section, configuration key path or
command) and, only where the tool can derive one, a `value`. A key that was not computed is omitted,
never `null`. A finding with no sensible action, such as an identifier collision, has no
`suggested_action` key. A `run-command` action is offered only when running that command would
remove the finding.

`urzua rules` lists, for every rule, each finding variant with its action or the reason it offers
none. A consumer should treat an unrecognised `kind` or `op` as no guidance and read `message`.

#### `urzua fix` takes record paths: `urzua fix docs/adr/ADR-1-x.md` examines, computes drift for and

repairs only that record, and a directory selects the records under it. Drift and `git blame` work
only for the selected records, so a single-record run is several times faster than a whole-corpus
run. A path that matches no record is reported as a failure naming it instead of being skipped, and
`--ids` and `--force` choose among the scoped repairs only.

#### Add `urzua fix --rebaseline`, an attributed re-baseline for `@sha256:` evidence pins. Without

`--apply` it lists each record whose pinned locator changed, with the recorded and current hash.
With `--apply`, `--ids` (or `--force`), `--by` and a required `--reason` it rewrites those records'
pins to the current hash and appends a revision-log row naming who attested and why; nothing else
on the `Realized-by` line moves.

#### `urzua fix --stdin-path <path>` reads the proposed content of one record from stdin and judges it

without writing, for an editor hook that wants a verdict before the file is saved. The path need not
exist yet, and drift is computed for that record only, so the run takes a fraction of a whole-corpus
`urzua fix`. Each repair now also carries `line`, the 1-based line of the stated field, so a hook can
point at it without parsing the record.

#### `urzua fix` now detects and, with `--apply`, writes `Embodiment: Drift detected` for a record whose

`Realized-by` locator changed after the `Realized-by` line was last touched -- the same condition
`urzua check` already reports. Previously `fix` ignored drift, so a drifted record that `check`
flagged could not be repaired by the tool. A stated `Drift detected` that is no longer drifted is
repaired back to the tier-computed value.

#### Add `urzuahq/urzua/.github/actions/urzua-gate`, a reusable GitHub Action that installs a pinned

release, verifies its build provenance, checks the checkout has full history, runs `urzua check`
and, on a pull request, `urzua check --base`, and reports findings as inline annotations and one
pull request comment that is updated in place. `urzua doctor` recognises a workflow that uses it
as wiring the checker into CI.

#### `diff.decider-required` now treats deleting a record that held a gated status (for example an Accepted ADR) as a

gated move that needs a decider, and the post-command check reports such a deletion; a rename is not a deletion.
Gate statuses are compared case-insensitively, so `accepted` or `ACCEPTED` can no longer slip past a gate on
`Accepted`.
Under `decider_table: base` or `both`, a record cannot be taken out of the gate's reach by excluding it or moving its
type's directory in the same pull request that deletes it.

#### Add `authority.waivable` (`never`, `base` or `any`) to decide whether a waiver record can silence

`diff.decider-required`. Under `base` only a waiver the base revision already holds counts, and under
`never` none does, so a pull request cannot accept a record by adding a waiver in the same change.
`urzua init` proposes `never` and `urzua doctor` warns when gates are declared and the key is not.
`doctor` also warns when no gate lists `.urzua/config.yaml`. Under `decider_table: base` or `both`, the
rule's `require_evidence` option is now the stricter of the base's and the pull request's, and a plain
local `urzua check` no longer prints the "no --base" decider-table notice.

#### `check` now tells you when `drift_grace_days` is holding drift back. One info notice,

`embodiment-drift-in-grace`, reports how many records `embodiment.consistency` will report once the window ends (records whose type declares no `Embodiment`, a sticky state such as `Deprecated`, or a stated `Drift detected` are not counted), the
date the earliest of them leaves the window and starts being reported, and their record paths (at most
20, with the full count). Dates follow `--as-of`. The notice never changes a finding or the exit code,
and a configuration without `drift_grace_days` reports exactly what it did before.

#### The agent harness checks the translator before it runs it. Every hook command `urzua harness install` and

`urzua harness update` write embeds the sha256 of `.claude/hooks/urzua-claude-code.py`, reads the file once
and runs it only if the hash matches; on a mismatch an edit is refused where `acceptance` or
`protected-paths` is on with `on_unjudged: deny`, and the other hooks say they did not run. An edit is
judged against the session's own repository and every governed repository that holds the file, inner or
outer, and a deny from any of them wins; a `.git` counts only when git accepts it, so an empty `.git` and a
configuration written above the project no longer replace the project's gates. An MCP tool named for reading
is judged when the call names an output or carries content, and an MCP path with surrounding whitespace, a
NUL, dot segments or an over-long run of them is read as the path it names.

#### Add `harness.outside_writes` (`allow`, `shared` or `deny`) and `harness.allow_paths`. With `shared`, an

agent's edit-tool write into a linked worktree of the repository, for example one at an older commit that
holds no gates, is also judged by the repository's own acceptance and protected-path gates, and a deny
wins. With `deny`, an agent's write outside the repository is refused unless it is under an `allow_paths`
directory. A key left undeclared behaves as `allow`, as before. `urzua init` proposes
`outside_writes: shared`, and `urzua doctor` warns when a guardrail is enabled and the key is undeclared.

#### `urzua doctor` reports whether the `pre-commit` and `pre-push` git hooks are wired. Each layer

appears as a `hook-wired` check with its `layer` and a `state`: `not-installed`,
`installed-running-urzua`, `installed-not-running-urzua` or `installed-not-executable`. It follows
`core.hooksPath` when one is set.

#### An index `format` for `index.freshness` and `urzua index` can use placeholders for declared header

fields, such as `{Status}` and `{Date}`, beside `{id}`, `{file}` and `{title}`. A field placeholder is
filled from the record and checked against the committed index; a placeholder that names no declared
field of the type fails the configuration load.

#### New opt-in rule `index.freshness`: a committed index of one record type, either a whole file or a list

between `begin`/`end` marker lines in a README, must list exactly the records. Declare each artifact under
the rule's `indexes` option with its `path`, record `type` and entry `format` (`{id}`, `{file}` and
`{title}` placeholders). `check` reports a record with no entry, an entry naming no record, a duplicated
entry, a stale file name or title, a missing file and a removed marker pair, and the rule's census says how
many artifacts it compared.

#### Index checking reads more shapes, and a new command regenerates an index. A declared `index.freshness`

`format` may span several lines (write the lines separated by `\n`), so a YAML-style or
link-plus-description entry is checked as one entry; it needs `{id}` or `{file}`, so a link list that
shows no id is identified by its file; and `{title}` is the heading without the record's number or
id. `urzua index` regenerates the declared index artifacts from the records: it only reports which
differ until `--apply` writes them, keeps everything outside the `begin`/`end` markers, and a second
run changes nothing.

#### A rule can now be set to `level: info`. Its findings are reported and counted like a warning's, with

`severity: "info"` in the report, but they never set `blocking` and never fail a run, whatever
`fail_on` says, so an advisory rule can be enabled in a repository that fails on warnings. `info` is
also accepted for a `field_levels` entry and a boundary `severity`, and a rule set to `info` never
reports above `info` because a field level or a boundary declared otherwise. `fail_on: info` is
refused, and every unrecognised level is refused at load with a message naming `off`, `info`,
`warn` and `error`.

#### `urzua init` now sets up a repository that has no records yet. `urzua init --types adr,rfc,spec,prd`

picks the record types to govern from built-in profiles, and `--dir` sets the layout root (default
`docs`, so each type lives in `docs/<type>`). The profile for each type is written out into
`.urzua/config.yaml` as ordinary, editable rules: its required fields, status values and allowed status
changes. A starter template for each type goes in `.urzua/templates/`. On a terminal, `urzua init` in a
repository with no record directory asks which types to govern. Running it again with the same
selection changes nothing and exits 0.

Record-shaped files that no selected type governs are reported in a notice, never as an error, and
adopt now names a record sitting at the repository root, which it cannot assign to a type.

#### `citation.issue-exists` can now also check named header fields. Declare `issue_refs.fields: ["Found-in"]`

and a `#N` or repository link in that field of any record is held to the committed snapshot like one under a
reference section; a finding names the field and its line. A name no record type declares is refused when the
config loads. Off unless declared, so existing configs behave as before.

#### Adds a new opt-in rule, `config.known-fields-declaration-missing`: reports a record type that

declares no `known_fields` at all, so a repository can require every type to make its field-set
governance an explicit choice (`known_fields: []` at minimum) instead of leaving it as the unchecked
default `header.field-set-consistency` gives an undeclared type. Mirrors
`config.pointer-declaration-missing`'s shape and message style exactly.

Design decided in `RFC-43`/`ADR-62`, in response to a code-review finding that correctly identified a
real (but already-decided, `ADR-53`-governed) gap: an undeclared field, including a case-variant of an
already-declared one, goes unchecked for a type with no `known_fields`. This rule gives a repository a
lever to close that gap for itself without changing the lenient default for everyone.

Enabled in this repository's own config in the same change; no fixes were needed since all six
declared record types already declare `known_fields`. No adopter-facing behavior change for a config
that doesn't enable the new rule: verified with the full test suite, clippy, `make ci`, and a
real-corpus `check` run reporting the same 70 findings before and after.

#### Add three tools to `urzua mcp`: `urzua_show_record` (one record in full, with the relation edges that touch it),

`urzua_get_findings_for_path` (which records govern a path) and `urzua_get_graph` (every relation edge). Each returns the report
the matching command prints, and none writes. The server's registration is documented in `docs/reference/mcp.md`.

#### New command `urzua migrate filenames`. It renames every record file whose name does not match its

type's filename convention, the declared `identity.pattern` or `TYPE-NNNN-slug.md` when none is
declared, keeping the directory, number and slug so a record's id and `Stable-Id` do not change. It
rewrites relative links and path mentions of the renamed files in records and other tracked markdown
files, and lists in `unrewritten` any it finds in other tracked files. A file it cannot rename safely
(no number, a name the pattern rejects, a new name that already exists) is listed in `skipped` and
left alone. It prints one JSON document and writes nothing unless `--apply` is given; a second applied
run changes nothing.

#### New opt-in rule `header.case-variant-key`: reports a record whose header writes two keys differing only

by case (`Status` and `status`), naming each spelling and its line. It needs no field declaration, so it
covers a record type that declares no vocabulary, and it reports the difference without saying which
spelling is right. Field names still compare exactly (`ADR-57`): an exact repeat of one spelling remains
`header.required-fields`' finding, and no other rule's verdict changes. Declare it under `rules:` at
`warn` or `error` to enable it; the run reports how many records it examined.

#### Add `urzua export --format json`

Emits every record of every configured type as plain JSON on stdout: its type, path, display id
(e.g. `MILE-115`), `Stable-Id` when it has one, the header fields as `check` parses them, and the
file's full text as `raw_text`. Tooling can read records through `urzua` instead of a second
parser. `--format json` must be named explicitly; `--format agdr` is unchanged and still not
implemented.

A record of a `header_shape: none` type is reported as `header_readable: true`. A tracked record
whose real path is outside the repository (a symlink) stops the export with an error instead of
having its contents published.

#### New opt-in rule `citation.resolution`: a decision ID cited anywhere in a record's prose (for example

`ADR-12` in a Context paragraph) must resolve to a record, or the rule reports an error naming the file
and line. It reads the identifiers carrying a record type's configured prefix, compares numbers by value
so `ADR-0012` and `ADR-12` are one id, and skips the header (which `pointer.resolution` governs) and
fenced code blocks. Declare it under `rules:` at `warn` or `error` to enable it; the run reports how many
citations it examined, so a corpus that cites nothing is distinguishable from a clean one.

#### Adds an opt-in `allow_extension_prefix` option to `header.field-set-consistency`. A repository that

declares `allow_extension_prefix: true` on that rule can annotate a record with a local field whose
header key begins `x-` (in any case) without declaring the field in `known_fields`. Undeclared keys
without the prefix are still findings, so a typo in a real key is still caught. The option is off by
default, so `check` reports exactly what it reported before for any repository that does not set it.

```yaml
rules:
  header.field-set-consistency:
    level: warn
    allow_extension_prefix: true
```

The option is accepted only on `header.field-set-consistency`; a config that declares it on any other
rule fails to load.

#### `field.pending` takes the optional `pending_until_required` option. Declared `true`, a pending value in a

field the type's `statuses.require` ties to certain statuses is not reported until the record reaches one
of them, so a placeholder such as "not yet written" on an open bug is no longer a warning while the same
placeholder on a fixed bug still is. A deferred slot stays counted as eligible and unexamined, a field no
`require` entry names is judged as before, and the option is off unless declared.

#### Adds `discovery.include_untracked`, an opt-in configuration option that makes `urzua check` read the record files you have written and not yet staged. With `discovery: {include_untracked: true}`, discovery includes untracked, non-ignored files directly under a declared record directory (git's own ignore rules decide), so a malformed draft is found before `git add`. Under `--base`, such a record counts as added and the diff rules judge it like a staged addition. Embodiment drift cannot judge a record with no git history; an info notice, `discovery.untracked-not-drift-judged`, counts the records it skipped for that reason. The report's `scope` gains `include_untracked` and `untracked_included`.

Without the option, nothing changes in what is examined, but a plain `check` no longer stays silent about drafts: an info notice, `discovery.untracked-skipped`, names the untracked records under declared directories that it did not examine (at most 20 paths and the count). It does not change the exit code. An unknown key under `discovery`, or a value that is not a boolean, is a load error.

#### A record type declares its revision log's class vocabulary and columns in `revision_log`:

`classes`, `class_column`, `date_column` and `tool_class`. `revision-log.change-class-required`
accepts exactly the declared classes and lists them, and reads the class and date from the columns
named by header, in any position; a header that lacks a declared column or repeats it is one
finding with an `edit-section` action, and `revision-log.version-bump-required` reports that
record, or a merge-base log it cannot read, as unreadable. `urzua fix --apply` and
`--rebaseline` write the type's `tool_class`, laid out by the log's header. A type that declares
none of the keys behaves as before.
A pipe written `\|` stays inside its table cell. When a type declares a column, a row with more
cells than its header is one finding that names the row and its cell count, instead of being
read by guesswork.

#### `urzua new` no longer reuses a number already taken on another branch. Besides the working tree, it

now looks at the type's directory on every local branch and remote-tracking ref, takes the next
number above the highest found, and prints to stderr which ref(s) already used each number it
skipped. This uses git only, with no network, so a branch that was never fetched into your clone is
still invisible: run `git fetch` first. If git cannot be read, `new` numbers from the working tree
as before and says so on stderr.

#### `urzua new` can now skip a number an open pull request already took. With `new: {check_open_prs: true}` in the configuration, `urzua new` fetches pull-request head refs with plain git (no `gh`, no API) into a private `refs/urzua/pr/` namespace before it numbers a record, then skips every number found there, on any branch, or in another worktree of the repository, untracked files included. A lock in the git directory makes two runs started together take two different numbers. `new.pull_ref_patterns` (default `refs/pull/*/head`), `new.remote`, `new.fetch_timeout_seconds` and `new.lock_timeout_seconds` tune it. Pull requests that were closed without merging keep their numbers reserved, which can leave a gap and never a duplicate.

Offline is never an error: when the remote is missing, unreachable or slow, `urzua new` numbers from the refs it has, exits 0 and adds a warning notice. The JSON report gains an always-present `pull_refs` object (whether it fetched and why not, the remote, the ref and worktree counts, and each skipped number with the refs or worktrees that held it). With the option off, `new` touches no remote.

Adds the opt-in rule `identity.open-pr-collision`. Under `urzua check --base`, it warns when a record the diff adds has the same type and number as a file on a fetched pull-request ref under a different `Stable-Id`. `check` never fetches: with no refs present the rule says how to fetch them and counts zero refs.

#### A record type in `.urzua/config.yaml` can now declare its document model with three optional keys,

`identity` (`from: filename` plus a `pattern`), `fields` (`from: prefix-lines` or
`yaml-frontmatter`) and `sections` (`from: headings`, with optional `depth` and `items`). The
declaration is validated when the config loads: an unknown source, an unknown key, an out-of-range
`depth`, a blank `pattern` or a key with no value is an error naming the record type and key. No rule
reads the declaration yet, so declaring it changes no finding; a config without it loads exactly as
before.

#### A record type that declares `sections.depth: 2` has its `###` headings read as sections nested under

their `##`, and a required section can declare `level` and `under` to ask for one, for example
`{name: Pros and cons, level: 3, under: Options}`. A level the declared `depth` does not reach fails
the config load. Types that declare no `level` are checked exactly as before.

#### A record can declare its kind, and the kind decides which evidence tiers count as delivered. Under `statuses.delivery.items`, a new `kind` key names a header field and a vocabulary of kinds; each kind may list its own `delivered_tiers` and may set `cited_records: success`. A `decision` kind that counts `Specified` computes delivered when every Done means item resolves at `Specified` or higher, so a milestone that only records a decision can now compute `Done`. With `cited_records: success`, a locator that resolves to a record counts only while that record's status is one its own type declares `success`: a Proposed ADR or a Draft spec evidences nothing. A record with no kind, or a kind that names no tiers, is judged by the type's `delivered_tiers` as before, and a configuration with no `kind` key is unchanged.

`status.done-items-shape` reports a `Kind` value outside the declared vocabulary, naming the allowed kinds, and the record is then judged as one with no kind. A `kind` declaration is refused at load when its field is blank or not one the type declares, when it lists no kinds or one twice, or when a kind's `delivered_tiers` names a tier that is not declared, not in `resolving_tiers` or in `unevidenced_tiers`. See `statuses.delivery.items.kind` in the configuration reference.

#### `urzua new` now follows the record type's declared `identity` pattern, status vocabulary and field

set, so a generated record passes `check` with no edit. A type declaring a padded pattern such as
`\d{4}` gets a matching file name (`ADR-0004-slug.md`), an H1 that opens `ADR-4`, and numbering read
from that pattern. The status field starts at the first value of the type's declared `statuses`
instead of a literal `null`. `Stable-Id` is left out, with a warning notice and `"stable_id": null`
in the report, when the type's closed `known_fields` does not list it. A pattern no file name can
satisfy is refused before anything is written.

#### Every command's JSON now carries a top-level `next_action` and `next_action_reason`, always present.

`next_action` is one runnable command, such as `urzua fix --apply --ids docs/adr/ADR-1-a.md`, or `null` when nothing is better
than reading the findings; `next_action_reason` is a short count-based reason such as
`repairs 3 of 5 findings`, and is `null` exactly when `next_action` is. `check` builds the command
from the unwaived findings' `run-command` actions (`urzua fix --apply --ids ...`, then
`urzua index --apply`), `fix` points at `urzua fix --apply --ids ...` after a detect run and `urzua check`
after an apply, and every other command emits both as `null`. The `urzua mcp` tools carry them too.

#### `citation.resolution` accepts an optional `non_reference_field` naming a header field in which a

record lists, as comma-separated exact ids, the ids it names without citing as a reference (an
example, or a number minted and retracted). A citation of a listed id is neither resolved nor
reported in that record only, so it no longer starts resolving to an unrelated record when one is
later created with that number; the same id cited in another record still resolves. A listed id the
record never cites and a malformed entry are reported, a field that is unknown or is a pointer,
narrative or relation-role field is a config error, and `urzua check` prints one notice naming the
listed ids that now match a real record.

#### A record type can own more than one directory. Declare `extra_dirs` beside `dir` and every listed

directory is part of the type's one identity namespace: `urzua new` numbers past the highest id in
any of them (and past a number claimed there on another branch), pointers and citations resolve
across them, and `identity.collision` and `filename.identity-pattern` see them as one population.
A directory listed twice, or owned by two types, is a config error, and an `extra_dirs` entry that
does not exist is reported by `type.dir-matches-nothing`.

#### The output contract now holds on stdout alone. `check`'s `blocking` is true exactly when the run

exits 1, so under `fail_on: warning` an unwaived warning reads `"blocking": true` (it read `false`
beside exit 1). A path argument that selects no record gets a warning notice even when another
argument matches, and every `not-run` report says why in an `error` or a notice. `waive` reports
the identity it used in place of `--by`, and numbering's notices, in a `notices` field, and `new`
reports a missing type directory as a notice. `rules` carries `"status": "ok"`. In the MCP server a
`not-run` report is an error result, and `urzua_list_records` no longer carries the whole-corpus
`roadmap` block.

#### `statuses.pending` accepts a pattern entry, `{pattern: "^\\(pending"}`, beside whole values, so a family of

placeholder statuses that share a stem is declared once. A value matching no entry and no pattern is still
a real value, and an invalid, blank or repeated pattern is a load error naming the type.

#### A suppression type can declare `permanent`, which says per rule which reasons let a waiver without

`Expires` take effect. Each entry names a rule, a `reason_field` the type declares, a non-empty list of
`categories` (each optionally `cite: required`) and, for a category that requires one, a `cite_field`
that is a pointer field. A waiver with no `Expires` then suppresses only for a listed rule, with a
category from that rule's list and, where required, a citation that resolves to a record. Declaring the
key also closes permanence for every rule it does not list. Otherwise the waiver suppresses nothing, its
finding is reported again, and `waiver.matches-nothing` reports it as `waiver-ineffective` with a message
that now names whichever requirement is unmet instead of always naming `effective_requires`. `check`
adds an info notice, `permanent-waivers`, counting the permanent waivers in effect by category. A waiver
with an `Expires`, and a config that declares no `permanent`, behave as before. Config load refuses an
unknown rule, a field the type does not declare, an empty or duplicated category list and a `cite_field`
that is not a pointer field.

#### The `embodiment.locator-promotion-candidate` rule accepts `scope: changed`.

With the option, a locator cited by more than one record is reported only when a record the
`--base` diff adds or changes cites it. The finding sits on that changed record and names every
citer, changed or not. A plain `check` reports none, and the records it did not judge are counted
in the population's `deferred_by_scope`.

#### A relation pair's `provisional` declaration takes an optional `acknowledged_by` field. A provisional claim

(from a claimant at one of its statuses) whose peer names the claimant in that field is settled by the peer's
own declaration and is not reported by `relation.pair-reciprocity`; a claim from any other status still needs
the inverse relation.

#### Add `urzua mcp`, a read-only Model Context Protocol server on stdio, so an agent can read the corpus without a shell. It

serves two tools, `urzua_check` (findings, scope and the rules run) and `urzua_list_records` (each record's id, type, path and
header fields, filtered by record type and by any status, so rejected and superseded decisions are one query). Every result is
the report the matching command prints, as structured content plus the same JSON as text. The server has no tool that writes.

#### A record-edit hook for coding-agent harnesses judges an edit to a computed field such as `Embodiment`

before it is written, by relaying `urzua fix --stdin-path`. It blocks a value that disagrees with the
evidence, says plainly when it could not judge, and accepts a named override. A Claude Code adapter
and `.claude/settings.json` wiring ship in this repository.

#### Adds a new optional per-type config key, `relation_fields`, mapping a fixed role set (`status`,

`embodiment_state`, `embodiment_locator`, `supersession`) to the field name that plays that role for a
record type — e.g. `relation_fields: { status: State }` for a type that calls its lifecycle field
`State` instead of `Status`. Every role defaults to its pre-existing literal
(`Status`/`Embodiment`/`Realized-by`/`Supersedes / Superseded-by`) when undeclared, so no existing
config needs to change.

Replaces seven rules' hardcoded field-name literals (`claim_status_agreement`, `pointer_target_status`,
`narrative_field_stale`, `embodiment_consistency`, `embodiment_locator_exists`,
`embodiment_locator_promotion_candidate`, `supersession_reciprocity`) with a lookup through the
declared-or-defaulted name, in both the gating logic and every finding message, closing `BUG-110`
(`Status` hardcoded) and the same defect shape found recurring in
`Embodiment`/`Realized-by`/`Supersedes / Superseded-by` during review. Adds
`config.relation-field-not-known`, reporting a declared `relation_fields` override not also present in
that type's `required_fields`/`known_fields`, mirroring `config.pointer-field-not-known`.

Design decided in `RFC-42`/`ADR-61`. No adopter-facing behavior changes for a config that doesn't
declare `relation_fields`: verified with the full test suite, clippy, `make ci`, and a real-corpus
`check` run reporting the same 70 findings before and after.

#### Add `relation_pairs` to `.urzua/config.yaml`: declare a pair of inverse relation fields (a forward

field and its inverse), whether the two ends may be different record types, and whether reciprocity
is required. Two new opt-in rules use it. `relation.pair-reciprocity` reports a claim on one side
that the other side does not echo, in either direction and across record types.
`relation.cross-type-disallowed` reports a claim to another record type in a pair that does not allow
it. Enabling either rule without a declared pair fails the config load. Existing configs and
`relation.supersession-reciprocity` are unchanged.

#### Add a per-category drift policy to `embodiment.claim-expired`: `drift: {code: block, test: block, spec: acknowledge}`.

`block` (the default for a category not named) refuses a renewal until the drifted locator is re-pinned; `acknowledge`
lets `urzua reverify --apply` renew over the drift and writes each drifted locator (a cited record by its id and its
newest revision-log date, any other file by its path, hash prefix and the commit that last changed it) with the reviewer's `--reason` into the
`Re-verified` row, after a `; Drift acknowledged:` separator, and re-pins a drifted pinned locator in the same act (old and new hash recorded); `ignore` leaves the category's drift out of renewal. An unknown category or policy is a load error
that lists the valid ones. The refusal for drift in a category that `--pin-only` cannot pin, such as a `spec:` locator,
now names the category, its policy and the `drift` setting, instead of suggesting `--pin-only`, and `reverify`'s review
lists a claim the policy lets through as renewable with `drift_acknowledged`. A configuration that declares no `drift`
renews as before.

#### A record type can declare how its revision log is found with `revision_log`: a `marker` line (default

`**Revision log**`) or a `heading`. The log is the section that line opens, never a line that merely
mentions the text, so a record whose prose contains the literal is no longer read against the wrong
table. `revision-log.change-class-required`, `urzua fix` and `citation.path-exists` all read the
declared identity. Existing repositories are unchanged.

#### A record type's `revision_log` declaration takes `required: true|false` (`true` when omitted). With

`required: false`, `urzua fix --apply` writes a computed field into a record that has no revision log
section, without adding a log row; the record type needs no `marker` or `heading` for that, so
`revision_log: {required: false}` is enough. Where the log is required, a record without the section
still fails the write, and the error now names the marker or heading it looked for.

#### Fixes several real defects found by review, plus a standardization pass on a recurring nondeterminism

class:

- **`HashMap`/`HashSet` iteration-order nondeterminism, standardized.** A declared field set's
  iteration order (feeding `field.untrimmed-value`/`header.field-case-mismatch`'s finding order) is
  now a `BTreeSet`, not a `HashSet` — empirically confirmed flaky before the fix (13/20 separate
  process runs disagreed on finding order for the same unchanged corpus; 20/20 stable after).
  `Config.record_types`/`Config.rules` are now `BTreeMap`, closing the same class at its root — this
  also lets `Config::sorted_type_names()` (round 22's workaround) simplify to a plain `.keys()` call,
  since `BTreeMap` already iterates in sorted order. Every other `HashMap`/`HashSet` in the codebase
  was audited and confirmed genuinely order-independent (pure lookup/containment, never iterated into
  observable output) and left unchanged.
- **`urzua audit` silently used a lossy record index and never ran `identity.collision`**, unlike
  `check` — a real identifier collision was resolved against an arbitrary record with no finding at
  all. `audit` now builds its index the same way `check` does and runs `identity.collision` too. As a
  side effect, `audit` on a prefixless corpus (`BUG-99`) now correctly reports `ok` instead of
  spuriously `not-run`, since `identity.collision` doesn't need a declared prefix to judge a corpus —
  `BUG-99` stays `Open` (the root cause is unrelated and still blocked on `RFC-38`), but its own
  reproduction no longer reproduces.
- `check.rs`'s `claim.status-agreement` enablement check was duplicated across two independent call
  sites; both now share one `claim_status_agreement_setting` function.
- `RecordTypeConfig`'s `header_shape: none` gate (`config.header-none-has-no-required-fields`,
  `config.known-fields-declaration-missing`, and `header.required-fields`'s own check) was duplicated
  three times; all three now call one shared `has_no_header()` method.
- A stale `#[allow(dead_code)]` on `RevisionLogEntry`'s `date`/`change_class` fields removed — both
  are actually read.

No adopter-facing behavior change beyond correctness: verified with the full test suite (each fix has
a planted-violation test, individually confirmed genuinely observed-failing before and passing after),
clippy, `make ci`, and a real-corpus `check` run reporting the same 70 findings before and after.

Three more findings from the same review are filed rather than fixed here (each performance-only or
needing a larger refactor, not correctness-affecting): `BUG-133` (`is_record_reference`/
`parse_record_filename` grammar duplication), `BUG-134` (a per-type-constant field set rebuilt per
record instead of using the existing cache), `BUG-135` (`compute_drifted_records` spawning
uncached git subprocesses per locator per record).

#### The drafting commands for ADR, RFC and milestone records are now generated like those of every

other record type, and the type-specific guidance they carried (an ADR records a choice already
made, an RFC a direction still open, a milestone planned work with checkable done-means criteria)
lives in the type's template, so `urzua harness status` and `urzua doctor` report a repository
that runs entirely on the installed bundle.

#### A scoped evidence pin drifts only when the hash of the item it names changes: an edit elsewhere in

the file is not drift, and a scoped pin has no file-based grace window (a declared audit date still
holds it back). `urzua explain <path>#<scope>` resolves a scope and prints the span, its hash and
the locator to paste into `Realized-by`. `urzua fix --rebaseline` shows the resolved span beside the
hashes, re-pins a scoped pin in the spelling it was written, and lists a pin whose scope no longer
resolves under `failed_pins` instead of re-pinning it.

#### A `Realized-by` locator can carry a scope between its path and its pin: `code:path#Symbol` for a

named item, `code:path#Type::member` for a method found across all of a type's `impl` blocks,
`code:path#Type::<Trait>::member` for a method of a trait impl, or `code:path#L10-L40` for a line
range. Symbols are found by resolvers a repository declares in the new top-level
`locator_resolvers` (a glob, a start pattern, and an end kind: `brace_balanced`, `indent`,
`heading`, `blank_line` or `semicolon`); the engine knows no language. `embodiment.locator-exists`
reports a locator that does not parse, a scope that does not resolve (not found, ambiguous, no
resolver, end not found, range past the end of the file, file not UTF-8), a scope given without a
pin, and a commit anchor, which is reserved and not yet supported.

#### A required section can declare `aliases`, other headings that satisfy it, and `preamble: true`, which

reads a marked Y-statement from the text before the first `##` heading when its section is absent.
`urzua check` reports each alias in use as an info notice, and the shape check judges an aliased
section or the preamble the same way it judges the named section.

#### Add `required` and `exempt_statuses` under a record type's `sections` declaration, and two opt-in

rules that read them. `section.required` reports a `##` section a record of the type must carry and
does not. `section.shape` checks a present section's content shape: `y-statement` requires the five
phrases `In the context of`, `facing`, `we decided`, `to achieve` and `accepting` in one block
(opened by an optional `marker` such as `TL;DR`), and `table-min-rows` requires a table with at
least `min_rows` data rows. Records in an `exempt_statuses` status, such as `Withdrawn`, are counted
out of scope. A type that declares no `required` is unchanged, and both rules report their population
in a new `section` unit.

#### Add three opt-in rules over a record's section structure, each declared per record type under

`sections`. `section.duplicate` (`unique_headings: true`) reports a heading that appears twice at one
level under one parent, naming both lines. `section.level-skipped` (`no_skipped_levels: true`)
reports a section nested more than one level below the one above it, such as a `###` with no `##`
above it, reading levels from `##` down to the type's `depth`. `section.order`
(`order: {from: template}`) reports a section that follows one the type's template
(`.urzua/templates/<type>.md`) puts after it. A type that declares none of the keys is unchanged, and
a declared order whose template is missing or holds no section is an error on each record of the type, so it fails a `fail_on: warning` run and the records are counted unexamined.

#### Add `acyclic: true` to a `relation_pairs` entry to declare it a sequencing relation, for example

`Depends-on` with its inverse `Blocks`. Two new opt-in rules read it. `sequence.cycle` reports every
record on a cycle with the cycle's path, including a record that depends on itself and a cycle across
record types. `sequence.unresolved-blocker` reports a value in either field that names no record,
whether a reference that resolves to nothing or prose. Enabling either rule with no acyclic pair
declared fails the config load. Existing configs are unchanged.

#### A record type can hold its `Status` to delivery evidence. `statuses.delivery` declares which computed

evidence tiers mean the work was delivered, which tiers mean nothing was, which statuses mean not done,
and the success status to set. The opt-in rule `status.delivery-agreement` reports a disagreement both
ways: a record whose `Realized-by` locators or citing source back-pointers show it delivered while its
`Status` reads not done, and a success `Status` with no evidence behind it. `urzua fix` proposes the
success status for the first case and `--apply` writes it with a structural revision-log row; it never
downgrades the second, which it reports as a notice. `fix` now also runs when only this rule is enabled.

#### `embodiment.consistency` takes an optional `sticky_states` list of `Embodiment` values a person sets

that no evidence can compute, such as `Deprecated` and `Inactive`. A record stating one is no longer
reported as drift or as an old claim, and `urzua fix` leaves it alone. A listed value that evidence
does compute is refused at load, since it would hide real drift.

#### `embodiment.back-pointer-resolution` takes an optional `test_paths` option declaring the test class:

a `Verifies: ADR-12` comment in a file under it is test-tier evidence, so a record whose tests cite
it computes `Verified` without a restated `test:` locator list. `embodiment.consistency` also judges
a record that has `Embodiment` but no `Realized-by` field when a source back-pointer cites it,
computing from the back-pointers and naming that evidence in the finding; a record neither cited
nor carrying `Realized-by` is still not judged. `urzua fix` repairs the same records.

#### Adds `field.untrimmed-value`, reporting a declared field whose value carries leading or trailing

whitespace. Every rule compares values exactly, so `Status: "Superseded   "` is not `Superseded` and a
status rule configured to report that status stays silent on it. Trimming inside those rules would
accept a value the author did not write; this reports the whitespace where the author can fix it,
which is the split `yamllint` makes between formatting and meaning.

Reachable only through `yaml-frontmatter`: the blockquote parser trims at parse time, and YAML trims
an unquoted scalar, so it takes a quoted value to carry the space through.

`narrative-field.stale` previously trimmed the target status where `pointer.target-status` and
`claim.status-agreement` did not. All three now compare exactly.

Also adds `release-guard` to the `Makefile`'s `.PHONY` list, where it was missing while its sibling
`release-invariants` was present.

#### `urzua waive --from-report` gains `--apply`, which writes the drafts it previews. Each draft is a

waiver with a pending `Reason` and no `Expires`, numbered once under the same lock `urzua new` takes,
so a second `--apply` writes nothing. `urzua check` afterwards reports each original finding again
beside a `waiver-ineffective` finding on its draft, until a person fills in a real `Reason` and an
`Expires`. A write error stops the run with an `error` field and `drafts[].written` says which drafts were
written: `partial-failure` with exit 1 when some were, `not-run` with exit 2 when none was. The report also lists, as skipped, a finding that persists even though its
own draft would hide it, and a finding inside a waiver record.

#### New command `urzua waive --from-report`. It runs `check`, then previews one draft waiver per rule and

file for the findings that are unwaived and at or above `fail_on`, each with a pending `Reason` and no
`Expires`, so a draft suppresses nothing until a person fills it in. It prints one JSON document:
the drafts, the findings it skipped with the reason (an existing waiver, an expired one, a finding the
drafts themselves resolve), and any `--rule` or `--path` filter that matched nothing. It refuses
(exit 2) when a draft could take effect: the waiver type must require `Reason`, and must either declare
`permanent` or require `Expires`.

#### Add `authority`, a top-level configuration key that says who may accept a record and who may change a protected path: named decider groups of `logins`, the `evidence` that may name one (`pr-author`, the default, or `pr-approver`), and `gates` by record type and status or by path. A group with no login, a login in no group, and a gate that names a type, status or group the configuration does not declare are load errors naming the entry; so is enabling the `acceptance` or `protected-paths` guardrail with no gate of its kind.

With those guardrails enabled, `urzua harness run edit` refuses an agent's Write, Edit or MultiEdit that moves a record to a gated status, or writes a gated path. The refusal is final: a named override does not lift it and the output says so. A person editing outside the agent harness is not affected. If the configuration does not load, an agent's edit to the configuration or a gated path is still refused, with the way out named.

Add the opt-in rule `diff.decider-required` and `urzua check --actors FILE`. Given the pull request's author and current approvers as JSON, it reports a gated move or changed gated path whose author or approvers are not in the gate's group, judged against the merge base, and it is `not-applicable` without `--base`. With no actors file a local run reports `info`, and the option `require_evidence: true` makes that an error. It proves authorship, not who decided: with a single maintainer it cannot tell an agent acting under the maintainer's token from the maintainer. The `urzua-gate` action takes an `actors: "true"` input that writes the file from the GitHub API, listing only approvals given on the head commit. `urzua doctor` says that branch protection, the required status check and `CODEOWNERS` are the host's and are not verified.

### Fixes

- `urzua init` no longer proposes `section.required`, `section.shape`, `section.duplicate`, `section.level-skipped` and `section.order`. Each reads a `sections` key that `init` never writes, so on a new or adopted corpus they ran over no record and reported a clean pass. The notice listing the rules `init` withholds now names them and the key each needs (`sections.required`, `sections.unique_headings`, `sections.no_skipped_levels`, `sections.order`); declare the key on a record type, then enable the rule.
- Source comments in the drift and re-baseline code now cite `ADR-67`, the decision that introduced evidence pins, the grace window and attributed re-baselining. No behaviour changes.
- Test-suite hygiene only: integration-test fixture directories are now removed when each test ends, including on failure, and CI fails when a test run leaves a `urzua-*` entry in the temp directory. No change to the `urzua` binary's behavior.
- A milestone whose evidence is its Done-means item clauses, with no `Realized-by` line, now expires under `embodiment.claim-expired` and is renewed by `urzua reverify` like any other claim (`ADR-97`). Its age counts from the first evidence line, `reverify` pins the item locators, and a record with neither a `Realized-by` value nor an item evidence clause stays not examined with the reason named. A record that states `Realized-by` is judged as before.
- A record whose Done means items all resolve but which depends on an unfinished record now computes `InProgress`, not `Done`, so `status.delivery-agreement` and `sequence.status-agreement` agree on one status for it. `sequence.status-agreement` skips that record with an info notice, `all items proven ahead of dependency X`, and still reports a record in progress out of order, one with an unresolved item. The finding, and the `urzua fix` notice, name the dependency that caps it. Configurations that declare no Done means items or no sequencing pair are unchanged.
- The editor hook now also judges a NotebookEdit and any MCP tool whose input carries a file path and the file's content, as a write of that file; an MCP call it cannot read as a whole-file write is refused under `on_unjudged: deny` only when its file is a protected path or a gated record. The post-command check reports a shell command that changed a gitignored protected file (such as `.claude/settings.local.json`) by keeping a content hash of every file under a protected path. Protected-path and record-directory matching use Unicode case folding, and on Unix a write through a hard link is judged as the protected path or gated record it shares an inode with. A change is still reported once; the documentation now says so.
- The post-command check now reports a shell command that changed a protected path (an `authority` path gate, such as `.claude` or `.urzua/config.yaml`), and covers a command that committed its own change: it remembers the commit it last saw (seeded at session start) and examines the files changed since. Both are reports, never blocks. An edit the adapter could not hand to the engine (an unreadable payload, an edit it cannot place, an engine that cannot be run) is now refused where `acceptance` or `protected-paths` is on with `on_unjudged: deny`, and still allowed with a notice where the configuration allows. An edit from a working directory outside the repository is judged by the repository that holds the file, protected paths and record directories match without regard to letter case, and an edit through a symlink is judged as the file it resolves to. `urzua export` and `urzua import` now require `--format`, and the `urzua fix --help` text matches what `fix` does.
- A source comment in the configuration reference now notes when the "no external adopters" premise ends. No behaviour changes.
- This repository now judges each milestone by its Done means items: a milestone's status follows the evidence its items name, and 22 milestones that claimed more than their items show are open again. No behaviour of the released CLI changes.
- A rule option declared in the configuration is now defined in one place inside the tool, so a newly added option can no longer be accepted by the loader yet ignored by the rule. Configuration load behaves as before: an option on a rule that does not take it is still refused, with the same message.
- Paths the tool reads or writes on a name from the arguments or the configuration are confined to the repository, symlinks included. `index --apply`, `fix --apply`, `migrate ids --apply`, `refs refresh` and `init` refuse to write through a symlink that leaves the repository, and `explain` refuses an absolute path, a `..` path or a symlink to a file outside it. `urzua --config FILE init` writes `FILE` instead of `.urzua/config.yaml`. `fix --reason` refuses a carriage return, `fix --rebaseline --apply` hashes the evidence after taking the lock, and the not-implemented message names the right spec.
- `embodiment.claim-renewal-unattested` says an anchor field "is malformed" when the branch left a value that is not a date, and "was removed" only when the field is gone. A back-pointer rule's `ignore_marker` has its back-pointer keywords and record id heads removed whatever their case before the specificity check, so `implements:`, `IMPLEMENTS: ADR` and `adr` alone are refused at load; markers that were specific still load. `sequence.status-agreement` and the dependency cap read the same dependency scan; their findings are unchanged.
- The editor hook judges an MCP file path that starts with `~` or `$HOME` as the home directory it names, and a path nested under a content key (`data`, `payload`, `value`, `body`, `contents`, `text`) like any other nested value; a string under such a key is still the file's content. A record file whose extension is `.MD` or `.Md` is a record, in discovery and in the acceptance gate. With `outside_writes: shared`, a record in a linked worktree is compared with that worktree's own copy. The gate action's comment step updates only a comment posted by a bot account, so a comment that merely contains its marker is ignored.
- A back-pointer rule's `ignore_marker` is refused at load when it is a fragment of a back-pointer keyword run into a record id head (`ements: ADR-`) or is itself a record id (`ADR-1`), since either would excuse every citation it appears in. Two `drift` category keys of `embodiment.claim-expired` that differ only by whitespace are refused at load instead of collapsing into one. The drift grace-window notice says the first window ends on a date in UTC.
- Records only: corrects the evidence behind two fixed bugs, an acyclic-pair amendment, and this repository's own `sections.depth` declaration. No change to `urzua` behaviour.
- No change to what the CLI prints. A repository that declares none of the opt-in options (`level: info`, `roadmap.work_types`, `scope: changed`, `statuses.delivery.items`) now has a regression test holding its `check`, `check --base`, `roadmap`, `export --format json`, `fix` and `explain` output byte for byte to what those commands printed before the options existed. The same release adds tests that run `urzua audit`, `urzua check --rules` and the key-absent `roadmap` and `export` over this repository's own corpus, show that `scope: changed` leaves a rule's suggested actions as they were, and run `urzua explain` over the Done-means-items acceptance corpus.
- Hardens `urzua reverify` (`ADR-97`). A locator that points outside the repository or at a file git does not track no longer resolves for a pin, in `reverify`, `fix --rebaseline` and the drift check. A first pin is written on the exact locator, so pinning `a.rs#Foo` no longer edits `a.rs#Foo::bar`. At an `error`-level period any actor but `--actor human` is refused, including an omitted `--actor`. `--force --ids` renews the named record. Detect lists a claim whose record has no usable revision log as refused, with the reason, instead of offering a renewal that `--apply` refuses. `apply_reverify` and `apply_rebaseline` share their evidence-line and revision-row code, with no change to what either writes.
- The agent harness no longer lets an edit through that it could not judge. A request the engine cannot read (a tool input with a lone surrogate escape, for one), or whose working directory cannot be entered, is refused for an agent where `acceptance` or `protected-paths` is set to `on_unjudged: deny`, and a working directory that cannot be entered is replaced by the session's project directory, which the Claude Code translator also runs the engine in. An MCP call that carries two different strings under content keys is treated as an edit the translator cannot place, instead of judging one of them. The engine judges at most 512 paths of a request and refuses the rest under deny, and asks git about each directory once. `urzua harness status` reports a `disableAllHooks` kill switch, and compares the installed bundle's version with the binary's as numbers (`0.10.0` is newer than `0.9.0`).
- An invalid record-type setting is now refused when the config loads, instead of producing unrecognizable or colliding records. A `prefix` that is empty, contains a path separator, `..`, whitespace, lower-case letters or a trailing hyphen, or that two record types share, is an error, as is an empty or `.` `dir`. `urzua doctor` reports the same errors, and now also reports an error when no rule is enabled, a config on which `check` exits 2 and examines nothing. `embodiment.locator-exists` no longer reports a `./`- or `..`-spelled path to a tracked file as missing.
- Every rule is now registered once, in one definition table. No rule or `check` output changes.
- A relation claim to a record type outside a pair's `types` list no longer makes `config.scope-matches-nothing` warn that the configuration cannot reach it; the claim is counted as eligible and unexamined.
- Test-suite hygiene only: the remaining unit and integration test fixtures that were removed on the success path only are now removed when the test ends, including when it fails. No change to the `urzua` binary's behavior.

#### An absent `claim_paths` directory no longer aborts `check`. Git keeps no empty directory, so a

declared `.changeset` ceases to exist the moment a release consumes the last fragment, and a
repository following the documented pattern lost `check` entirely. The absence is reported as a
notice, which is visible and never moves the exit code; a path that is actively wrong — a file where a
directory was declared, or a link resolving outside the repository — still aborts.

#### The corpus acceptance report is now a gate. An unfiltered acceptance run fails when a rule has no

failing case and is not on the committed allow-list, `tests/acceptance/coverage-allowlist.yaml`,
where each entry carries a rule id and a reason. The list can only shrink: an entry for a rule
outside the fixed ceiling is refused, and an entry whose rule gained a case or left the rule table
is stale. Five rules are on it; `narrative-field.stale` and `waiver.matches-nothing` gained failing
cases.

#### `reverify` under `drift: acknowledge` renews a record whose drift has outlived `drift_grace_days`

without a prior `fix`, and the acknowledged renewal clears that drift for `embodiment.consistency`
(an unpinned locator is judged from the date of the acknowledging `Re-verified` row that names it; a
pinned one by its re-pin). Only a row in the exact shape `reverify` writes acknowledges or attests
anything, and `embodiment.claim-renewal-unattested` reports an added acknowledging row that names a
locator which had not drifted at the merge base or lists evidence that is not the record's current
locators. `check` prints the `citation.issue-exists` snapshot notice only when that rule is part
of the run.

#### `urzua init` no longer writes a config that cannot load. It never proposes a hidden directory such as

`.changeset`, a directory none of whose files has a header, or a directory of headed pages that carry no
`status` field; it names the ones it left out, gives every type a name that is a legal prefix, drops a
type whose prefix another already holds, and refuses to write a config that does not parse.
`init --dry-run` now works when a config already exists. `harness install` reports a refused install
honestly (every item `would-*`, nothing `created`, `merged` or `written`) and warns, as `doctor` does,
about a hand-copied first-generation editor hook wired beside the bundle's. `doctor` decides that a git
hook or workflow runs urzua from an actual call, not from the word in a comment, and no longer knows a
`make records` target. A tracked record deleted from the working tree is judged as removed and named in a
notice, where it used to make every `check` exit 2. A `--rules` selection no longer reads the source
files under `back_pointer_paths` unless a selected rule needs them.

#### `urzua rules` labels `embodiment.locator-promotion-candidate` as `anchor-scoped`, and with

`scope: changed` the rule reports a shared locator on every changed record that cites it, not only
the first, so `check <path>` and a file-scoped waiver each see the finding on their own record.

#### The source back-pointer matcher now reads every id in `Implements: ADR-1 (note), ADR-2`, continuing

past a parenthetical or trailing note to the next comma-separated id, and reads a marker that
follows code on the same line (`let x = 1; // Implements: ADR-1`). `embodiment.back-pointer-decision-accepted`
judges the same widened set. Prose after an id still cannot name one: `ADR-1 and ADR-2` yields `ADR-1`.

#### `check`, `fix` and `reverify` run `git blame` once per record file instead of once per line, and ask git for each

commit's time once, so a run over a large corpus starts several times fewer git processes and finishes sooner. Output is
unchanged for every run.

#### `census_records` collected every examined candidate's path into a `Vec`, then sorted and deduped the

whole thing once at the end -- for a `Field`-unit rule, a record with many declared fields could
contribute one clone per examined slot before the same set collapsed out of a single bulk sort. Now
uses a `BTreeSet`, deduped incrementally as candidates are examined instead of in one final pass over
the full candidate list. No behavior change.

#### A comma in a `Realized-by` locator path no longer truncates the path. A comma now separates entries only

when it is followed by a category prefix (`spec:`, `code:`, `test:`); any other comma is part of the path,
so `code:src/weird,name.rs` names `src/weird,name.rs`.

#### `urzua check` and `urzua audit` now tell a reference to a record that exists on disk but is not tracked by

git apart from one that exists nowhere. The finding names the file and says it is not tracked, so a record
drafted beside the one citing it no longer reads like a typo. A `Realized-by` locator naming an untracked
file on disk is reported the same way. Tracking the file clears the finding.

#### `urzua check` and `urzua audit` build a shared record-identity index that silently keeps one of several

colliding records (first-seen-wins) for six independently opt-in reference-resolving rules
(`pointer.resolution`, `pointer.target-status`, `claim.status-agreement`,
`relation.supersession-reciprocity`, `relation.target-status-undeclared`, `narrative-field.stale`). If a
repository enabled any of those but left `identity.collision` off, a real identifier collision produced
no finding and no visible warning at all -- every consuming rule silently resolved against whichever
record happened to be inserted first.

Both commands now disclose every identity collision as a `Notice`, regardless of whether
`identity.collision` is enabled -- the same disclosure `urzua graph` already got in a prior release.
`identity.collision`'s own blocking `Finding` is unaffected: still exactly as opt-in as before. No
finding-count change on a corpus with no collisions.

#### `is_record_reference` (prose/pointer-field record mentions) and `parse_record_filename` (filename

parsing) each independently re-implemented the same two "is this a valid record identifier segment"
predicates -- byte-for-byte identical code with no enforcement that they stay in sync. This duplication
is exactly how they drifted apart twice already (a hyphenated prefix, then a digit-bearing prefix), each
time silently breaking every reference to a record of that shape until someone noticed and filed a bug.

The two predicates are now one definition each (`is_digit_segment`, `is_prefix_segment` in
`new_record.rs`), shared by both recognizers. No behavior change; a regression test pins their agreement
so a future edit to one can't silently re-open the drift.

#### `declared_slots_for_roles` (used by three embodiment/relation rules) and `declared_cross_record_value`

(used by every cross-record `Status` read) each rebuilt a type's declared-field set from scratch on
every record/reference they examined, instead of using the per-type cache this codebase already
computes once elsewhere. Both now reuse that cache -- O(types) instead of O(records) or
O(records × references) per `check` run. No behavior change; findings and output are identical.

#### `compute_drifted_records` (the git-history check behind `embodiment.consistency`'s drift detection)

spawned a `git log`/`git merge-base` subprocess per `Realized-by` locator per record, with no caching
across records or locators that share the same path or resolve to the same commit -- on a corpus
where several records cite the same shared file, the same commit history was looked up repeatedly.
Locator lookups and drift comparisons are now memoized per locator path and per commit pair for the
duration of a single `check` run. No behavior change; findings are identical.

#### `relation.supersession-reciprocity` read a resolved target's own field via a plain `.get()` with no

check for whether the target's header was unreadable -- which reads identically to a genuinely absent
field. A record correctly citing a target whose header failed to parse got a spurious "does not
reciprocally name back" finding, blaming it for the target's own unrelated parse failure. The rule now
skips a target with an unreadable header rather than judging it.

#### `urzua migrate schema --report` reported every record of a `header_shape: none` type as `Unreadable`

with a spurious "header did not parse" notice, excluding it from the preview -- such a type has no
header to parse by construction, and can never legally declare the candidate field at all (`ADR-50`).
`schema_report` now gates on the same `has_no_header()` check every other header-parse call site
already uses, and excludes such records from the preview without treating them as a defect.

#### The em-dash "no value" sentinel for pointer/relation fields was duplicated byte-for-byte in two rules

instead of living in one place alongside this project's other shared domain primitives. Extracted into
`values::is_no_value_sentinel`; no behavior change.

#### `MILE-112`'s pre-0.4.0 duplication sweep found and fixed three instances of logic re-implemented in two

or more places, all internal, with no behavior change:

- `record_id` and `filename_number` were independently hand-written copies of the same filename-id
  extraction, differing only in their return shape.
- `pointer_resolution`, `pointer_target_status`, `narrative_field_stale`, and
  `relation_target_status_undeclared` each opened with the identical "resolve a declared field's
  references" guard chain before their own differing per-reference logic.
- Eight config-schema rules (`type_no_declared_spec`, `header_deprecated_shape`,
  `config_pointer_declaration_missing`, `config_known_fields_declaration_missing`,
  `config_pointer_field_not_known`, `config_relation_field_not_known`,
  `config_pointer_narrative_overlap`, `config_header_none_has_no_required_fields`) shared one skeleton
  for examining `Config::record_types` itself.

All three now share one definition each. Full test suite and real-corpus finding count unchanged.

#### `urzua init` swallowed a hypothetical YAML serialization failure into an empty config body, reporting

`written: true` with no error -- if it ever fired, an adopter would get a config with no
`record_types`/`rules` and no sign anything had gone wrong. Changed to fail loudly instead, matching the
same "should structurally never fail" precedent already used for JSON output elsewhere. No input this
function is called with today can actually trigger the failure; this closes the failure mode itself.

#### `urzua init` now proposes the header shape each record type's existing records use, instead of

`yaml-frontmatter` for every type, so the first `check` reads an adopted corpus rather than reporting
every record as having no header. A type whose records mix shapes gets the majority shape and a
`header-shape` notice naming the split; a type whose records parse under no shape keeps
`yaml-frontmatter` with a notice. A deprecated shape is still flagged by `header.deprecated-shape`.

#### A record type's `dir` or `extra_dirs` entry written as `./docs/adr`, `docs/adr/` or `docs//adr` is now a

configuration error that names the canonical spelling (`docs/adr`). It used to load, match nothing and run
as `not-run` with no reason.

#### `urzua graph` and `urzua explain` now report a `population` (`eligible`, `examined`, `unreadable` and

`out_of_scope` records, as `check` does for a rule), and a warning notice lists any record left out because
its header could not be read. Status stays `ok`, so an empty result from an unread corpus is no longer
indistinguishable from an empty result from a read one.

#### Outside a git repository, commands that read one now fail with `urzua needs a git repository: no .git

directory found above the given path` (exit 2). The documented directory-walk fallback never existed and
is removed from the specification; discovery is the tracked set.

#### `config.scope-matches-nothing` no longer warns about records a `sections.exempt_statuses` declaration sets aside:

`section.required` and `section.shape` count them as eligible and unexamined instead of out of scope, so a
declared exemption is not reported as a misconfiguration.

#### `urzua new` writes the H1 id padded exactly as the file name is, so a type whose identity pattern requires

zero-padded numbers no longer gets an unpadded H1 beside a padded file name. Header fields a type's template
lists that the type declares but does not require are kept on the new record, blank.

#### `check`, `audit`, `fix` and `reverify` resolve each `#symbol` scope in a locator once per run and compile each resolver

pattern once, instead of recompiling every pattern and scanning the file twice per locator. On a repository with many
scoped pins a run takes about half the CPU time. Output is unchanged for every run.

#### `SPEC-2`'s documented `## Output contract` example JSON was RFC-3's original, pre-implementation

illustration -- `camelCase` fields, a 4-value `status` enum where the real one has 3, a claimed stderr
rendering removed years ago, `rules_executed` shown as a bare count instead of the real per-rule array,
and a `suggestedAction` field that never existed on `Finding` at all. Rewritten to match the real shipped
shape. Documentation only; no behavior change.

#### `SPEC-5` (`urzua init`) documented a full greenfield mode, `--types`/`--dir` flags, and built-in

profiles as if built -- none of it exists; `init` has one mode (adopt an existing corpus) and two
flags (`--dry-run`, `--config`). That aspirational design is relocated to a new milestone, `MILE-114`,
and `SPEC-5` is corrected to describe only the adopt path that actually ships, now `Accepted`. Two more
inaccuracies found in the same pass and corrected: the Layout section implied `init` writes
`templates/`, `cache/`, and `.gitignore`; it writes only `.urzua/config.yaml` today. Documentation
only; no behavior change.

#### `filename.title-consistency` no longer builds a record number out of the digits of a title's first word.

A title such as `# 2026-09-16 Notes` or `# v1.2 Release` is reported as carrying no record id instead of
claiming the number 20260916. A type's declared `identity` pattern decides the title form (its `prefix`
and a number); a type with no pattern accepts a leading `PREFIX-N` or a bare number. A number too large to
compare is reported as such.

#### `urzua init` no longer proposes `field.quality`, `field.pending`, `header.field-set-consistency` or

`header.pointer-field-clean`. Each reads a per-type declaration (`required_fields`, `known_fields` or
`pointer_fields`) that `init` does not write, so on a freshly adopted corpus they reported `ran` and
examined no record. The report now carries a `rule-applicability` notice naming the four rules and the
declaration each one needs, so the adopter can declare it and enable the rule.

#### The record type that suppresses findings is now declared with `role: suppression` in `.urzua/config.yaml`, so a

corpus can call it anything. Only one type may declare it. A config that declares no such type keeps using the
type named `waiver`, so existing configs behave as before.

#### `urzua check --base` now agrees with a full check in five more cases. Changing or deleting a waiver

reports the finding it stopped waiving on the unchanged record. A comma-joined pointer value such as
`ADR-1,ADR-3` traces each reference when one is deleted. A changed file below a declared directory no
type owns keeps its `type.record-outside-declared-dir` finding. Retargeting a symlinked configuration
puts every record in scope. An empty diff with no rule enabled is `not-run` (exit 2), as a full run is.

#### `check` is organised as separate stages for loading, running rules, scoping and deciding the verdict, with the

verdict a pure function. Output is unchanged for every run.

#### The corpus acceptance report no longer counts a rule that merely ran as a rule that can fire. It

states, per rule, whether the suite has a failing case, a clean run only, or no case, and the
suite gains failing cases for twenty-three rules that had one of the last two. Acceptance scratch
repositories are named by corpus as well as case, so two corpora that share a case id no longer
share a directory. Integration tests run the binary with a stub `gh`, so no test depends on the
machine's login or the network. `make ci` now runs `check --base` and the knope pin check, and
says which CI checks it leaves out. The release workflows install knope through one action, the
release App token is requested with only the permissions the job uses, and the merge queue runs
the diff rules. A release-invariants guard now reads a fragment's level by key rather than from
line two.

#### The `urzua-gate` action now verifies the downloaded binary's build provenance against the release

workflow that published it, not just the repository, and fails with a could-not-run result when its
`working-directory` cannot be entered instead of checking the wrong tree.

#### `citation.path-exists` accepts an optional `skip_revision_log` flag: when `true`, each record's

revision log is not read, so append-only history that names a path as it was is not reported as a
missing file. It is `false` by default, so behaviour is unchanged unless it is declared.
`config.scope-matches-nothing` no longer warns about the citations of an `external_ids` entry of
`citation.resolution`: they are counted as eligible and unexamined instead of out of scope, so a
declared input is not reported as a misconfiguration.

#### A record type's `statuses` declaration accepts an optional `claim_retained` list: statuses reached

only after acceptance, such as `Superseded`, that may still carry an `Implemented` or `Verified`
embodiment claim. `embodiment.requires-accepted-status` allows the claim at those statuses as well
as at the type's `success` statuses.

#### `embodiment.claim-renewal-unattested` accepts a `urzua reverify` row dated any day from the branch's

first commit to the commit that added the row, so a row written on one day and committed the next, a
row amended or fixed up into an older branch commit, and a row whose line a later commit touched are
no longer reported. A date in the future, or before the branch's first commit, is still reported.

#### A genuine `urzua reverify` row is no longer reported by `embodiment.claim-renewal-unattested` once

main has moved on: the row's date is compared with the date of the commit that added it, not with the
merge base's commit date, and a future date is still reported. A back-pointer `ignore_marker` that is
the tail of a record id head followed by a number (`DR-1`, `R-12`) is refused at load, `urzua new`
and `urzua init` numbering read record files with an upper-case `.MD` extension, and claim files
under `claim_paths` are found whatever the case of `.md`.

#### Hardening from a review of the CLI. `check --base` pulls in an unchanged record whose `Realized-by`

glob covers a changed file, so it no longer exits 0 where a full check exits 1. A shallow clone
without the base's history is reported as a shallow clone, with the fix (fetch full history), not as
unrelated histories. `status.transition-allowed` counts a merge-base copy it cannot read as
unreadable, with a notice, instead of as a new record. Records, the config and index artifacts are
written through a temporary file and a rename, so a failed write cannot leave a truncated file.
`fix --apply` and `--rebaseline` refuse an identity containing `|` or a line break. `migrate ids
--apply` reports `partial-failure` when a write fails, takes the `fix` lock and re-reads before
writing. `doctor` reads the global `--config`. The suggested edit for an uncovered back-pointer path
targets the configuration file.

#### A record type can declare pairs of fields that must be declared together with `companion_fields`, a

list of relation role pairs such as `[embodiment_state, embodiment_locator]`. The new
`config.companion-declaration-missing` rule reports a type that declares `Embodiment` without
`Realized-by`, or the reverse, which left records able to claim a state no rule could check.

#### `authority.gates` path entries are read like `published_paths` (`./.agent` gates `.agent`), and an entry

that could match nothing (`.`, a backslash) is a load error, for `diff.decider-required` and the
`protected-paths` guardrail alike. A waiver `Expires` that is not a real calendar date (`2026-99-99`)
expires at once and says so. A re-baseline rewrites only the exact pin it names, `fix` reads a path with
glob characters literally when finding its last commit, and a file-discovery failure while resolving a
locator is reported rather than read as a missing file.

#### An item with a valid waiver no longer fails `embodiment.locator-exists` while its evidence does not

exist yet, as the waiver intends. Messages about the delivery items name the section the type declares
instead of "Done means". A type whose `delivered_tiers` names a tier outside `resolving_tiers` is
refused at load, naming both lists. A locator that resolves outside the repository is no longer
probed on disk.

#### Fixes BUG-91: `doctor`'s `ci-wired` check scans every workflow rather than reading

`.github/workflows/ci.yml` by name, so a repository whose invocation lives in a differently named
workflow is no longer reported as having an unwired checker. A workflows directory that exists but
cannot be read is now an error rather than being reported as unwired. Its `required_fields` warning
also stops claiming header rules will never fire — they do.

#### `drift_grace_days` no longer contradicts a record that already states `Embodiment: Drift detected`.

Inside the window `embodiment.consistency` accepts either the tier value or `Drift detected`, and
`urzua fix` leaves a stated `Drift detected` alone; drift older than the window still computes
`Drift detected`.

#### `urzua explain` on a path that is no file or directory, matches no glob and that no record cites is

now an error (exit 2) instead of an `ok` report with an empty `governing_records`; a rule id given
as a path says to use `--rule`. `urzua init` states what `check` does with records whose headers it
cannot read (nothing, until the type declares `required_fields`) and what to add, and writes
`exclude_files: [README.md]` for a proposed directory that holds a README.

#### `urzua fix --apply` no longer corrupts a record whose `Embodiment` is `null`, `~` or empty. It used to

write the computed value at the start of the line (`ImplementedEmbodiment: null`) and report success;
it now fills the value in place, and the revision row reads "stated no value" instead of an empty span.

#### `urzua fix` computes embodiment drift only when `embodiment.consistency` is enabled, as `check`

does, and both commands run their `git blame` calls in parallel, so a whole-corpus run is
noticeably faster. A shallow clone, or any git call that fails while computing drift, is now
reported as an `embodiment-drift-git` notice instead of reading as no drift.

#### Fixes BUG-92: resets the in-tree version to `0.3.0` and removes a generated CHANGELOG section that

reached `main` from a release-prep branch. No `0.4.0` was ever published, so the next release computes
`0.4.0` from the pending changesets rather than `0.4.1`.

#### The SPEC-22 function-size bound (a function is at most 100 lines) now applies to every non-test

function in the workspace, not only `check.rs`. The `fn_length_ratchet` test measures each crate's
source against a committed ceiling list, `rust/fn-length-ceilings.tsv`, which records the lengths
of the functions that were already over the bound. A function outside the list fails at 101 lines,
a listed one fails when it grows, and a listed one that has shrunk must have its entry lowered or
deleted, so the list can only shrink.

#### The `urzua-gate` action's install step now accepts a downloaded binary only when the version it

reports is exactly the pinned release, so a binary reporting `1.2.35` no longer passes for a pin of
`1.2.3`.

#### A drift-acknowledging `Re-verified` row written just before UTC midnight on a branch whose first

commit is authored after midnight is no longer reported by `embodiment.claim-renewal-unattested`:
the date window starts one UTC day before the first commit. On a branch with no commit yet, the
window starts at the merge base, so a row written yesterday and not yet committed passes a local
`--base` run. The harness `post-command` hook reports a deleted gated record once, not on every
second command, and `check --base` warns when it cannot read a deleted record's base copy.

#### `urzua graph` used the plain, non-collision-reporting record index — unlike `check`/`audit` (since

round 24), it never even computed whether two records shared an identifier, so an edge naming a
colliding identifier could silently point at the arbitrary winner with no way to tell. `graph` now
uses the same collision-aware index builder and discloses any collision found as a `Notice`
(`GraphReport`'s existing `notices` field), regardless of whether `identity.collision` is enabled —
this doesn't override the adopter's rule policy (`identity.collision`'s own blocking `Finding` stays
exactly as opt-in as before); it discloses that the engine's own shared computation had to resolve an
ambiguity, the same class of fix `BUG-125`/`RFC-45` shipped for `Population.unreadable()`.

No adopter-facing behavior change for a corpus with no identity collisions (this repository's own):
verified with the full test suite, clippy, `make ci`, and a real-corpus `check`/`graph` run — `check`
reports the same 70 findings, `graph` emits no new notices.

#### The gate's renderer handles reports over 128 KB (it passed them to `jq` as an argument and failed

with `Argument list too long`, so a corpus with many waived findings lost its pull-request report).
The record-edit hook now judges every finding that needs no git history in a shallow clone and
names the drift rule it could not judge, where it used to allow every edit unjudged; it no longer
suggests a `urzua fix` command for an error the edit itself introduced. `check <path>` reads git
history only for the records in scope, which cuts a one-record check on a few hundred records from
about five seconds to about one. `field.pending`'s built-in match reads `pending` as a whole word,
so a value such as `docs/spending/report.md` is no longer unfinished work. `init` proposes a flat
directory of header-bearing files and flags a proposed directory in which no file has a header.
`new` creates a type's directory for its first record. `doctor` and the hook name urzua commands,
not make targets, and the `filename.identity-pattern` finding names five renames and counts the
rest.

#### `embodiment.locator-on-back-pointer-path` no longer advises dropping the `Realized-by` locator of a

record whose type counts no source comment as evidence (`statuses.delivery.back_pointer_evidence:
false`), and tells a locator whose file already carries the `Implements:` or `Verifies:` comment to
drop the locator instead of adding the comment. `closed_statuses` (`claim.status-agreement`) and
`terminal_statuses` (`narrative-field.stale`) are optional once every record type either declares
`statuses` or has no status field.

#### A YAML frontmatter header is now read as written: a version such as `1.10` or a number such as `0x1F` is no longer

rewritten to `1.1` or `31`, a leading byte-order mark no longer hides the header, and a non-string key keeps its
text instead of vanishing. `urzua migrate schema --report` judges a field the way `field.quality` does for types
whose placeholders come from their template. A refs snapshot with unsorted numbers is refused, `urzua new` stops
with an error when no record number is left to issue, and `urzua fix` reports the real reason when it cannot
create its lock file.

#### `embodiment.locator-exists` reports `Realized-by` text that is not a `spec:`, `code:` or `test:` entry

instead of ignoring it. `header.field-set-consistency` suggests renaming a key that differs from a
declared one only in case, never removing it. `date.field-valid` rejects a `date_fields` entry no
record type declares. `revision-log.change-class-required` reads markdown alignment rows
(`|:---|:---:|`) as delimiters, and `header.required-fields` no longer judges types that declare
no required fields.

#### Hardening, four decided items. `identity.open-pr-collision` now tells the reader to rename the file

and its H1 to the next free number, or to waive the finding when the other pull request is abandoned,
instead of pointing at `urzua migrate ids`, which cannot renumber. `fix` without `--apply` honours
`--ids` (an entry that matches nothing is a failure, as under `--apply`) and rejects `--force`, `--by`
and `--actor` as usage errors instead of ignoring them. A waiver's required field counts as a
placeholder only when its whole value is a declared pending value or the template's text, so a
`Reason` such as "pending the upstream fix" takes effect, and a placeholder is reported as a
placeholder, not as missing. A name under `harness.guardrails` that this build does not ship is a
configuration load error for every command, with the nearest valid name suggested, and `doctor` no
longer reports "no unrecognized keys" beside an unrecognized harness entry.

#### The agent harness closes the gaps the fifth review found. `urzua harness run post-command` now

reports a move to a gated status even when the same command removed or narrowed the status gate or
switched `acceptance` off, and reports that loss (`session-start` does too); a new session forgets
what an earlier one was told, so a change made again is reported again. The Claude Code translator
reads a path used as a dictionary key and a `file:` URL in any case, no longer drops strings beyond
the 64th (a call with more than 512 distinct strings or files, or more than 8 files checked as records, is
refused as unjudged under `on_unjudged: deny`), judges the acceptance and protected-path gates of
every file in an MCP call in one engine process, however many files, before it checks any record and
stops at the first block (a call with 2200 files no longer outlasts the hook timeout), and the edit
hook sets a 120 second timeout. The neutral request gains `gates_only` and `contents` and the verdict `record` and `records`. A
status gate whose decider group is changed is reported as the move and as a change to the
configuration, not as a shrunk gate.

#### The agent harness no longer lets an edit through when its hook command runs from a directory other than

the project root: the settings command finds the project from `CLAUDE_PROJECT_DIR`, else from the
outermost `.urzua/config.yaml` above the current directory, and when it cannot run the translator it
refuses the edit where `acceptance` or `protected-paths` is on with `on_unjudged: deny`. An edit is judged
by the outermost governed repository, so a nested git repository no longer takes its files out of the
gates, and a write through a symbolic link whose target does not exist yet is judged as the file it would
create. The edit tools are refused for the files the harness keeps (`.urzua/cache`, `.urzua/harness.lock`,
the override log, `.git/hooks`, `.git/config`). MCP tool calls are read for every path-like string at any
depth (`from`, `source`, `uri`, `files[]`). The session-start check compares the installed files and hook
entries with the lock and reports `disableAllHooks`. The post-command check lists a record hidden with
`git update-index --assume-unchanged` or `--skip-worktree`. A settings file nested more than 128 levels is
refused instead of overflowing the stack.

#### Fixes `BUG-128`: `config.header-none-has-no-required-fields` only forbade a `header_shape: none` type

from declaring `required_fields` — the same contradiction applies to `known_fields`, `pointer_fields`,
`narrative_fields`, and `relation_fields`, none of which had an equivalent guard. A type declaring
`header_shape: none` and `pointer_fields: ["Parent"]`, for example, previously passed config
validation and then had every one of its records misleadingly reported as `Outcome::Unreadable` by
every rule reading that slot. The same rule now checks all five field-declaration lists.

No adopter-facing behavior change for a config that doesn't hit this combination (this repository's
own config has no `header_shape: none` types declaring any of the four): verified with the full test
suite and `make ci`.

#### Fixes two silent-corruption risks from ADR-57's exact-match `Header::get`. `waiver`'s `Rule`/`Scope`/

`Expires` and `migrate ids`'s `Stable-Id` are the engine's own reserved keys, never adopter-declared
vocabulary — there is exactly one field named `Stable-Id`, so a record spelling it `stable-id` has made
a typo, not declared a different field. Reading them exactly let a waiver written with lowercase keys
be silently dropped from the active set, and let `migrate ids --apply` assign and write a second,
conflicting `Stable-Id` onto a record that already had one under different casing. Both now match
case-insensitively via a new `Header::get_reserved`, kept separate from `get`'s adopter-vocabulary exact
match. `Status`/`Embodiment`/`Realized-by` are unaffected: they are adopter-declared `known_fields`
vocabulary and ADR-57's exact match is the intended behaviour for them.

#### Correct two lines of `--help`. `urzua new --help` said the numbers it skips are named on stderr; they

are listed in the `skipped` field of its JSON, not on stderr. The `--as-of` help
omitted `waive`, which accepts it in its read-only preview.

#### A panic now exits with code 2, the documented code for "could not run", instead of 101, and a closed

stdout pipe (`urzua check | head -1`) ends the run quietly instead of panicking. `urzua new` fills a
record's date and author from the field names a type declares under `relation_fields.date` and
`relation_fields.author` (defaulting to `Date` and `Author`), so a type that calls them `Created` and
`Owner` gets them filled instead of `null`.

#### A record type's `statuses` declaration accepts an optional `implementable` list: the statuses at

which implementation of the record is permitted, which must be among its `values`.
`embodiment.back-pointer-decision-accepted` now judges a back-pointer against that list instead of
the type's terminal `success` statuses, falling back to the rule's `accepted_statuses` option and
reporting the status as unjudgeable when neither is declared.

#### `urzua init` disambiguates a proposed type name when two directories share the same last path

component (`docs/adr` and `legacy/adr` would both propose `adr`), rewriting the colliding ones to
their full, hyphen-joined path. The collision check itself only ever compared base names, never the
disambiguated name it produces -- so a disambiguated name could still collide with an unrelated
directory that already happened to be named that, and the generated config's `Mapping::insert`
silently overwrote one type's entry with the other's. An entire directory's records could disappear
from `.urzua/config.yaml` on the very first `init` run, with no error and a config that still loaded
and passed cleanly.

`init` now checks the fully-disambiguated names for a second collision and refuses (exit 2) rather
than silently dropping one, naming both colliding directories and the shared name.

#### The `urzua init --dry-run` help and documentation say that the config a dry run proposes is

byte-identical to the file the real run writes, and that the report differs in `dry_run`,
`written` and in carrying the config text only on a dry run.

#### Fixes `BUG-124`: the new `config.known-fields-declaration-missing` rule (`RFC-43`/`ADR-62`) flagged a

`header_shape: none` type for not declaring `known_fields`, even though such a type has nowhere for
any header field to be (`ADR-50`) and `known_fields` is meaningless for it. Now exempted, matching
`config.header-none-has-no-required-fields`'s existing reasoning for `required_fields`.

#### `urzua check` now reports a record of a `header_shape: none` type as out of scope, rather than as

unreadable or unexamined, in every rule that reads headers. Twelve rules previously counted such
records as `unreadable` in their population (`header.field-set-consistency`,
`header.pointer-field-clean`, `field.untrimmed-value`, `header.field-case-mismatch`,
`pointer.resolution`, `pointer.target-status`, `narrative-field.stale`, the three `embodiment.*`
rules, `relation.supersession-reciprocity` and `relation.target-status-undeclared`), implying a
malformed corpus where the configuration declares no header at all, and `header.layout-consistency`
counted them as `absent` (eligible but not examined) when the type declared a `header_layout`.

#### Finishes the corpus acceptance harness (`MILE-170`, `SPEC-4` 0.6): cases can make further commits, pass

`--base` and run in a shallow clone, and every shipped rule now has an acceptance case that fails when the
rule is switched off. New cases cover git-history drift (a shallow clone is reported, never read as no
drift), the diff-gate rules, the published-path boundary, `citation.issue-exists` against a snapshot,
`status.delivery-agreement` and the permanent ceiling of structural checks. No change to `urzua` itself.

#### The commands that read the corpus now share one startup path (repository root, config, tracked-file

discovery, record loading) and one could-not-run report. No behavior change: every command's JSON
output and exit codes, including the exit-2 could-not-run results for a missing repository, a missing
config and an unreadable record, are identical.

#### This repository's `spec` record type now declares a `Shipped` status after `Accepted`, and requires `Embodiment` and

`Realized-by` of a spec in that status. It is configuration only: the same `statuses.require` declaration any type can
use, with no change to the CLI's behaviour for other configurations.

#### `urzua harness run edit` now applies the protected-path and acceptance gates to every path of an edit request that names

several paths, so an agent cannot write a protected file by listing it beside another. With the editor-hook guardrail on,
an agent's request with several paths is also refused when a guardrail set to `on_unjudged: deny` covers one of the
paths. A person's request with several paths, and an agent's that nothing gates under `on_unjudged: allow`, is judged
as before. Requests with one path are unchanged.

#### `urzua new` reports the numbers it passed over in its JSON output instead of on stderr. The report gains

a `skipped` list, always present, with one `{number, reason}` entry for each number and reason: `open-pr
<ref>` for a number held by a pull request's head ref, `worktree <path>` for one held in a sibling
worktree, and `local` for one held by a local branch or remote-tracking ref. The `urzua: skipped N` lines
are gone. When git cannot read the other refs, `new` numbers from the working tree as before and says so in
a warning notice (subject `new.numbering`) rather than on stderr.

#### Adds the census: a rule declares the population it will judge as a candidate list, and `eligible` is

that list's length rather than a separate count kept beside the loop. `field.quality` and
`field.pending` report `field 1017/1017` on this repository — the same number `records_examined` has
been reporting against a 307-record corpus, now carrying the unit it was always counting.

#### The five record-scoped rules report a population. `revision-log.change-class-required` now shows

`eligible: 307, examined: 236` on this repository — 71 records carry no revision-log marker, which
`BUG-50` records as indistinguishable from compliance. The report performs that subtraction for the
first time. `header.layout-consistency` reads `eligible: 0`, which is a rule with nothing in scope
rather than a rule whose matcher is broken.

#### `rules_executed` entries begin carrying a `population` — what a rule was eligible to examine and what

it judged, in a named unit. The six configuration rules and `type.record-outside-declared-dir` report
it first: a config rule now says `record-type 6/6` rather than `records_examined: 6`, which claimed a
quantity it was not counting. `records_examined` stays authoritative until every rule is converted.

#### `embodiment.claim-renewal-unattested` says an anchor field "is empty" when the branch left it

blank, and keeps "was removed" for a field that is gone. An unreadable merge-base copy of a deleted
record is warned about once per path, under the rule that reached it, instead of twice under
`diff.decider-required`.

#### Adds the first slice of `SPEC-4`'s acceptance suite (`MILE-101`): a seeded, hand-rolled generator over

an alphabet of characters that have broken case-insensitive comparisons, and two properties checked
against it. The declared-field matcher is extracted as `rules::field_is_declared`/`fold_field_name`, so
the suite tests the matcher the rules use rather than a restatement of it. No rule behaviour changes.

#### `urzua harness run post-command` now reports a command that removes the configuration file, or any

other path, from the `authority` path gates. A changed protected path is judged against the gates the
session started with as well as the edited configuration, so the removal is reported as a changed
protected path and as a shrunk protected set, and files under a removed gate are still compared.
`urzua harness run session-start` reports a protected set that shrank since the previous session
before it takes a new baseline.

#### The installed record helper (`urzua-record.sh`) no longer occasionally omits its `Pinned:` line, or

reports "nothing was re-pinned", after a successful `reverify` or `repin`. It tested the engine's
JSON through a pipe into `grep -q` under `pipefail`, where `grep` exiting on its first match could
turn a match into a failure; it now reads the text directly.

#### Adds a mechanical guard (`crates/urzua-cli/tests/relation_field_literals.rs`) against

`BUG-118`/`BUG-119`/`BUG-120`'s recurring shape: a fix to `RFC-42`/`ADR-61`'s declared relation-field
names landed wherever a reviewed diff touched, while a sibling call site elsewhere in the tree kept
reading the old literal, three times in one review round. The new test walks every source file in
`urzua-core`/`urzua-cli` and fails if `"Status"`, `"Embodiment"`, `"Realized-by"`, or
`"Supersedes / Superseded-by"` reappears as a bare string literal in production code outside
`config.rs`, where `RelationRole`'s own defaults are declared. Ships with planted-violation cases,
verified to actually fail against the real shape of all three bugs before being restored.

#### `config.relation-field-not-known` also reports a `relation_fields.date` or `relation_fields.author`

that is not in the type's `required_fields` or `known_fields`, as it already did for the relation roles.
Which rules accept which options is read from the rule table; the load errors are unchanged.

#### Fixes `BUG-118`, `BUG-119`, `BUG-120`: `RFC-42`/`ADR-61` made `Status`/`Embodiment`/`Realized-by`/

`Supersedes / Superseded-by` adopter-declared per type for the seven rules `urzua check` runs, but three
other consumers of the same fields were left reading the old literals directly — `urzua fix`
(`detect_repairs`), `urzua explain`/`urzua graph` (`explain`, `graph`), and drift detection
(`compute_drifted_records`). A type declaring a `relation_fields` override got `check` correctly judging
it while these three commands silently disagreed. All three now resolve the same
`relation_field_name`/`RelationRole` accessor `check`'s rules use.

Also: `config.relation-field-not-known` is now enabled in this repository's own config (it was added to
`ALL_RULES` in the previous round but not turned on here, unlike its sibling
`config.pointer-field-not-known`); `RelationRole` gained an `ALL` constant and `RelationFields` a `get`
accessor so a role-enumerating loop can't silently miss a future 5th role; the two "declared alias not
known" rules now share one `Finding`-construction helper; and `SPEC-2`'s rule table is resynced from
twenty-one to its real twenty-nine rows (`BUG-52` remains open as the actual missing-mechanism gap this
manual sync doesn't close).

No adopter-facing behavior changes for a config that declares no `relation_fields`: verified with the
full test suite, clippy, `make ci`, and a real-corpus `check` run reporting the same 70 findings before
and after.

#### Fixes BUG-93: `make ci` checks two facts about the release that nothing read before — that the

manifest version matches the newest tag while changesets are pending, and that a changeset describing
a breaking change declares `major`. Pre-1.0 a `minor` fragment ships as a patch, so the second one
would otherwise release a config-breaking change with no Breaking Changes section.

#### `check`'s staged loading is split into named helpers, each short enough to read on its own, with no

change to what `check` prints. The staged functions in `check.rs` are held to a stated size bound
by a test, and the output of every command is pinned by golden tests.

#### `urzua new` fills the field a record type names under `relation_fields.title` with the record's title, and the `revision_log.version_bump` field with `version_bump.initial`, so a record whose required fields hold nothing else the tool cannot fill, such as a spec's `Subject` and `Version`, passes `check` as written. Both keys are optional; a type that declares neither starts those fields blank as before. This repository's `spec` type declares them.

`type.dir-matches-nothing` also reports each status a type's `sections.exempt_statuses` lists that none of the type's records holds, naming the type and the status, so an exemption that sets nothing aside is no longer silent. The suggested action is an `edit-config` `set` on the type's `sections.exempt_statuses`.

#### A back-pointer `ignore_marker` that would sit on every back-pointer line is now refused when the configuration loads:

one with fewer than 3 non-whitespace characters, one made only of comment leaders or punctuation (`//`, `#`), and one
that is contained in a declared record id head, so `ADR-` and `ADR` alone are both refused. A specific marker such as `urzua:allow` loads as before.

The revision row `urzua fix --rebaseline --apply` writes now ends with a rationale that names who re-verified the
claim: an agent (to be confirmed by a reviewer in the PR), a person, or the attesting actor when none was declared.

#### A record of a type that declares `revision_log: {required: true}`, with no revision-log section, is now a finding of

`revision-log.change-class-required`, with the `edit-section` `add` action naming the section to add.
Before, a record that lost its log left the rule's scope and the check stayed green. A type that does not declare it is unchanged: its records without a log stay unexamined.

#### Fixes BUG-87: a record whose filename contains a non-ASCII character is part of the corpus. git

C-quotes such paths under its default `core.quotepath`, so the name arrived wrapped in quotes, its
parent never matched the declared `dir`, and the record was dropped — a planted violation was
invisible and the run exited 0. Discovery reads git's output NUL-separated.

Fixes BUG-89: a `claim_paths` symlink pointing at an ancestor of the declared prefix is refused by
name rather than followed, so the walk cannot sweep the whole repository and report blocking findings
about files never declared as claims.

Fixes BUG-90: `claim.status-agreement` counts the records a claim resolved against, not the claim
files it read, so its number can no longer exceed the size of the corpus.

#### Fixes BUG-94: a rule reporting that it was handed nothing no longer certifies the corpus `ok`. The

population added this release said `eligible: 0` while the same report said `status: ok`, because the
gate read only the older `scope` field.

Fixes BUG-95: `field.quality` and `field.pending` report `records_examined` as a record count again.
They had changed its unit to field slots in the same release that documents the field as
record-shaped, so one entry contradicted itself — `6` beside `scope: records` on a two-record corpus.

Fixes BUG-96: the release invariant guards run in CI. They were wired into `make ci` only, which fires
when a human runs it — not the unattended path both defects they guard against actually took. An
undeterminable tag is now a failure rather than being read as agreement.

#### `header_pointer_field_clean` and `narrative_field_stale` were missed by an earlier pass consolidating

duplicated `(record, field)` slot construction into the shared `field_slots` helper; both now use it,
matching every other `Field`-unit rule. A comment in `new_record.rs` using temporal language against
the project's comment convention is reworded to state the invariant directly. Both found by a
`/code-review v0.3.0...main` pass; no adopter-facing behavior change, verified with a real-corpus
`check` run reporting the same findings before and after.

#### Fixes `BUG-111`: `parse_record_filename` supports a hyphenated type prefix (`DOC-ADR-2-x.md`), but the

reference recognizer used by `claim.status-agreement`, `pointer.resolution`/`narrative-field.stale`,
and `header.pointer-field-clean` split on only the first hyphen, so a citation of a hyphenated-prefix
record (`DOC-ADR-2`) was rejected as not-a-reference everywhere it was written. One shared predicate
now matches `parse_record_filename`'s own algorithm (last segment is the number, everything before it
is the prefix) at all three call sites.

Fixes `BUG-112`: the date-vs-record-number filename heuristic required zero-padded month/day segments,
so `2026-9-19-notes.md` (a genuine, just-unpadded date) parsed as record `2026` instead of being
recognized as a date.

Fixes `BUG-113`: `Population::detailed`'s `examined <= eligible` invariant was checked with
`debug_assert!`, which compiles to nothing in a release build — the exact profile `make ci` and an
adopter's own CI run. Now a real `assert!`.

Files `BUG-110` (`Status` is a hardcoded field-name literal rather than adopter-declared vocabulary,
needs a config-schema decision) without fixing it.

No adopter-facing behavior changes beyond the three fixes above: verified with the full test suite,
clippy, `make ci`, and a real-corpus `check` run reporting the same findings before and after. Four
other candidates from this review pass were investigated and refuted as deliberate, already-decided
design (untrimmed `Status` comparisons, a flat `terminal_statuses` config replacing a hardcoded
per-type table, `declared_slots`' pre-population type-declaration filter, and a hardcoded em-dash
placeholder check) or as premised on something that can't happen (an old `schema_version` config
loading silently rather than failing loudly).

#### Fixes `BUG-114`: `parse_record_filename`'s prefix check accepted only ASCII-uppercase letters, so a

digit-bearing type prefix (`V2`, `S3`) never matched, silently breaking `urzua new`'s duplicate-number
protection for such types. Digits are now allowed alongside uppercase letters in a prefix segment; the
number segment can't be confused with it, since it's already selected as the first all-digit segment.

Fixes `BUG-115`: `check.rs` called `compute_drifted_records` (which shells out to `git` per
`Realized-by` field) unconditionally, even when `embodiment.consistency` — its only consumer — is
configured `Off`. It now runs only when that rule is enabled.

Fixes `BUG-116`: `identity.collision` rebuilt the shared normalized-record index independently instead
of reusing the one `check.rs` already builds for five other rules, doing the same O(corpus) traversal
twice per run. `build_index_reporting_collisions`/`IdentifierCollision` are now `pub`, and
`identity_collision` takes the pre-computed collisions list as a parameter.

Fixes `BUG-117`: the `claim_paths` symlink-ancestor guard (`prefix_canonical.starts_with(&resolved)`)
was reflexive, rejecting a symlink that resolves to exactly the declared prefix root as if it were a
genuine ancestor, even though `visited` already dedupes it safely. Now requires
`resolved != prefix_canonical` in addition.

No adopter-facing behavior changes beyond the four fixes above: verified with the full test suite,
clippy, `make ci`, and a real-corpus `check` run reporting the same findings before and after. One
other candidate from this review pass (`header_field_case_mismatch` vs. `near_miss`'s inline
near-miss logic) was investigated and found not to be a real duplication risk — both already share
`FieldName`'s single `Ord` impl, so they cannot diverge — and one (a temporal-language comment quote)
was refuted as a hallucinated citation not present in the file.

#### Fixes a real nondeterminism defect found by review: a cross-type record-identifier collision's

first-seen-wins index slot depended on `HashMap` iteration order (randomized per process), so which
of two colliding records other rules resolved against could flip between runs of an unchanged
corpus. `Config::sorted_type_names()` is now the one shared, sorted way every `record_types`
iteration site gets its type list, replacing 11 independently hand-written `collect()`+`sort()`
call sites and 3 that iterated unsorted.

Also: a comment in `config.rs` narrating its own prior wording (rather than stating the current
invariant) rewritten to just state the invariant; an algebraic no-op in `new_record.rs`'s filename
parser simplified; `report.rs`'s `census`/`census_records` tallying logic de-duplicated (the latter
now implemented in terms of the former); and `check.rs`'s two near-identical symlink-containment
checks in `claim_paths`' walk consolidated into one shared `resolve_inside_repo` helper.

No adopter-facing behavior change: verified with the full test suite, clippy, `make ci`, and a
real-corpus `check` run reporting the same 70 findings before and after. The `HashMap`-ordering fix
was additionally verified empirically across 20 separate process invocations of the pre-fix binary
against a real collision fixture (13/20 resolved one way, 7/20 the other) versus 20/20 stable on the
fixed binary — the nondeterminism can't be captured as a classic single-process observed-failing
test, so this round's regression test instead asserts the fix's actual guarantee directly.

Files `BUG-129` (left `Open`, not rushed): `parse_realized_by` truncates a `Realized-by` locator path
at a literal comma before checking for the category delimiter, since the format splits on `,` before
`:`. Fixing it needs a delimiter/escaping decision this changeset doesn't make.

Two additional "duplication" findings from the same review were investigated and refuted: both
`scan_references`/`extract_references`'s reference-token check and `config.rs`'s rule-option
validation were already fully consolidated through a single shared function/table in the current
code — the review described an already-fixed prior state, not the code as it stands.

#### Fixes three real defects found by review:

- `RecordId::normalize` split at the first hyphen, not the last, so a multi-segment type prefix
  (`DOC-ADR`, already a supported shape per `BUG-111`/`BUG-114`) never matched a zero-padded and
  unpadded reference to the same record as one id.
- `is_record_reference` still rejected a digit-bearing prefix segment (e.g. a type declaring
  `prefix: "V2"`), even though `BUG-114` already loosened `parse_record_filename` to accept one — a
  mention of such a record in prose or a pointer field never resolved.
- `check.rs`'s `claim_paths` pre-flight validation re-derived its own symlink-containment check
  instead of using the `resolve_inside_repo` helper defined specifically so this check has one place
  to land; both now share a `path_is_inside_repo` predicate.

No adopter-facing behavior change beyond correctness: verified with the full test suite (each fix has
a planted-violation test, observed failing before and passing after), clippy, `make ci`, and a
real-corpus `check` run reporting the same 70 findings before and after.

Three more findings from the same review were investigated and refuted or filed rather than rushed:
`records_read_by_any_rule`'s report-scope vs. each rule's whole-corpus population is deliberate
(`BUG-67`); `init`'s independence from `docs/` is deliberate (`BUG-36`). `BUG-130`
(`duplicate_keys`'s exact-match reach), `BUG-131` (the identity index's undisclosed first-seen-wins
resolution when `identity.collision` is off), and `BUG-132` (`init`'s directory-disambiguation missing
a second collision check) are filed, left `Open` — each needs a real design decision, not a rushed fix.

#### Round 26's `/code-review` found and fixed three more defects, and filed one design-decision item:

- `field.quality`, `field.pending`, and `header.required-fields`'s own per-slot population loop
  misclassified a `header_shape: none` type's declared slots as `Unreadable` instead of `OutOfScope` --
  the same defect `BUG-137` fixed in `migrate::schema_report`, recurring in three more places (`BUG-142`).
- `relation.target-status-undeclared` rebuilt a type's declared-field set from scratch per reference
  instead of reusing the per-type cache every sibling rule already uses -- a missed sibling of `BUG-134`
  (`BUG-143`).
- `pointer.target-status` and `relation.target-status-undeclared` duplicated the same resolved-target
  walk; extracted into one shared helper (`BUG-144`).

All three: no behavior change, full test suite and real-corpus finding count unchanged.

Filed, not fixed: two hand-maintained rule-id allowlists carry the same drift risk the project's
`RuleOption` table was built to eliminate, but unifying them needs a real schema decision (`BUG-145`).

#### Round 27's `/code-review` found and fixed four more defects:

- `header.field-set-consistency`, `header.pointer-field-clean`, `field.untrimmed-value`, and
  `header.field-case-mismatch` misclassified a `header_shape: none` type's declared slots as
  `Unreadable` instead of `OutOfScope` -- the same defect `BUG-137`/`BUG-142` fixed elsewhere,
  recurring in four more rules (`BUG-147`).
- `header.pointer-field-clean` rejected the `—` no-value placeholder when mixed with a real reference
  in the same field (`Derives-from: RFC-1, —`), contradicting its own documented contract (`BUG-148`).
- `IDENTITY_DEPENDENT_RULES` omitted `relation.target-status-undeclared`, so `init` could propose it on
  a prefix-less corpus even though it resolves references through the same identity-dependent mechanism
  as its listed sibling (`BUG-149`).
- `discover_tracked_files` spawned a redundant third `git` subprocess on every invocation; merged into
  the second call's own richer output, with a staged rename's two-path-field shape explicitly handled
  and tested (`BUG-150`).

Two round-27 findings verified and refuted: a proposed status-comparison trim would have reversed an
already-decided, tested policy (`ADR-57`'s exact-match extends to status comparisons by design); a
population-eligibility change from `RFC-42`/`ADR-61` is deliberate, not an oversight.

All fixes: no behavior change beyond the corrected defect itself; full test suite and `make ci` pass
throughout.

#### `fix --apply` and `migrate ids --apply` now change only the lines they mean to: a CRLF record keeps its

line endings, a file with no trailing newline stays without one, and `fix --apply` adds its revision-log
row inside the table whether the log is a blockquote table or a plain one. `fix --apply --ids <entry>`
now reports an entry that matches no pending repair as a failure (exit 1) naming it, instead of a silent
`ok` that wrote nothing. `urzua new` refuses a title with no ASCII letters or digits instead of creating
a slug-less `ADR-N-.md` whose number the next `new` would reissue.

#### A back-pointer `ignore_marker` padded with a keyword or an id head, such as `// Implements:`, is

refused at load: it would excuse every back-pointer line. A Done-means item that writes its waiver
clause before its evidence clause on the first line is reported, as it already was on final lines,
instead of the evidence being read as part of the waiver reason. A pipe character in a path written
into a revision-log row is escaped so the row keeps its columns.

#### Fixes `BUG-125`: a rule's silence about `Outcome::Unreadable` (a header present but not parseable)

used to depend on `header.required-fields` being separately enabled to surface it — one opt-in rule's
honesty depending on a second, independently opt-in rule, violating "rules should not depend on other
rules." `Population` now discloses its own `unreadable()` count directly, and the fix lives in the
shared `census`/`census_records` machinery, so it covers every current and future rule that reads
`record.header` with no exceptions (verified programmatically against every such rule in `rules.rs`).

`RFC-45`/`ADR-63`/`MILE-111` also adds `relation.target-status-undeclared`: a resolved pointer or
narrative reference whose target type never declares `Status` is now flagged by one dedicated rule
(matching `header.field-case-mismatch`'s precedent), rather than `pointer.target-status` and
`narrative-field.stale` each inventing its own version of the same check. `claim.status-agreement`
resolves claim-sourced references, a different source the new rule does not cover; it still silently
skips a target whose type doesn't declare `Status`, unchanged and out of scope for this round.

Fixes `BUG-126`: `type.record-outside-declared-dir` no longer warns about a record staged for
deletion that sits below (not directly in) a declared dir.

`BUG-127` (a suspected symlink-argv defect in `resolve_argv_overrides`) was investigated and re-graded
`Not a bug`: `relative_scopes` already canonicalizes — and thus dereferences — every argv path before
`resolve_argv_overrides` runs, so the reported defect is not reachable through the real CLI call
chain. No behavior change; the vacuous regression test for it is removed.

Also enables `config.header-none-has-no-required-fields`, an already-built rule that was never turned
on in this repository's own config, found via an audit comparing `ALL_RULES` against the enabled
`rules:` block.

No adopter-facing behavior change to existing findings: verified with the full test suite, clippy,
`make ci`, and a real-corpus `check` run.

#### `urzua rules` lists the rules in the order `urzua check` runs them and reports them in

`rules_executed`, with `config.scope-matches-nothing` and `waiver.matches-nothing` last.

#### A path argument and `--base` now keep or drop each finding by what its rule reports on, declared once

on the rule, instead of by a list of rule ids kept in `check`. No finding is kept or dropped
differently.

#### Fixes `BUG-121`: `check.rs`'s path-scope filter dropped a `claim.status-agreement` finding whenever

the claim file (under `claim_paths`) fell outside the requested `check <path>` scope, even though the
claim was genuinely checked — the same defect `BUG-86` documented but only worked around at this
repository's own Makefile caller, never fixed in the filter itself. A new declared list,
`rules::RULES_REPORTING_OUTSIDE_THE_CORPUS`, exempts `claim.status-agreement` findings from the
path-scope filter the same way a config-file finding already is exempted.

No adopter-facing behavior change for unscoped `check` (this repository's own invocation): verified
with the full test suite, clippy, `make ci`, and a real-corpus `check` run reporting the same 70
findings before and after. A new integration test exercises the previously-broken case directly: a
scoped `check docs/adr` over a false claim under `changes/` now correctly reports and blocks.

Three other candidates from this review pass were investigated and refuted as deliberate,
already-decided design: `supersession_reciprocity`/`embodiment_consistency`'s declaration-gating
(`ADR-53`'s "declared, not voted" principle, already the pattern throughout this release),
`header.rs`'s exact-case duplicate-key comparison (`ADR-57`, whose own doc comment anticipates and
answers exactly this scenario), and the TOML-to-YAML config break shipping no migration diagnostic
(`ADR-54`, decided with measured adoption evidence). A reported code-duplication finding (six rule
functions allegedly not sharing a slot-construction helper) was also refuted: five of the six already
use the shared `field_slots` helper: only `header_required_fields` doesn't, deliberately, for
record-scoped rather than field-scoped findings.

#### A scoped `check` no longer reports `status: not-run` beside `blocking: true`. A config finding

survives every scope filter, so scoping to a path holding no records produced a report claiming both
that nothing ran and that a blocking error was found, exiting 2 where 1 was correct.

`records_read_by_any_rule` is now counted over the same set as `files_examined`. Rules run against the
whole corpus so a reference resolves outside the scope, but counting those records here put the two
numbers on different denominators — a scoped run reported reading four records while examining one.

#### Make `urzua check <path>` read only the source files the back-pointer rules need. A run with a path

argument and no `--base` reads the files under the requested paths, and any other file whose
`Implements:` or `Verifies:` comment cites a record in scope, for `embodiment.back-pointer-resolution`,
`embodiment.back-pointer-decision-accepted` and `code.dangling-bug-reference`, instead of every file
under `back_pointer_paths`. On a benchmark repository of 2,361 tracked files a single-record run went from 4.61 s to
1.91 s; on a small repository the run takes about 59% of a whole-corpus run's time, and the saving grows
with the number of source files, so it is much faster on large repositories. Its
findings are a whole-corpus run's restricted to those paths, plus the decision findings about the records
in scope; the files it did not judge are counted in each rule's population as `deferred_by_scope`, with
an `info` notice. A run with no path argument and a `--base` run read every file, as before.

#### `pointer.resolution`, `pointer.target-status`, `narrative-field.stale`, `claim.status-agreement` and

`relation.supersession-reciprocity` each rebuilt their own normalized id-to-record index independently
on every `check`/`audit` run — the same O(corpus) work done five times over. `check` and `audit` now
build one shared index per run and pass it to each rule. Verified as a no-op: findings, `files_examined`
and `records_read_by_any_rule` are unchanged on this corpus. `relation.supersession-reciprocity`'s own
index used to resolve a duplicate-id collision last-wins; the shared index resolves it first-wins
instead, which changes nothing observable since `identity.collision` already reports any such collision
on its own.

#### `urzua new` always writes a `Stable-Id` and reports it in `stable_id`; when the record type's closed

`known_fields` does not list `Stable-Id`, it adds a warning notice naming that configuration
inconsistency. `type.dir-matches-nothing` now also warns about an `exclude_files` entry that matches
no file directly in the type's `dir`, so a stale exclusion is visible.

#### Fix `diff.decider-required` and `status.transition-allowed` pairing a new record with a deleted one that merely

shares its `Stable-Id`. A record the base lacks is now judged against a deleted record only when the record id
(type and number) also matches, so a slug rename still keeps its status while a new Accepted record that copies a
deleted record's `Stable-Id` is a gated move that needs a decider.
The deleted copy is read with the type's declared document model, so a type that declares its own `identity` pairs
its renames too.

#### `status.transition-allowed` now follows a renamed record. A record renamed or moved in the change under

`--base` is matched to its copy at the merge base by `Stable-Id` and judged against the status it had
there. Before, a rename read as a new record: renaming an `Accepted` decision gave a false error, and
renaming one while resetting its status to an initial one passed. A record with no `Stable-Id` still
reads as new.

#### Every date `urzua` compares is now a UTC date. A waiver's `Expires` and the date `urzua refs refresh`

writes into the snapshot were read from the machine's local clock, so a waiver could lapse a day
earlier or later depending on the contributor's time zone; claim ages already counted from UTC.

When `config.pointer-field-not-known`, `config.relation-field-not-known`,
`relation.target-status-undeclared` or `config.companion-declaration-missing` names a record type that
declares no `known_fields`, the suggested `edit-config` now sets the full list of fields that type's
records use, plus the field in question, instead of appending one field. Appending created a one-item
list, which turned `header.field-set-consistency` on and flagged every other optional field.

`urzua new` keeps the generated `Stable-Id` for a record type that lists `Stable-Id` under
`required_fields`; it was overwritten with a blank.

#### `section.shape` with the `y-statement` shape no longer reports a phrase as missing when the block is

a blockquote and the phrase wraps across lines: the leading `>` on each line is ignored before
matching.

## 0.3.0 (2026-09-16)

### Breaking Changes

#### `check` and `audit` emit the declared `scope.source` value, not a Rust `Debug` rendering

`scope.source` came back as `"GitTracked"` — the `Debug` rendering of an internal enum, a value that
appears in no specification. SPEC-2 declares four values for this field, and **no run ever produced
one of them.** An agent branching on `scope.source == "tracked-sweep"`, exactly as the spec
documents, took the wrong branch on every invocation.

It now emits `"tracked-sweep"`.

The field is a serde-renamed enum rather than a `String`, so renaming an internal type can no longer
change published output silently. Only values the tool can actually emit exist on that enum — SPEC-2's
table now marks which of its four declared values a run produces today, so a consumer is not misled
into branching on a mode that isn't built.

**Breaking** for any consumer matching the old `"GitTracked"` string. Nothing could have matched it
correctly against the documented contract, since it was never a documented value.

## 0.2.1 (2026-09-16)

### Fixes

#### `filename.title-consistency` reports the right cause, at the right line, and stops inventing findings

Three defects in one rule, all of which only show against a corpus this project did not author:

- **An H1 that exists but carries no number** (`# Add Status Field`) reported `no H1 title found`.
  Two different defects — "add a title" and "this corpus numbers its records somewhere other than the
  H1" — shared one message, and it was false for the second. Each cause now has its own message, with
  the no-H1 wording unchanged so that case is not a behaviour change.
- **Every finding pointed at line 1.** Records here open with YAML frontmatter, so an H1 is never on
  line 1. Findings now carry the line the heading was actually found on; the no-H1 case carries no
  line rather than a fabricated one.
- **A `# ` inside a fenced code block counted as the title.** A record whose only `# ` line was a
  shell comment reported a confident mismatch against a number that was not a record number. Fenced
  regions are now skipped, as is YAML frontmatter — whose comment lines start with `# ` for the same
  reason.

Separately, **a waiver whose `Expires` value is not a date no longer suppresses anything.** The
comparison was lexical and unvalidated, so `Expires: soon`, `TBD`, or a mistyped `2026-9-16` sorted
above every real date and produced a waiver that could never expire. A non-date expiry now expires
immediately. This mechanism may fail toward more findings, never fewer.

## 0.2.0 (2026-09-16)

### Breaking Changes

#### `pointer_fields`/`narrative_fields` are now config-declared per type

`pointer_resolution`, `header.pointer-field-clean`, and the renamed `narrative-field.stale` (was
`blocked-on.stale`) previously scanned a fixed Rust array — `Implements`/`Derives-from`/`Parent`/
`Blocked-on` — regardless of what a type's `.urzua/config.toml` entry actually declared. An org
adding a custom relationship field (e.g. `Feeds-into`) would pass `header.field-set-consistency`
but never get it resolved, checked for dangling references, or shown in `urzua graph`.

Both are now real per-type config keys, `pointer_fields` (clean, comma-separated references,
format-enforced) and `narrative_fields` (prose-tolerant, staleness-checked) — undeclared means zero
fields of that kind checked, no implicit default. **If your own `.urzua/config.toml` doesn't declare
either key for a type, that type silently loses the pointer-resolution/clean-format checking it used
to get for free from the old hardcoded field list** — add `pointer_fields`/`narrative_fields`
explicitly to keep the same coverage; three new `check` rules will otherwise stay silent on an
undeclared type rather than warning you. `config.pointer-declaration-missing` (a type must declare
both lists explicitly, even as `[]`), `config.pointer-field-not-known` (a declared field must also
be in `required_fields`/`known_fields`), and `config.pointer-narrative-overlap` (a field can't be in
both lists).

`urzua graph` gains a `kind: "pointer" | "narrative"` field on every edge, is now config-driven
instead of a hardcoded 3-field list (so a type's own `Parent` now appears for free), and no longer
reports a false `dangling: true` for a padded reference against an unpadded filename (a normalization
gap `pointer_resolution` never had).

#### Legacy pre-type-prefix filenames no longer resolve

`record_id`/`filename_number`/`next_display_number` now recognize only the `TYPE-NNNN-slug.md`
filename shape. The legacy `NNNN-slug.md` shape (accepted permanently by ADR-36's original
decision) is no longer parsed -- a reference to a filename in that shape is now correctly dangling,
the same as a reference to any other nonexistent record. BUG-9 found this repo's own corpus, and
every other known corpus, has zero files still using the legacy shape.

#### One shared output contract across every command: `Report`, `Notice`, `emit()`

`check`, `audit`, `fix`, `new`, `explain`, `graph`, and `doctor` previously each hand-rolled a
different, incompatible JSON shape. Every one of them now implements a shared `Report` contract and
prints through one function -- concretely:

- Every report gains an optional `notices` array: non-fatal observations (`severity: "info"|
  "warning"`, `subject`, `message`) that never affect the exit code, omitted entirely when empty.
  `--by`'s divergence from an authenticated `gh` login is the first real user of this.
- Every fatal "could not run" path now emits real JSON on stdout, not just an unstructured stderr
  message -- previously true only for `fix`; `new`, `explain`, `graph`, and `doctor`'s early guard
  printed nothing on stdout at all on failure.
- **`check`/`audit`'s failure-path JSON changes shape**: it used to include `scope`/`rules_executed`/
  `files_examined` as present-but-empty/zero keys even when the tool couldn't run at all; those keys
  are gone on that path now (`{"status": "not-run", "error": "..."}` instead). Every other command's
  change is additive (nothing on stdout before, real JSON now).
- Stderr is no longer used for anything this tool's own code controls, including genuine errors --
  the error message now lives in the JSON itself. `--help`/`--version` remain plain text on stdout
  (never JSON-wrapped), matching `cargo`'s own convention. An unexpected panic now prints a minimal
  JSON object to stdout instead of Rust's default raw text to stderr.
- A bad CLI flag now produces the same JSON-on-stdout, real-exit-code behavior as any other fatal
  error, instead of clap's own unstructured usage message.
- `urzua init` and `urzua migrate ids` join the contract too: `init` reports `proposed`/`written`,
  with the full rendered config only present on `--dry-run` (nothing else to read the preview from,
  since nothing was written); `migrate ids` reports `missing` and, when `--apply` runs, a real
  per-file `applied`/`skipped`/`failed` outcome instead of `[OK]`/`[SKIPPED]`/`[FAILED]` prose lines.
  `migrate ids --apply` also now exits 1 if any file failed to write, matching `fix --apply`'s own
  signal for the same shape of outcome -- previously exit 0 unconditionally, even on a real write
  failure.

#### Stdout is always JSON -- `--format` is gone

`check`, `fix`, `explain`, `graph`, and `migrate schema --report` no longer support `--format`.
Stdout is unconditionally one JSON object, on every invocation, with no human-readable rendering
anywhere (stdout or stderr). If you were parsing `--format human` output or relying on a stderr
rendering, switch to parsing the JSON on stdout -- it was already the more complete report.

### Features

#### `urzua audit`: supersession reciprocity and dangling references

`urzua audit` reports cross-record issues `check` doesn't scope to: a record claiming to supersede
another that doesn't point back, and cross-references that don't resolve. Read-only -- never writes,
since a bulk cross-reference rewrite is a real data-loss risk without a review step.

#### `check` now detects Embodiment drift

`embodiment.consistency` checks whether a `Realized-by` locator changed, per git history, since
the `Realized-by` line was last touched -- if so, the expected `Embodiment` is `Drift detected`
regardless of tier, and a stated value that disagrees is a finding. No new schema field: nothing is
stored, the comparison reads git blame/log directly. Requires full git history to detect anything
-- a shallow clone (CI's default checkout depth) makes this rule silently unable to find drift.

#### `urzua explain` and `urzua graph`

`urzua explain <path>` lists every record whose `Realized-by` names that file as evidence -- which
decisions govern this file. `urzua graph` dumps the full `Implements`/`Derives-from`/`Supersedes`
relationship graph as data, flagging edges that don't resolve.

#### `urzua new` now emits type-prefixed, unpadded filenames

New records are named `TYPE-N-slug.md` (e.g. `ADR-36-...md`) instead of `NNNN-slug.md` --
matching how a record is already referenced everywhere else (`Implements: ADR-36` is now the same
string as the file's own name). The number is never zero-padded: fixed-width padding doesn't solve
lexicographic sort order permanently, it only defers the break to whenever one type crosses the
padded width, and breaks worse there (mixed-width filenames). Unpadded is at least consistent
forever. The descriptive slug is kept.

Existing `NNNN-slug.md` filenames are never renamed and keep working forever -- both shapes resolve
identically regardless of padding, and a directory can hold a mix of both indefinitely. Also fixed:
`filename.title-consistency` had its own separate hardcoded 4-digit-only parsing (the same defect
class as BUG-2, in a different function) -- both now accept any digit length and compare by numeric
value.

#### `urzua fix`: detect and apply Embodiment repairs

`urzua fix` reports fields whose stated value disagrees with what Urzua computes from
`Realized-by` evidence. `urzua fix --apply --ids <records> --by <you>` writes the computed value
back -- one field's line only, with a required identity and an appended revision-log entry. Refuses
outright on a record with no revision log, rather than writing somewhere it can't be audited.

#### Two more header shapes: bold-list and YAML frontmatter

Declare `header_shape = "bold-list"` or `"yaml-frontmatter"` per record type in
`.urzua/config.toml` alongside the existing `blockquote` default. Lets `check`/`fix` read a corpus
that already uses a bold markdown list or real YAML frontmatter, instead of forcing a rewrite to
adopt Urzua.

#### `fix --apply`/`new` now prefer an authenticated `gh` session over `--by`; `new` gains a `--by` flag

Identity resolution for `fix --apply` and `new` (RFC-2/ADR-31) now checks `gh api user` first, then
falls back to an explicit `--by` value, and only then to `git config user.name` if neither is
available. Previously `--by` won outright, which meant a typed name could silently override an
authenticated `gh` session -- that no longer happens. If a `--by` is given and it differs from an
authenticated `gh` login, `gh`'s login still wins; the divergence is surfaced as a `notices` entry
in the command's JSON output rather than the value being silently discarded. `git config user.name`
is equally unsigned local input as `--by`, so it no longer outranks it (ADR-31's amendment).
`urzua new` also gains the `--by` flag `fix --apply` already had. `gh api user` is also now bounded
to 5 seconds rather than hanging indefinitely if `gh` doesn't respond.

None of these sources is a cryptographic attestation -- each is just harder to spoof by accident
than the one below it (RFC-2's own stated goal is raising the cost of a careless rubber-stamp, not
achieving unforgeable proof). Treat the resulting `Author`/`Tool-authored (by ...)` value as
attribution, not verification of who actually reviewed the change.

#### `urzua migrate ids`: backfill stable IDs

Backfills a collision-free `Stable-Id` (ULID) into every record that lacks one, without touching
the filename or any cross-reference -- the display number stays exactly what it already is.
Dry-run by default; `--apply` writes.

#### `urzua migrate schema --report`: preview a new required field

`urzua migrate schema --report --field <Name>` lists every record that would newly fail if `Name`
were added to `required_fields` today -- a read-only preview before you ever touch config.

#### Record types can now declare a shorter filename prefix

`RecordTypeConfig` gains an optional `prefix` field, decoupling a type's filename/ID prefix from
its own name -- the same way `dir` already decouples the type name from its directory. Omitted, a
type's prefix still defaults to its name upper-cased (no change for existing configs).

This repo's own `milestone` type now declares `prefix = "MILE"`: `urzua new milestone "..."` still
takes the same argument and `docs/milestones/` is still the directory, but new records are named
`MILE-N-slug.md` instead of `MILESTONE-N-slug.md`. All 36 existing milestone records were renamed
to match, with cross-references updated.

#### `urzua new`: create a record with a stable ID assigned

`urzua new <type> [title]` fills in the type's checked-in template (or synthesizes YAML
frontmatter if the type declares that shape and has no template) with a fresh `Stable-Id`,
resolved author, and today's date. Never asks you to pick a number.

#### `check` now flags pointer fields that mix in explanatory prose

New rule, `header.pointer-field-clean`: `Implements`/`Derives-from`/`Parent` are meant to hold only
comma-separated reference IDs, but nothing previously caught a value like `RFC-1 (Accepted)` or
`SPEC-1 (v0 CLI), which lists the bug classes this suite must reproduce.` -- `extract_references`
silently reads the leading token and discards the rest. `Blocked-on` is deliberately excluded: it
legitimately mixes free text with an optional embedded reference.

#### `blockquote`/`bold-list` deprecated; `init` now proposes `yaml-frontmatter`

`urzua init`'s adopt mode now proposes `header_shape = "yaml-frontmatter"` for every newly-adopted
record type, regardless of what shape the existing corpus already uses -- it recommends the
destination going forward rather than preserving whatever the corpus happens to look like today. A
new non-blocking rule, `header.deprecated-shape`, warns when a configured type's `header_shape`
isn't `yaml-frontmatter`. Parsing support for `blockquote`/`bold-list` is unchanged and stays
indefinitely: an existing corpus, or a first-time evaluator's unmodified docs, still read correctly.

Also fixed: `render_synthetic_yaml` (what `urzua new` uses for any `yaml-frontmatter`-shaped type)
previously built its output with unescaped string formatting -- a real value containing a colon or
a numeric-looking string could round-trip incorrectly. It now serializes through a real YAML
mapping, and no longer adds `Date`/`Author` to a record unless the type's own `required_fields`
actually names them.

### Fixes

#### Fix: `check <path>` now actually scopes to `<path>`

`urzua check docs/adr/` previously examined the entire corpus regardless of the path given --
`paths` was used only to locate the repository root, never to filter what got checked. Fixed:
`check` now genuinely restricts examination to files under the requested path(s). No CLI syntax
change; results narrow correctly for the first time.

Also fixed: record identifiers no longer require exactly 4 digits in the filename, and reference
matching (`Implements`/`Derives-from`/`Supersedes`) now compares by numeric value, so `ADR-34`
and a hand-typed `ADR-34` resolve to the same record regardless of padding.

#### `urzua new` respects the configured header_shape, even with an old template present

`urzua new` picked its output shape by whether `.urzua/templates/<type>.md` existed, ignoring
`header_shape` entirely -- a type configured as `yaml-frontmatter` with a leftover blockquote
template still got blockquote output (BUG-3). The configured shape now wins unconditionally: a
`yaml-frontmatter` type always gets synthesized YAML frontmatter, and if a template happens to
exist, its body sections (everything from the first `## ` heading onward) are still spliced in
underneath, so fixing the header shape doesn't cost the template's scaffolding.

#### `check` now surfaces the real reason a YAML header failed to parse

A record whose `yaml-frontmatter` header contained genuinely broken YAML (a value starting with a
reserved character, an ambiguous unquoted colon) previously reported only `"no header-shaped region
found"` -- the actual parser error, including its line number, was discarded. `header.required-fields`
now appends the real `yaml_serde` error (or a specific "must be a mapping" message when the YAML
parses but isn't the right shape) to that finding, instead of leaving a reader to diagnose the raw
bytes by hand.

## 0.1.0 (2026-09-05)

First tagged release. `urzua check` and `urzua init` (adopt mode) are real; `new`, `audit`,
`migrate`, `export`, and `import` still exit 2 with "not implemented yet."

### Added

- `urzua check`: validates a corpus against a declared shape — header-format consistency, cross-
  reference resolution (`Implements`/`Derives-from`), field presence/quality (blank vs. placeholder
  vs. pending vs. present), filename/title consistency, and `Supersedes`/`Superseded-by`
  reciprocity. JSON (`--format json`) and human-readable output.
- `urzua init`: adopt mode only — proposes a `.urzua/config.toml` from an existing corpus without
  moving any files. Never clobbers an existing config; `--dry-run` supported.
- `urzua doctor`: reports on the tool's own configuration health (does the config exist, does it
  parse, does CI actually invoke `check`) rather than on record content.
- Waivers: a reviewed exception to a rule is its own record type (`Rule`, `Scope`, `Reason`,
  optional `Expires`), never a config-level ignore list. A waived finding stays listed in output,
  just excluded from the blocking exit code.
- `.urzua/config.toml` carries `schema_version` from this release onward.

### Known limitations

- `new`, `audit`, `migrate`, `export`, `import` are unimplemented stubs.
- The full SPEC-2 rule set (roles, required sections, declared scope, boundaries) is not yet
  built — see `docs/specs/SPEC-2-urzua-check.md`.
