# Changelog

All notable changes to `urzua` are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/). Written for the person installing it — what changed
for you — not a commit dump.

## 0.4.0 (2026-09-23)

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

#### New opt-in rule `field.leading-reserved-indicator`: flags any declared field whose value starts with a

reserved YAML indicator character (`@`, `*`, `&`, `!`, `%`, `|`, `>`), which needs quoting to parse at
all. `BUG-18` found this happening to every hand-typed `Author`/`Deciders` value in this corpus (a
purely decorative `@`, forcing avoidable quoting) via a one-off backfill with no guard against it
recurring -- which it did, twice, the same day as the backfill itself. This rule generalizes the check
past those two fields to any declared field, so the next instance is caught by `check` rather than found
by hand again.

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

### Fixes

#### An absent `claim_paths` directory no longer aborts `check`. Git keeps no empty directory, so a

declared `.changeset` ceases to exist the moment a release consumes the last fragment, and a
repository following the documented pattern lost `check` entirely. The absence is reported as a
notice, which is visible and never moves the exit code; a path that is actively wrong — a file where a
directory was declared, or a link resolving outside the repository — still aborts.

#### `census_records` collected every examined candidate's path into a `Vec`, then sorted and deduped the

whole thing once at the end -- for a `Field`-unit rule, a record with many declared fields could
contribute one clone per examined slot before the same set collapsed out of a single bulk sort. Now
uses a `BTreeSet`, deduped incrementally as candidates are examined instead of in one final pass over
the full candidate list. No behavior change.

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

#### Fixes BUG-91: `doctor`'s `ci-wired` check scans every workflow rather than reading

`.github/workflows/ci.yml` by name, so a repository whose invocation lives in a differently named
workflow is no longer reported as having an unwired checker. A workflows directory that exists but
cannot be read is now an error rather than being reported as unwired. Its `required_fields` warning
also stops claiming header rules will never fire — they do.

#### Fixes BUG-92: resets the in-tree version to `0.3.0` and removes a generated CHANGELOG section that

reached `main` from a release-prep branch. No `0.4.0` was ever published, so the next release computes
`0.4.0` from the pending changesets rather than `0.4.1`.

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

#### Fixes `BUG-124`: the new `config.known-fields-declaration-missing` rule (`RFC-43`/`ADR-62`) flagged a

`header_shape: none` type for not declaring `known_fields`, even though such a type has nowhere for
any header field to be (`ADR-50`) and `known_fields` is meaningless for it. Now exempted, matching
`config.header-none-has-no-required-fields`'s existing reasoning for `required_fields`.

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

#### Adds the first slice of `SPEC-4`'s acceptance suite (`MILE-101`): a seeded, hand-rolled generator over

an alphabet of characters that have broken case-insensitive comparisons, and two properties checked
against it. The declared-field matcher is extracted as `rules::field_is_declared`/`fold_field_name`, so
the suite tests the matcher the rules use rather than a restatement of it. No rule behaviour changes.

#### Adds a mechanical guard (`crates/urzua-cli/tests/relation_field_literals.rs`) against

`BUG-118`/`BUG-119`/`BUG-120`'s recurring shape: a fix to `RFC-42`/`ADR-61`'s declared relation-field
names landed wherever a reviewed diff touched, while a sibling call site elsewhere in the tree kept
reading the old literal, three times in one review round. The new test walks every source file in
`urzua-core`/`urzua-cli` and fails if `"Status"`, `"Embodiment"`, `"Realized-by"`, or
`"Supersedes / Superseded-by"` reappears as a bare string literal in production code outside
`config.rs`, where `RelationRole`'s own defaults are declared. Ships with planted-violation cases,
verified to actually fail against the real shape of all three bugs before being restored.

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

#### `pointer.resolution`, `pointer.target-status`, `narrative-field.stale`, `claim.status-agreement` and

`relation.supersession-reciprocity` each rebuilt their own normalized id-to-record index independently
on every `check`/`audit` run — the same O(corpus) work done five times over. `check` and `audit` now
build one shared index per run and pass it to each rule. Verified as a no-op: findings, `files_examined`
and `records_read_by_any_rule` are unchanged on this corpus. `relation.supersession-reciprocity`'s own
index used to resolve a duplicate-id collision last-wins; the shared index resolves it first-wins
instead, which changes nothing observable since `identity.collision` already reports any such collision
on its own.

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
