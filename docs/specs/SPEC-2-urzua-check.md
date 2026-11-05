---
Version: '0.74'
Date: 2026-08-20
Status: Accepted
Embodiment: Drift detected
Realized-by: code:rust/crates/urzua-cli/src/commands/check.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
Author: beauwilliams
Subject: '`urzua check` -- discovery, rule execution, and the JSON output contract.'
Implements: RFC-1, RFC-3, RFC-10, RFC-11
Parent: SPEC-1
---
# SPEC-2 — `urzua check`

## Purpose

The validator, and the whole of Phase 0. Everything else in the roadmap parses and validates
records first, so this is the one piece nothing routes around. Cross-cutting rules — no-silent-
no-op, the permanent content-scope ceiling, the three-corpus acceptance bar — are stated in
`Parent`'s target, `SPEC-1`, and are not restated here except where this command narrows them.

`check` reads records, applies rules, and reports findings. It never writes. `--fix` is deferred to
RFC-8 and is out of scope here for a recorded reason: a bulk cross-reference rewrite caused a
documented data-loss incident in one source implementation, and repair is not safe until a revision
log makes it reversible.

## Success criteria

Done when all five hold:

| # | Criterion | Phase |
|---|---|---|
| 1 | `make check` runs `urzua check` in this repo's CI and fails the build on a deliberately broken record | A |
| 2 | It reports files examined, records read by any rule, rules executed, **and each rule's input population with its unit** | A |
| 3 | It exits non-zero when given paths that match zero files | A |
| 4 | Every rule family in §Rules is implemented and configurable | B |
| 5 | It runs green in one adopter's codebase with that adopter's hand-written linter **deleted** | C |
| 6 | It reproduces every documented bug in SPEC-1's acceptance suite against real corpora (SPEC-4) | D |

Criterion 6 belongs to the Phase 1 exit (`SPEC-1`, Exit bar), not to this spec's done.

Criterion 5 is the bar for the *product*. Criterion 1 is the bar for *momentum*: until the tool
validates this repository, every rule after it is written against corpora nobody has run.

A validator running alongside the linter it was meant to replace has failed, whatever its coverage
says.

## Phases

The command specified below is the finished shape. It is not the first thing to build, and the
ordering here exists to answer one question: **what is the shortest path to the tool running against
this repository's own records?**

Everything changes once that loop closes. Until it does, every rule is a claim about corpora nobody
has run; after it, each rule is exercised on real records the day it lands, and the project's own
governance is validated by the thing it is building.

**Corpus zero is `docs/` in this repository.** SPEC-4's anonymized corpora are required by
criterion 3 — reproducing the documented historical bugs — and are *not* required to close the loop.
That decoupling is deliberate: anonymization carries an unresolved question about whether the
corpora can be published at all, and the fastest deliverable must not sit behind the slowest one.

### Phase A — the loop closes

**Done when `urzua check` runs in this repository's CI and fails on a deliberately broken record.**
Nothing more. Unscoped: a path argument narrows which findings are reported (`BUG-67`), and this
repository's corpus is no longer confined to `docs/` -- the claim files a rule reads live beside it.

Scope:

- Git-tracked discovery, with `scope.source` reported.
- Header parsing for the record types in `docs/`.
- A minimal `.urzua/config.yaml` — record types and directories only. Not the full configuration surface,
  but a real file rather than a hardcoded layout, so the config thesis is exercised from day one
  rather than retrofitted.
- **Exactly two rules**, chosen because both fire on this corpus today (see below).
- The output contract in full: `status`, `filesExamined`, `rulesExecuted`, `scope`, `blocking`.
  The contract is cheap now and expensive to change once anything consumes it.
- Exit codes 0/1/2.
- A planted-violation test per rule.

Explicitly not in Phase A: the remaining rule families, severities, roles, boundaries, `doctor`,
the acceptance corpora.

**The two seed rules, and why these two.** Both are chosen so that the first honest run produces
findings rather than a green light — a validator whose first output is `ok` has told you nothing
about itself.

1. **Header format consistency within a record type.** At the time Phase A was scoped, this
   corpus's 22 records carried three header formats: RFC and ADR used one key per blockquote line;
   SPEC-1 used that shape while SPEC-2 through SPEC-4 used a pipe-delimited single line plus bold
   keys and a wrapped continuation. The third format had arrived in this repository's own corpus
   after header closure was specified — which is the failure mode RFC-10 describes, arriving in the
   corpus of the tool meant to prevent it. (Since converged onto `yaml-frontmatter` corpus-wide,
   ADR-33/ADR-42 — this historical drift no longer describes the corpus's current state, only what
   motivated choosing this as one of the two Phase A seed rules.)
2. **`Implements:` / `Derives-from:` resolves, and the target's status is reported.** Six pointers
   in `docs/specs/` currently name RFCs that are all `Draft`. That is RFC-12's
   decision-before-implementation gate with live subjects. Phase A only needs the pointer to
   resolve and the target status to be surfaced; whether a `Draft` target is an error, a warning, or
   the normal state of a young corpus is a decision for RFC-12, not for the checker.

Both are also the cheapest possible proof that the checker reads structure rather than greps text.

### Phase B — the rule set

The remaining rule families in §Rules, severity resolved from configuration (SPEC-3), and
`doctor`. Each rule lands with its planted-violation test and is exercised against `docs/` the day
it merges.

### Phase C — the second corpus

Part of the Phase 1 exit (`SPEC-1`), not Phase 0's. The falsifiable one. A second codebase runs green under the same binary with no code change, and its
hand-written linter is deleted. A failure here is a schema
finding to record rather than a code path to add.

### Phase D — the historical suite

SPEC-4's corpora and the documented bug classes. This is what turns "it works on two corpora" into
criterion 3, and it is last because it is the only phase whose blocker is not engineering.

## Discovery

**Default scope is what the VCS tracks**: `git ls-files` ∪ staged paths, intersected with the record
directories configured in SPEC-3. Never a raw directory walk.

This is not a preference. One source implementation globbed the filesystem directly, and a single
untracked scratch file under the records directory failed *every* run from that checkout, including
runs touching zero records. The tracked set is also what a reviewer sees, which makes the check's
input equal to the review's input.

Explicit paths on argv override discovery and are used as given -- for a directly-named file. A
directory argument still scopes the tracked-only sweep rather than walking the filesystem raw
(`BUG-24`): the "never a raw directory walk" guarantee above governs a directory argument the same as
the default sweep, and only a path naming one file precisely bypasses the tracked-set filter.

A file named in its type's `exclude_files` (`SPEC-3`, `MILE-150`) is dropped at this step: it is never
loaded as a record, so it is in no rule's population, `files_examined` and the type's file census for
`filename.identity-pattern`.

A type owns its `dir` and each of its `extra_dirs` (`SPEC-3`, `MILE-130`): discovery loads the files
directly in any of them as that type's records, so the type's ids form one namespace for pointer
resolution, `identity.collision` and `filename.identity-pattern`.

| `scope.source` | Meaning | `base` | Produced today |
|---|---|---|---|
| `tracked-sweep` | the tracked record set | `null` | **yes** |
| `git-diff` | changed against a base ref | the ref | **yes** (`MILE-123`) |
| `argv` | a directly-named file not in the tracked set | `null` | **yes** (`BUG-24`) |
| `none` | nothing selected | `null` | no |

The values are a serde-renamed `ScopeSource` enum (`BUG-25`), not a rendering of whichever internal
type produced them. Only variants the tool can actually emit exist on that enum; the rest of this
table is the declared vocabulary those modes will use when built, and is marked so a consumer is not
misled into branching on a value no run produces.

`base` is always present in `scope`: the ref as the caller gave it for a `git-diff` run, `null` for
every other source.

**`git-diff` scope** (`MILE-123`) is selected by `urzua check --base <ref>`. It is a reporting scope,
not a reading scope, exactly as a path argument is (`BUG-60`):

- The changed set is every path that differs between the merge base of `<ref>` and `HEAD` and the
  working tree, so a branch is not charged for what its base gained since the fork, and staged and
  unstaged edits to tracked files count. Untracked files never do (the tracked-only guarantee
  above). A rename is a deletion plus an addition, and a deleted path is simply not a record any
  more.
- The **examined set** is the changed records plus their dependents, and `files_examined`,
  `records_read_by_any_rule` and the findings are limited to it. A dependent is an unchanged record
  that (a) names a changed or deleted record's id in any header field (a pointer, narrative or
  relation field), (b) shares a changed record's id, or (c) cites changed or deleted evidence in
  its `Realized-by`, or (d) cites a normalised `Realized-by` locator that a changed or deleted
  record also cites, so a cross-record finding such as `embodiment.locator-promotion-candidate`
  lands on the same record a full check reports it on, or (e) is the `Scope` (or any record, for
  `"*"`) of a waiver that changed or was deleted, as it was at the merge base or as it is now, since
  that waiver may have stopped waiving a finding on it. A header value is split on commas first, so
  `ADR-1,ADR-3` names both. Deleted records are read from the merge base, since they are no longer in the
  tree; a rename is a deletion of the old id plus an addition. Without dependents a change
  elsewhere that breaks an unchanged record -- a deleted target, a renamed id, a changed `Status`,
  a reciprocal supersession, drifted or deleted evidence -- would be hidden, and a gate that hides
  what a full check reports is not a narrower check but a wrong one. One hop only: a dependent's own
  dependents are not pulled in.
- Reference resolution and every other corpus-wide computation still index the whole tracked
  corpus, so an unchanged target is never reported missing. Whether a finding survives a path argument or `--base` is decided by the
  anchor its rule declares in the rule table, never by rule id: a finding on the configuration, or on a
  file no record-type `dir` covers (a claim file, a source file, an index artifact, a changed path),
  survives every scope; one on a record follows the record; one on a tracked path or a type's directory
  follows the changed paths (`BUG-86`, `BUG-121`).
- A change to the configuration file the run loaded puts **every record** in scope (the rules
  themselves changed), and `scope.whole_corpus_reason` says so. A configuration that is a symlink
  counts as changed when either the link's own path or its target's path differs. A path argument
  still narrows this afterwards. A changed file below a declared dir that no type owns keeps its
  `type.record-outside-declared-dir` finding, though it is not a loaded record.
- `scope` carries the account of the examined set: `changed_paths` (every path git reports,
  deletions included), `dependents` (the unchanged records pulled in) and `whole_corpus_reason`
  (`null` unless the configuration changed). These fields appear only on `git-diff` runs.
- Path arguments still apply and narrow the examined set further.
- A diff with no changed record and no configuration change examines nothing and reports `ok`
  (exit 0, `files_examined: 0`), not `not-run`, provided some rule is enabled and looked at
  anything (a configuration with no rule enabled is `not-run` here exactly as in a full run): the question asked had the answer "nothing to
  judge". It is told apart from a whole-corpus pass by an info `notice` and by `scope`
  (`changed_paths` is empty, or holds no record), and a blocking finding that survives the scope
  (about the configuration) still makes it `findings-present`. A ref that does not resolve, or
  one with no merge base with `HEAD` (the error says "no merge base"), is a could-not-run (exit 2),
  never an empty selection.
- `--base` together with a directly-named file git does not track is refused (exit 2): the diff
  lists tracked paths only, so the file could not be examined, and dropping it silently is the
  failure `BUG-24` fixed.

A `null` base is not "no scope" — a full sweep legitimately has one. Any consumer keying "did this
run" on a missing base is reading the wrong field, and the four sources exist so it does not have to.

**Fallback with no git repository**: fall back to a directory walk, and say so in `scope.source`.
Never silently — the fallback has the failure mode above, so a run using it must be identifiable
from output alone.

## Rules

Rules are data where possible, not code (`urzua-core`). Each rule declares an id, the record types
it applies to, the **statuses it applies to**, and its default severity.

Status applicability is part of the declaration rather than an `if` inside each rule. A record can
legitimately hold a number without stating a decision — a tombstone (RFC-13) — and a check that
presupposes a decision is not wrong, it is *scoped*. A corpus that cannot express that forces its
records to invent content or its linters to be switched off.

### Shipped, by actual rule id

This is the complete, current set — generated from `urzua_core::rules::RULE_TABLE` by
`scripts/generate-rule-table.py` (`BUG-52`), never hand-edited, so it cannot drift from
the rule table the way its hand-maintained predecessor did. Regenerate with `make rule-table`; `make
ci` fails if the committed table is stale. Not a delta on top of the aspirational language below,
which stays only for what genuinely isn't built yet.

<!-- rule-table:start -->
| Rule id | What it checks |
|---|---|
| `header.required-fields` | Each type's `required_fields` (SPEC-3) are present; a missing header region or a duplicate key is also reported here. |
| `pointer.resolution` | A type's config-declared `pointer_fields` ∪ `narrative_fields` (MILE-90/ADR-44) resolves to a real record when it names one — a dangling reference is a finding, a resolved one is silent (`pointer.target-status` and `narrative-field.stale` separately judge what a resolved target's status means). No hardcoded field list — an undeclared type is skipped, not defaulted. |
| `pointer.target-status` | A reference resolves, but its target's `Status` is one the repository declared unacceptable (`not_in`). The optional `fields` option restricts it to the named declared pointer/narrative fields, so a dependency field (`Implements`) can be judged apart from a lineage field (`Derives-from`); an excluded field is not part of the rule's population, and a `fields` entry naming no declared field is a config error. Split from `pointer.resolution` in `MILE-80`. |
| `field.quality` | Field presence with real semantics: `blank`, `placeholder`, and `pending` are three distinct states, not one — the single most-repeated bug class in hand-written record linters. An absent field is never silently readable as any of the three. The optional `field_levels` option (`MILE-140`) sets `warn` or `error` per field; a field it does not name takes the rule's `level`. The optional `template_placeholders` option (`BUG-22`), off unless declared `true`, reads a field whose value is identical after trimming to its type's template text for that field (`.urzua/templates/<type>.md`) as a placeholder, and stops applying the compiled-in placeholder tokens; a type with no template is judged by its declared `pending` vocabulary alone, and declaring the option when no type has a template is disclosed as a warning notice. |
| `field.pending` | A required field is marked `Pending` — work declared unfinished, as distinct from forgotten. Split from `field.quality` in `BUG-38`. The optional `field_levels` option (`MILE-140`) sets `warn` or `error` per field; a field it does not name takes the rule's `level`. The optional `pending_until_required` option (`MILE-169`), off unless declared `true`, treats a pending value as legitimate until the record reaches a status the type's `statuses.require` ties the field to: a deferred slot stays eligible and is counted unexamined, and a field no `require` entry names is judged as before. |
| `claim.status-agreement` | A file outside the corpus claims to close a record whose own `Status` disagrees. Scans `claim_paths`; `closed_statuses` declared. |
| `filename.title-consistency` | The filename's display number and the H1's display number agree (ADR-3). |
| `relation.supersession-reciprocity` | If A supersedes B, B points back; status-aware (a claim only binds once the claiming record is itself terminal-accepted). Shared with `audit` (ADR-30) — `check` runs it too, not only `audit`. |
| `relation.pair-reciprocity` | For each declared `relation_pairs` entry with `reciprocity: required` (`RFC-47`, `MILE-122`): a claim `A.forward = B` is echoed by `B.inverse = A`, and a claim `A.inverse = B` by `B.forward = A`, across record types where the pair allows it. A pair that declares `types` is carried by those types only, and a claim to a type outside the list is excluded as out of scope (`MILE-155`). A pair that declares `provisional: {statuses, level}` reports an unechoed claim from a claimant at one of those statuses at the provisional level (default `warn`), and any other claimant at the rule's level (`MILE-154`). The optional `provisional.acknowledged_by` names a field on the peer that settles a provisional claim by naming the claimant (`MILE-163`); it is not read for any other claimant. Needs `relation_pairs`; enabling it with none declared is a load error. Beside, not replacing, `relation.supersession-reciprocity`, which keeps the combined `Supersedes / Superseded-by` key. |
| `relation.cross-type-disallowed` | A claim in a declared pair's field whose target is a different record type, for a pair declaring `cross_type: false` (`RFC-47`, `MILE-122`). Such a claim is excluded from `relation.pair-reciprocity` and reported here, never skipped silently. |
| `sequence.cycle` | A cycle in a declared `relation_pairs` entry that sets `acyclic: true` (`RFC-48`, `MILE-27`): following the written claims of either field, in the direction each declares, returns to the record it started at. Every record on a cycle is reported with the cycle's path, including a record that names itself and a cycle that crosses record types. A claim whose target does not resolve is `sequence.unresolved-blocker`'s. Needs an acyclic pair; enabling it with none declared is a load error. |
| `sequence.unresolved-blocker` | A value in either field of a declared `relation_pairs` entry that sets `acyclic: true` and names no record (`RFC-48`, `MILE-27`): a reference that resolves to no discovered record, or an entry that is not, as a whole, one record reference (`MILE-4 MILE-5` is prose). A claim in a field that is one of the type's `pointer_fields` is left to `pointer.resolution` and `header.pointer-field-clean` when those are enabled. A fact that is not yet a record is a record of its own, not a prose value (`ADR-76`). Needs an acyclic pair; enabling it with none declared is a load error. |
| `relation.target-status-undeclared` | A resolved pointer or narrative reference whose target type never declares `Status` at all — `ADR-60`'s gate would otherwise silently exclude it from every status-reading rule, with no disclosure of why (`RFC-45`/`ADR-63`). |
| `revision-log.change-class-required` | Every revision-log entry carries a `change_class` (ADR-14); missing or unclassified is blocking. The log is the section a type's `revision_log` identity (default the line `**Revision log**`) opens, found as that line and never by a line that mentions the text (`MILE-167`). |
| `revision-log.version-bump-required` | A changed record that adds a revision-log row of a class its type declares must also change its version (`BUG-51`, `SPEC-18`: a substantive or structural change is a `Version` bump plus a revision-log entry). A type declares it as `revision_log: {version_bump: {field: Version, classes: [substantive, structural]}}`: a field name and the classes that require a bump, and no type has it by default. Judged against the merge base's copy of the record, so it reports `not-applicable` on a run with no `--base`, never clean; a record new in the diff, including one renamed in it, has nothing to bump from and stays unexamined. A row counts as added when its cells appear in the current log and not in the merge base's. Enabling the rule when no type declares a bump is a config load error. |
| `embodiment.consistency` | A stated `Embodiment` agrees with the tier its `Realized-by` locators compute to (the tier order of the `embodiment` declaration, by default `test:` outranks `code:` outranks `spec:`); a locator that changed per git history since `Realized-by` was last touched overrides the expected value to the declared drift state unconditionally (RFC-5 tier 1, ADR-18/ADR-32). A record with no `Realized-by` field is judged on the back-pointers that cite it (`MILE-149`); one stating a state the declaration marks `claims_built` that has neither a `Realized-by` field nor a citing back-pointer is reported as a claim with no evidence, computed to the declared no-evidence state (`MILE-161`), and one stating any other value is not judged and stays unexamined. Requires full git history — a shallow checkout makes this rule silently unable to detect anything. Three opt-in refinements, all off by default (`MILE-129`): a locator written `path@sha256:<64 hex>` drifts when the file's content hash differs and not otherwise, in place of the git-history heuristic; the `drift_grace_days` option holds back reporting drift whose locator last changed fewer than that many days ago, and a record in that window agrees with either its tier or a stated drift state (the window never contradicts a record that already says so, and `urzua fix` leaves it alone); the optional `drift_audit_field` option names a declared date field (needs `drift_grace_days`): a record carrying a `YYYY-MM-DD` date there has its window counted from that last-audit date instead of from the cited file's last change, and a malformed date is reported and falls back to the file (`MILE-166`); the `claim_max_age_days` option warns when the `Realized-by` line was last touched more than that many days ago. The optional `sticky_states` option lists `Embodiment` values a human sets that no evidence can compute, such as `Deprecated` and `Inactive`: a record stating one is examined and never reported as drift or as an old claim, `urzua fix` leaves it alone, and a listed value that evidence does compute (a state of the `embodiment` declaration) is a load error because it would hide real drift (`MILE-148`). Source back-pointers (see `embodiment.back-pointer-resolution`) are evidence too: an `Implements:` citation is code tier and a `Verifies:` citation in a declared test file is test tier, equal to a `test:` locator (`MILE-146`). A record with `Embodiment` but no `Realized-by` field is judged on back-pointer evidence alone when one cites it, and a record neither cited nor carrying `Realized-by` is not judged (`MILE-149`); a finding names the evidence it computed from. |
| `embodiment.requires-accepted-status` | A record whose `Status` is not an accepted status states an `Embodiment` the `embodiment` declaration marks `claims_built` (`MILE-138`): a decision not yet made cannot have been built. The accepted statuses are the type's declared `statuses.success`; a type declaring none uses the rule's `accepted_statuses` option, and with neither the type is not examined. Any other state, including the drift state and a sticky one, is not flagged, and a record missing either field is left to the header rules. Opt-in; the population counts the records the rule could judge. |
| `embodiment.back-pointer-resolution` | A source comment line `Implements: ADR-12` (a comment leader such as `//`, `#`, `--` or `*`, then the marker, then comma- or space-separated record ids, each optionally followed by a note, as in `Implements: ADR-1 (why), ADR-2`) in a file under a `back_pointer_paths` entry counts as code-tier realization evidence for the record it cites, in `embodiment.consistency` and `urzua fix` (`MILE-128`, narrowing `RFC-5` section 2), and a back-pointer citing a record that does not exist is reported here. The marker is read at the start of a comment line or in a trailing comment after code (`//`, `#`, `--`, `/*`, `<!--` after whitespace); the marker is case-sensitive; reading continues past a note to the next comma-separated id and stops at a segment that does not open with one; ids are compared numerically. Opt-in and needs `back_pointer_paths` (path prefixes, normalised at config load, no globs); evidence is collected only while this rule is enabled. A declared path covering no tracked file is itself a finding, and a non-text file is counted as unreadable, never as clean. The optional `test_paths` option (path prefixes, each inside a `back_pointer_paths` entry, else a load error, or globs such as `**/*.test.*` matched among the scanned files; a glob that cannot compile is a load error and one matching no scanned file is a finding) declares the test class: a `Verifies: ADR-12` comment in a file under it is test-tier evidence for the record it cites, and is checked for resolution like an `Implements:` one; a `Verifies:` comment elsewhere is not read (`MILE-146`). |
| `embodiment.back-pointer-decision-accepted` | A source back-pointer (`Implements: ADR-12`, read exactly as `embodiment.back-pointer-resolution` reads it) citing a record whose `Status` is not one at which implementation is permitted is an error: code that says it implements a decision is landing ahead of that decision (`MILE-133`). Permitted means the cited type's declared `statuses.implementable` (a terminal `success` status is not assumed to permit it); a type that declares none uses the rule's `accepted_statuses`, and with neither the status cannot be judged, which is reported rather than passed. Needs `back_pointer_paths`; `accepted_statuses` is optional. On a `--base` run only back-pointers in files the diff changed are judged (file granularity). A citation of no record is `embodiment.back-pointer-resolution`'s, not this rule's. A declared path covering no tracked file is itself a finding. Mechanical fixes carry no back-pointer and are outside it. It checks the cited record's status, never whether the code matches the decision. |
| `embodiment.locator-exists` | A `Realized-by` locator names a path that is not both git-tracked **and** present on disk, or is empty. Tracked alone passes a staged deletion; on disk alone passes a gitignored file. |
| `embodiment.locator-promotion-candidate` | The same locator cited by more than one record's `Realized-by` is a promotion candidate, surfaced as a finding, never auto-promoted (ADR-18). |
| `header.field-set-consistency` | Every header field belongs to its type's `required_fields` ∪ `known_fields`, when `known_fields` is declared (ADR-39). The optional `allow_extension_prefix` option (`MILE-141`), off unless declared `true`, exempts header keys beginning `x-` in any case; an unprefixed undeclared key is still a finding. |
| `field.untrimmed-value` | A field value carrying leading/trailing whitespace, reachable only through `yaml-frontmatter` — disclosed rather than silently trimmed. |
| `field.leading-reserved-indicator` | A declared field's value starts with a reserved YAML indicator character (`@`, `*`, `&`, `!`, `%`, `|`, `>`), which needs quoting to parse at all — the same avoidable, decorative-prefix hazard `BUG-18` found for `Author`/`Deciders`, generalized so the next instance is caught rather than found by hand. |
| `header.field-case-mismatch` | A declared field written under a different case than its declaration (`RFC-40`/`ADR-58`) — the one place a case-only miss is distinguished from genuine non-adoption. |
| `header.case-variant-key` | One record's header writes two keys differing only by case (`Status`/`status`), reported as the fact of the difference, never as which spelling is right (`MILE-113`). Opt-in and declaration-free: it reads the keys a record wrote, not a declared vocabulary, and composes beside the exact-match rules without changing them (`ADR-57`). An exact repeat of one spelling stays `header.required-fields`' finding. |
| `narrative-field.stale` | A type's config-declared `narrative_fields` pointer (e.g. a milestone's `Blocked-on`) resolves to a target whose `Status` has reached a terminal state (e.g. a cited bug is now `Fixed`) — a signal to re-examine (ADR-42, generalized beyond `Blocked-on` by MILE-90/ADR-44). `terminal_statuses` is declared per repository. |
| `type.dir-matches-nothing` | A type's declared `dir` does not exist at all (`BUG-56`'s hazard for `dir`, no load-time guard). An existing, empty directory is a legitimate declared-but-unused state and is not flagged. Also reports a type’s `exclude_files` entry that names no tracked file directly in its `dir` (`MILE-150`), so a stale exclusion is visible. Reports each `extra_dirs` entry that does not exist (`MILE-130`). |
| `identity.collision` | Two records resolving to the same identifier — a corpus error the index cannot represent, reported rather than papered over (`BUG-79`). |
| `type.record-outside-declared-dir` | A record-shaped tracked path that no type's `dir` claimed (`BUG-62`) — a config written before `RFC-35` silently loses coverage on upgrade otherwise. |
| `type.no-declared-spec` | Which configured types currently have no `spec` pointer declared at all (ADR-41/ADR-43) — a signal to review, never a mandate; a type can permanently have none declared. |
| `header.pointer-field-clean` | A type's `pointer_fields` entries are clean, comma-separated references only, format-enforced (MILE-90/ADR-44) — an empty entry between commas or freeform trailing prose is flagged. Never applies to `narrative_fields`, which tolerate prose. |
| `config.companion-declaration-missing` | A type's `companion_fields` lists pairs of relation roles, for example `[embodiment_state, embodiment_locator]`, whose fields it must declare together (`BUG-41`). A type declaring one field of a pair in `required_fields`/`known_fields` without the other is reported, as is a pair whose fields it declares neither of: a rule reading both, such as `embodiment.consistency`, cannot judge a record that lacks one. The fields are the ones the type's `relation_fields` names for the roles; no pair is known to the engine. |
| `config.pointer-declaration-missing` | A type declaring either `pointer_fields` or `narrative_fields` must declare both explicitly, even as `[]` (MILE-90/ADR-44) — omitting one is not the same as declaring zero fields of that kind. |
| `config.pointer-field-not-known` | Every field named in a type's `pointer_fields`/`narrative_fields` must also appear in that type's own `required_fields`/`known_fields` (MILE-90/ADR-44) — otherwise `header.field-set-consistency` would never have heard of it. |
| `config.pointer-narrative-overlap` | A field must be exactly one kind: named in both `pointer_fields` and `narrative_fields` for the same type is a contradiction (MILE-90/ADR-44). |
| `config.header-none-has-no-required-fields` | A type declaring `header_shape: none` has nowhere for a field to be, so a non-empty `required_fields`, `known_fields`, `pointer_fields`, `narrative_fields`, or `relation_fields` is a self-contradiction (`ADR-50`). |
| `config.relation-field-not-known` | Every field named in a type's `relation_fields` (`RFC-42`/`ADR-61`), including the `date` and `author` fields `urzua new` fills (`MILE-176`), must also appear in that type's own `required_fields`/`known_fields`, the same shape as `config.pointer-field-not-known`. |
| `config.known-fields-declaration-missing` | A type declaring no `known_fields` at all gets no field-set governance from `header.field-set-consistency` (`ADR-53`'s "declared, not voted" default) — opt-in, so a repository can require the choice be made explicit rather than left ambiguous by omission (`RFC-43`/`ADR-62`). |
| `filename.identity-pattern` | A file directly in a type's `dir` whose name does not match the type's declared `identity.pattern` (or, for a type declaring none, the `TYPE-NNNN-slug.md` convention) is reported rather than left out of every rule's population (`MILE-120`). When every file in the directory is off-pattern, such as a bare-number corpus (`0001-slug.md`), it is one finding per type naming the expected file name for each file, since renaming is the supported route (`RFC-46` was rejected). The census counts each file examined, and omits a file the type declares in `exclude_files` (`MILE-150`). |
| `status.undeclared-value` | A record's `Status` is not one of the values its type declares in `statuses.values` (`MILE-132`). A type declaring no `statuses` is not examined, and a record with no `Status` is left to `header.required-fields`. |
| `status.required-field-missing` | A field the type's `statuses.require` ties to the record's current `Status` is blank, unedited template text, or (when the type declares `statuses.pending`) one of that declared placeholder vocabulary's whole values such as `Unknown` or `none recorded`, or a value matching one of its `{pattern: ...}` entries for a placeholder family (`MILE-132`, `MILE-153`, `MILE-162`), for example `Deciders` on an `Accepted` record. A type declaring no `pending` keeps the old reading, so a value that only contains the word is still a real value. A type declaring no `require` entries is not examined. The optional `field_levels` option (`MILE-140`) sets `warn` or `error` per field; a field it does not name takes the rule's `level`. |
| `status.forbidden-field-present` | A field the type's `statuses.forbid` bars at the record's current `Status` carries a value (`MILE-132`), for example an acceptance date on a record that is not accepted. A type declaring no `forbid` entries is not examined. |
| `status.transition-allowed` | A changed record's `Status` moved along an edge its type's `statuses.transitions` does not list, comparing the status the merge base committed with the one in the change under check, or a record new in the change starts at a status outside `statuses.initial` (`MILE-144`, `ADR-72`). Needs `--base`: a whole-repo run reports it not-applicable, not a pass. A type declaring no `transitions` is not examined, a record the diff did not change is eligible and not examined, and a record whose committed status cannot be read or is no longer declared is counted unreadable. A renamed record reads as new (`BUG-191`), because the diff reports a rename as a deletion plus an addition. A repository whose ADRs and RFCs enter at `Proposed` or `Draft` and are accepted in a later change declares that as `initial`, so a record filed already accepted is reported; this repository does so. |
| `status.unreachable-state` | A status a type declares in `statuses.values` that no path of its `statuses.transitions` reaches from `statuses.initial` (`MILE-144`, `ADR-72`): no record can legitimately hold it. Judges the declaration, not a record, so it needs no `--base`. A type declaring no `transitions` is not examined. Enable it at `warn` for the informational reading. |
| `status.delivery-agreement` | A record's `Status` agrees with its delivery evidence, in both directions (`MILE-183`): the tier computed from its `Realized-by` locators and source back-pointers (drift ignored) is one of the type's `statuses.delivery.delivered_tiers` while the `Status` is one of its `not_done` statuses, so a built record still reads as unbuilt; and a `success` status whose evidence computes to one of its `unevidenced_tiers`, so a record claims to be done with nothing behind it. The mapping is the type's own declaration (`delivered_tiers`, `unevidenced_tiers`, `not_done`, `set_status`); the rule knows no status or type by name. `urzua fix --apply` writes `set_status` for the first direction; the second has no single correct value and is repaired by adding evidence. A type declaring no `delivery` is not examined. The population counts the records of types that declare it. |
| `date.field-valid` | A value in a declared date field (`date_fields`) that is not a well-formed `YYYY-MM-DD` date, or is well-formed but not a real calendar date such as `2026-02-30`, is an error (`MILE-137`). A blank, placeholder or pending value is not a date value and is left to `field.quality` and `status.required-field-missing`. Needs `date_fields`. The population counts the date values examined. |
| `date.field-order` | Within one record, a date field that is later than the field `date_order` says must not be later is an error: `Date <= Accepted-on` reports a record accepted before it was written (`MILE-157`). Equal dates pass. A value that is blank, a placeholder, pending or not a real `YYYY-MM-DD` date is not compared and is left to `date.field-valid`, `field.quality` and `status.required-field-missing`, which own those states. Applies to every record type that declares both fields of an entry. Needs `date_order`. The population counts one slot per record and entry; a record missing either date is eligible and unexamined. |
| `citation.resolution` | A decision ID cited anywhere in a record's prose (`ADR-12`, with a configured type prefix, standing alone) resolves to an existing record, compared numerically so zero-padding does not matter (`MILE-136`). The header, which `pointer.resolution` governs, and fenced code blocks are not read; an inline code span is. Opt-in. The optional `external_ids` option (`MILE-156`) lists ids that share a record prefix but are not records, each an exact id (`RFC-4122`) or an inclusive numeric range written `PREFIX-LOW..HIGH` under a declared prefix; a matching citation is counted as eligible and unexamined, never resolved or reported, and not as out of scope, so `config.scope-matches-nothing` does not read a declared id as a misconfiguration. An entry under no declared prefix is a config error. Its population counts the citations, so a run over none is distinguishable from a clean one, and external citations are visible as eligible and unexamined. The optional `skip_revision_log` option, when `true`, leaves each record's revision log (the table under its type's declared `revision_log` identity) unread, so append-only history that cites an id as it was is not reported; it is `false` by default (`MILE-156`). The optional `non_reference_field` option (`BUG-190`, `ADR-75`) names a header field in which a record lists, as comma-separated exact ids (`ADR-72`; no ranges, compared by value), the ids it names without citing as a reference: a citation of a listed id in that record only is counted as eligible and unexamined, neither resolved nor reported, with `external_ids` taking precedence. A listed id the record never cites in the prose the rule reads, and an entry that is not an exact id of a declared prefix, are findings on the record naming the field and its line. The field must be listed in a record type's `known_fields` and must not be a pointer, narrative or relation-role field, or the config does not load. `check` prints one aggregated Info notice naming declared ids that now match a real record; a notice never changes the exit code. |
| `citation.path-exists` | A repository path cited in backticks inside a `reference_sections` section (a `##` heading, matched exactly) exists in the working tree, tracked and on disk (`MILE-143`). A cited path is a code span with no whitespace, containing a `/` and ending in a file name with an extension or in `/`; a URL, a host-led path (`example.com/x`), an absolute or `~` path, a glob or placeholder (`*?<>{}$|`) and anything else is not a path and is not read. A trailing `:line` or `#anchor` is dropped, and `./` or `../` resolves from the record's directory. Fenced code blocks are not read. Needs `reference_sections`; a record without such a section is out of scope, and the population counts the paths cited. The optional `skip_revision_log` option, when `true`, leaves each record's revision log (the table under its type's declared `revision_log` identity) unread, so append-only history that names a path as it was is not reported. It is `false` by default. Offline and deterministic. Cited issue and pull request numbers are `citation.issue-exists`'s. |
| `citation.issue-exists` | An issue or pull request cited inside a `reference_sections` section (a `##` heading, matched exactly) is in the committed snapshot of the repository's numbers (`MILE-143`). `issue_refs` declares the `snapshot` file (relative to the repository root), the `repository` (`owner/name`) it describes and the citation `forms` read: `number` is `#123` (outside inline code, not `owner/repo#123`, not an anchor link `](#123)`), `url` is a `https://github.com/owner/name/issues/123` or `/pull/123` link to that repository; a form not declared is not read. A number is judged against the snapshot's issues and pull requests together, since GitHub shares one number space and redirects either URL. Offline and deterministic: `check` never reaches the network, and only `urzua refs refresh` writes the snapshot. A snapshot that is absent, unreadable, of another schema or of another repository is one finding on the snapshot path, and every citation is counted as eligible and unexamined: the rule never passes for want of its input. Each finding names the date the snapshot was generated, and `check` prints a notice with that date and the snapshot's size, so staleness is visible. The optional `skip_revision_log` option behaves as for `citation.path-exists`. The optional `issue_refs.fields` names header fields (as a type declares them, such as `Found-in`) whose values are read for the same forms, each finding naming the field and its line; a name no record type declares is refused at load, and a record without the field is not examined. Body prose outside `reference_sections` is deliberately not readable. The population's unit is the citation. |
| `boundary.published-path-reference` | A record identifier (any identifier carrying a configured type prefix, resolvable or not) is cited from inside a `published_paths` entry (a path prefix, normalised at config load; globs are not supported and a glob is treated literally), which publishes a pointer no external reader can resolve (`RFC-11`/`ADR-9`, `MILE-125`). An identifier split across lines is not detected. A declared path matching no tracked file is itself a finding, never a clean pass; the rule's level is the boundary's severity. |
| `section.required` | Each `##` section a type's declared `sections.required` names is present in every record of that type (`MILE-4`, `RFC-17`), under its `name` or one of its declared `aliases` (`MILE-165`; `check` discloses each alias in use as an info notice), or, for a `y-statement` declaring `preamble: true` and a `marker`, in the text before the first `##`. A type that declares none is out of scope; a record whose status is in the declaration's `exempt_statuses` (a record that made no decision, `MILE-7`) is counted eligible and unexamined, not asked for the sections, and not out of scope, so `config.scope-matches-nothing` does not read a declared exemption as a misconfiguration (`BUG-187`). A required section may declare `level` and `under` to be a `###` (or deeper) heading nested under a named `##`, which needs the type's `sections.depth` to reach it (`MILE-98`). Presence only: it never judges what a section says. |
| `section.shape` | A required section that declares a `shape` has that shape (`MILE-5`, `MILE-121`): `y-statement` needs its TL;DR block to carry the five phrases `In the context of`, `facing`, `we decided`, `to achieve`, `accepting`, word-boundary matched, and reports a marker with no block as its own error; `table-min-rows` needs a table with at least `min_rows` data rows. A section found under an alias is checked too, and a `preamble: true` Y-statement is judged from the text before the first `##`. A missing section is `section.required`'s to report, not this rule's. Shape only, never meaning. |
| `index.freshness` | A committed index of one record type (a file, or a marked list inside a README, in a declared `format` with `{id}`, `{file}`, `{title}` and declared-field placeholders such as `{Status}`) lists exactly the records: a record with no entry, an entry naming no record, a duplicated entry, an entry whose file or title differs from the record's, and a line the format does not match are each an error (`MILE-139`). A `format` may span several lines (written with `\n`), reading one entry over that many non-blank lines; it needs `{id}` or `{file}` (a link list that names no id is identified by its file); `{title}` is the H1 without its leading number or the record's own id (`MILE-152`). A missing artifact, or one whose declared `begin`/`end` markers are absent, is an error too. Needs `indexes` (`path`, `type`, `format`, optional `begin`/`end`); `check` only compares, and `urzua index` regenerates the artifacts from the records. The population counts the artifacts compared, so a run over none is distinguishable from a clean one. |
| `diff.source-requires-record` | A `--base` diff changes a path under a declared `source_paths` entry (a prefix or a glob) but changes no governed record (a `.md` file directly in a record type's `dir`) (`MILE-124`, `MILE-158`). A changed path matching `exclude_paths` is not source and stays in the population unexamined, so a diff touching only such files (tests beside the code) is not a trigger. One finding per changed source path, so a waiver can scope to one. It asks only that a record changed, never that it is the right one. Judges the changed set, so it reports `not-applicable` on a run with no `--base`, never clean. Deleted paths count as changed. |
| `diff.source-and-records-mixed` | A `--base` diff changes both a governed record and a path under a declared `application_paths` entry, a prefix or a glob (less any `exclude_paths`), a prompt to split the change (`MILE-142`). One finding naming the first path of each kind, in path order. A governed record is never application source, whatever the prefixes say. Reports `not-applicable` on a run with no `--base`, never clean. |
| `config.scope-matches-nothing` | Judges the run itself, not the corpus: whether the configuration's declared rules reached what they were handed, from the other rules' own executions (`MILE-106`). |
| `waiver.matches-nothing` | A waiver that does nothing is reported on the waiver record, naming the rule and scope it claimed (`MILE-168`). Four cases: the waiver is expired (its `Expires` has passed or is not a `YYYY-MM-DD` date, so it already covers nothing); it is not in effect (it lacks a field the suppression type's `effective_requires` names, or holds a blank, placeholder or pending value there, so it suppresses nothing and its finding is reported again); it names a rule this build does not ship; or it is active, its rule ran, and no finding of that rule falls in its scope. A waiver matching a finding is not reported, and an expired waiver is reported once, as expired; an ineffective one is reported once, as ineffective. The unmatched case is judged only on a whole-corpus run in which the target rule ran: under a path scope or `--base`, or with the target rule off, an absent finding proves nothing, so the waiver is counted as eligible and unexamined (never out of scope). The expired and unknown-rule cases need only the waiver record to have been examined. Opt-in. The population counts waiver records. |
<!-- rule-table:end -->

### Not yet built

Design language from this spec's original Phase B scope, with no shipped rule behind it yet:

- **Status enum validation, tolerant of trailing free-text annotation**, and validated against
  corpus *prose*, not only instantiated values: a value that no record currently holds may already
  be committed to by another document, and is real a release before anything is set to it. Nothing
  validates `Status` against an enum for any record type today (found live via MILE-71's red-team of
  the milestone `Status` field).
- **Role rules** — Author / Reviewers / Deciders per type and status, with self-acknowledgement
  detection: an author is not their own reviewer of record.
- **Required sections** per profile, generalized beyond `change_class` on revision entries (the one
  instance that's actually built).
- **Declared scope** (RFC-11 §6) and **boundaries** (RFC-11 §1) — neither is built; RFC-11's own
  open questions (nested-scope inheritance, glob-vs-subject width) are still unresolved.

Severity per rule comes from config (SPEC-3) in principle; in practice severity is still hardcoded
per rule at the source (`Error`/`Warning` only, no `Info`), not yet read from any config field —
tracked as MILE-80, which also names the still-open question of a richer severity taxonomy versus a
separate log-verbosity axis. `status` reports what was found; whether that fails the build is the
caller's choice and lives in `blocking`.

## Output contract

Implements RFC-3, as amended by ADR-7 (real field names, not RFC-3's illustrative casing) and
ADR-26/ADR-46 (stdout is the only stream; there is no stderr rendering at all). Stdout carries nothing
but the JSON object below, so a consumer never has to find it inside prose.

```json
{
  "status": "ok",
  "files_examined": 51,
  "records_read_by_any_rule": 51,
  "rules_executed": [
    {
      "rule": "header.required-fields",
      "population": { "unit": "field", "eligible": 1248, "examined": 1248, "out_of_scope": 0, "unreadable": 0 },
      "status": "ran"
    },
    {
      "rule": "claim.status-agreement",
      "status": "not-enabled"
    }
  ],
  "scope": { "source": "tracked-sweep", "base": null, "record_types": ["adr", "bug", "rfc"] },
  "blocking": false,
  "findings": [
    {
      "rule": "pointer.target-status",
      "severity": "warning",
      "file": "docs/adr/ADR-48-x.md",
      "line": null,
      "message": "Derives-from: ADR-29 resolves, but its Status is Superseded"
    }
  ],
  "notices": [
    { "severity": "warning", "subject": "identity.collision", "message": "..." }
  ]
}
```

(`rules_executed`/`findings` truncated above for length; a real run lists every declared rule and every
finding.)

`rules_executed` is the field that distinguishes this from every check in the source corpora --
`files_examined` alone answers "did it read anything," not "did it *check* anything," and a rule set
narrowed by a config error yields a confident zero over a swept corpus. Each entry's `status` is
`"ran"`, `"not-enabled"` or `"not-applicable"` (`RuleStatus`); a `"ran"` entry carries `population` (`eligible`/`examined`,
plus `out_of_scope`/`unreadable` -- `ADR-55`'s disclosure of what the rule was handed versus what it
judged), a `"not-enabled"` entry carries none. `"not-applicable"` is an enabled rule the run cannot supply
what it judges: the diff rules (`diff.source-requires-record`, `diff.source-and-records-mixed`) judge the
changed set, so on a run with no `--base` they report it, carry no population and count as not having
looked, so a run whose only enabled rules are diff rules is `"not-run"`, never `"ok"`. On a `--base` run
their findings are about changed source paths rather than records, so the diff scope does not filter them
out, and a path argument does not either. Their path options (`source_paths`, `application_paths`,
`exclude_paths`) are normalised like `published_paths`, and each entry is either a path prefix or a glob
(`MILE-158`, `SPEC-3`); `exclude_paths` removes matching paths from `source_paths` as well as from
`application_paths`, so a diff touching only excluded files is not a trigger; those paths stay in the
population as eligible and unexamined (not out of scope, which `config.scope-matches-nothing` would
report as an unreachable configuration); "a governed record changed" means a changed `.md`
file directly in a record type's `dir`, deleted ones included, and a governed record is never application
source. Their population is counted in `changed-path`.

`status` is the real `ReportStatus` enum: `"ok"`, `"findings-present"`, or `"not-run"` -- never `"ok"`
when zero files were selected, keyed on the file count, not on a missing scope, nor when no rule
looked at anything. The one exception is a `git-diff` run whose diff reaches no governed record
while some rule is enabled and ran: that is `"ok"` with `files_examined: 0` and an info notice. `notices` (`ADR-46`)
is non-fatal observations (`Info`/`Warning` only) that never affect `status` or the exit code, omitted
entirely when empty -- the same is true of a `Finding`'s own `waived` field, omitted unless a waiver
covers it.

Every finding's `file` is relative to the repository root, including a finding about the configuration
(`.urzua/config.yaml`), so a report compares and baselines across checkouts (`BUG-185`). A `--config`
outside the repository has no relative form and is named as given.

### Suggested action

`ADR-73` decides this field; this section reproduces its vocabulary. A finding may carry one
`suggested_action`: the machine-actionable form of what to do about it, so an agent need not read
`message`. It is additive: a finding without one is unchanged, and `message` is always present.

```json
{
  "rule": "field.quality",
  "severity": "error",
  "file": "docs/bugs/BUG-9-x.md",
  "line": null,
  "message": "field 'Regression-test' is Blank -- not a real, present value",
  "suggested_action": {
    "kind": "edit-field",
    "op": "set",
    "target": { "file": "docs/bugs/BUG-9-x.md", "field": "Regression-test" }
  }
}
```

Keys are snake_case and kinds kebab-case, like the rest of the report. The vocabulary (a value in
`kind`, the `op` an edit takes, the `target` keys and the `value`):

| kind | required `target` keys | optional `target` keys | `op` (required) | `value` |
|---|---|---|---|---|
| `edit-field` | `file`, `field` | `line` | `set`, `remove`, `append`, `rename` | a string (a list-valued header uses the comma-separated form `SPEC-2` reads). For `set`, `append` and `rename` (the new name) only when derivable and single-valued; omitted means a human chooses. For `remove`, omitted removes the whole field and a value names the entry removed from a list field |
| `edit-section` | `file`, `section` | `line` | `add`, `set`, `remove` | never (prose) |
| `edit-config` | `file`, `key_path` (array of path segments, never a dotted string) | | `set`, `remove`, `append` | only when single-valued: the YAML value as JSON (a string, a list of strings, or a boolean). For `remove`, omitted removes the whole key and a value names the entry removed from a list |
| `add-record` | | `type` (omitted when not determinable), `file` (path to create, if known) | none | never |
| `run-command` | `argv` (array) | | none | never |

- `file` is repository-root-relative, as in findings (`SPEC-2`, `BUG-185`), and need not equal the finding's `file`: for the reciprocity rules the finding sits on the claimant and the edit belongs on the counterpart. `line` is given only where it differs from the finding's. `field` is the declared spelling.
- One flat key vocabulary: `file`, `field`, `section`, `type`, `rule`, `scope`, `key_path`, `argv`, `line`. A new key is added to this table, not invented per rule.
- `value` is any JSON value of the types in the table: a string, a list of strings, or a boolean; no other type occurs in the appendix. A key or `value` that was not computed is omitted, never `null`. The same holds for an absent `suggested_action`.
- `argv` is concrete in the emitted finding (no placeholders), `argv[0]` is `urzua`, and a consumer adds `--by` only when no identity resolves (`SPEC-8`).
- `run-command` means: run exactly this command; the command is the source of truth for what it writes. For `fix` and `index --apply` it is offered only when the command would produce that repair for this finding (a test checks each): `urzua fix --apply --ids <record>` for `embodiment.consistency` and `status.delivery-agreement`, `urzua index --apply` for `index.freshness`, `urzua refs refresh` for `citation.issue-exists`, which is not judged by that test: it needs the network and a token, and an unresolved citation may be a typo, so that command is a lead and `citation.issue-exists` declares `edit-section` set as its action instead. Because it carries no `value`, a `fix` repair stays the one place its computed value lives; `edit-field` carries a value only where no `fix` repair exists (relation reciprocity, `field.untrimmed-value`, `header.field-case-mismatch`, `config.pointer-declaration-missing`). "Derivable" follows `ADR-15`'s eligibility clauses. A test asserts that every finding offering `urzua fix` has a matching repair in `urzua fix` output for that record and field. A `run-command` can still be refused by the command's own gates (`--ids`, revision-log requirements); the finding promises the command, not its success.
- `waive` is not in the first release. A finding carries exactly one `suggested_action`, so there is no slot for a secondary action, and a kind with no producing rule would be a contract without a test. Adding `waive` (a waiver is a record of the `waiver` type, `ADR-11`) and a secondary-action mechanism such as an `alternatives` array are a later, separate decision; both are non-breaking under the compatibility rule.
- There is no `note` key: `message` is the prose and is always present.

Declaration. The declaration is a data table, not a function, in `RULE_TABLE` (`MILE-171`): each rule lists, in `actions`, one `VariantAction` per finding variant it can emit, naming the variant and an `ActionDecl`, which is either an action (a `kind` and, where the kind has several, an `op`) or `Exempt(&'static str)` with a reason, modelled on `HeaderNone::Exempt`. A rule never builds an action: it attaches an `ActionSource` to the finding, naming the variant and carrying only what it computed (a field name, a configuration key path, a value), and the engine joins that to the declaration. A variant with neither an action nor an exemption, a variant a rule emits that is not declared, and a declared variant no rule emits fail a coverage test; a source missing a key its declared kind requires asserts in a debug build. Exemption reasons are shown by `urzua rules`, so "no action by design" is discoverable although it is an omitted field on the wire. A waived finding carries its action unchanged.

Compatibility. The set of kinds, `op` values and `target` keys is closed per release and extensible across releases. Adding a kind, an `op` value or an optional `target` key is non-breaking. Renaming or removing one, changing its meaning, or changing which keys are required is breaking. Consumers MUST treat an unrecognised `kind`, or an unrecognised `op` on a known kind, as non-actionable and fall back to `message`, and MUST ignore unrecognised `target` keys. `SPEC-2` states this in its output contract. The report has no output version field today; if one is added it is a separate record and kind additions stay minor.

The action each finding variant declares, or the reason it declares none, is generated from the same
rule table as the table above (`urzua rules` lists it as each rule's `actions`; an exemption shows
its `exempt` reason), so this section and the rule table cannot disagree. Regenerate with
`make rule-table`.

<!-- action-table:start -->
| Rule id | Variant | Declared action |
|---|---|---|
| `header.required-fields` | `header-region-missing` | exempt: the header region is neither a field nor a section, so no action kind addresses it |
| `header.required-fields` | `duplicate-key` | `edit-field` `remove` |
| `header.required-fields` | `field-missing` | `edit-field` `set` |
| `pointer.resolution` | `pointer-unresolved` | `edit-field` `set` |
| `pointer.target-status` | `pointer-target-status` | `edit-field` `set` |
| `field.quality` | `field-quality` | `edit-field` `set` |
| `field.pending` | `field-pending` | `edit-field` `set` |
| `claim.status-agreement` | `claim-status-disagrees` | exempt: the rule catches a claim that may be false, so the right fix is often the claim file, a non-record file with no action kind yet, and editing the record Status would be the mechanical answer and possibly the wrong one |
| `filename.title-consistency` | `title-mismatch` | exempt: the fix is a rename or an H1 edit, and no action kind addresses either yet |
| `relation.supersession-reciprocity` | `supersession-unresolved` | `edit-field` `set` |
| `relation.supersession-reciprocity` | `supersession-unreciprocated` | `edit-field` `append` |
| `relation.pair-reciprocity` | `pair-unresolved` | `edit-field` `set` |
| `relation.pair-reciprocity` | `pair-unreciprocated` | `edit-field` `append` |
| `relation.cross-type-disallowed` | `cross-type-claim` | `edit-field` `remove` |
| `sequence.cycle` | `sequence-cycle` | exempt: breaking a cycle means choosing which dependency to drop, a decision the tool cannot compute |
| `sequence.unresolved-blocker` | `blocker-unresolved` | `edit-field` `set` |
| `sequence.unresolved-blocker` | `blocker-not-a-reference` | `edit-field` `set` |
| `relation.target-status-undeclared` | `target-status-undeclared` | `edit-config` `set` |
| `revision-log.change-class-required` | `change-class-missing` | `edit-section` `set` |
| `revision-log.version-bump-required` | `version-bump` | `edit-field` `set` |
| `embodiment.consistency` | `embodiment-drift` | `run-command` |
| `embodiment.consistency` | `embodiment-unevidenced` | `edit-field` `set` |
| `embodiment.consistency` | `last-audit-date-invalid` | `edit-field` `set` |
| `embodiment.consistency` | `claim-too-old` | exempt: the remedy is re-verifying the claim, which may need no edit, and touching the field to reset the age without checking would defeat the rule |
| `embodiment.requires-accepted-status` | `embodiment-state-not-accepted` | `edit-field` `set` |
| `embodiment.back-pointer-resolution` | `back-pointer-unresolved` | exempt: the edit is to a source file comment, and no action kind addresses a non-record file yet |
| `embodiment.back-pointer-resolution` | `back-pointer-uncovered-path` | `edit-config` `set` |
| `embodiment.back-pointer-resolution` | `back-pointer-uncovered-test-path` | `edit-config` `set` |
| `embodiment.back-pointer-decision-accepted` | `decision-not-implementable` | exempt: the edit is to a source file comment or to the cited record, and no action kind addresses a non-record file yet |
| `embodiment.back-pointer-decision-accepted` | `decision-unjudgeable` | exempt: the edit is to a type or rule declaration that depends on which statuses permit implementation, a policy choice with no single target |
| `embodiment.back-pointer-decision-accepted` | `decision-uncovered-path` | `edit-config` `set` |
| `embodiment.locator-exists` | `locator-empty` | `edit-field` `set` |
| `embodiment.locator-exists` | `locator-missing` | `edit-field` `set` |
| `embodiment.locator-promotion-candidate` | `locator-promotion-candidate` | exempt: an advisory nudge that is never auto-promoted |
| `header.field-set-consistency` | `undeclared-field` | `edit-field` `remove` |
| `field.untrimmed-value` | `untrimmed-value` | `edit-field` `set` |
| `field.leading-reserved-indicator` | `leading-reserved-indicator` | `edit-field` `set` |
| `header.field-case-mismatch` | `field-case-mismatch` | `edit-field` `rename` |
| `header.case-variant-key` | `case-variant-key` | exempt: the rule reads the keys a record wrote, not a declared vocabulary, so it cannot tell the declared spelling from the stray one and a remove could delete the declared key |
| `narrative-field.stale` | `narrative-field-stale` | `edit-field` `set` |
| `type.dir-matches-nothing` | `exclude-files-matches-nothing` | `edit-config` `set` |
| `type.dir-matches-nothing` | `extra-dir-missing` | `edit-config` `set` |
| `type.dir-matches-nothing` | `dir-missing` | `edit-config` `set` |
| `identity.collision` | `identifier-collision` | exempt: the fix is a renumber, and no action kind addresses a rename yet (`urzua migrate ids` exists) |
| `type.record-outside-declared-dir` | `record-outside-declared-dir` | exempt: the fix is a move, and no action kind addresses a move yet |
| `type.no-declared-spec` | `no-declared-spec` | exempt: a signal to review, never a mandate |
| `header.pointer-field-clean` | `pointer-field-not-clean` | `edit-field` `set` |
| `config.companion-declaration-missing` | `companion-field-missing` | `edit-config` `append` |
| `config.companion-declaration-missing` | `companion-pair-undeclared` | `edit-config` `append` |
| `config.pointer-declaration-missing` | `pointer-declaration-missing` | `edit-config` `set` |
| `config.pointer-field-not-known` | `pointer-field-not-known` | `edit-config` `append` |
| `config.pointer-narrative-overlap` | `pointer-narrative-overlap` | exempt: a field named in both lists could belong in either, and which list should keep it is the author's decision, not a computable repair |
| `config.header-none-has-no-required-fields` | `header-none-field` | `edit-config` `remove` |
| `config.relation-field-not-known` | `relation-field-not-known` | `edit-config` `append` |
| `config.known-fields-declaration-missing` | `known-fields-declaration-missing` | `edit-config` `set` |
| `filename.identity-pattern` | `identity-pattern-none-match` | exempt: the fix is a rename, and no action kind addresses a rename yet |
| `filename.identity-pattern` | `identity-pattern-mismatch` | exempt: the fix is a rename, and no action kind addresses a rename yet |
| `status.undeclared-value` | `status-undeclared-value` | `edit-field` `set` |
| `status.required-field-missing` | `status-required-field-missing` | `edit-field` `set` |
| `status.forbidden-field-present` | `status-forbidden-field-present` | `edit-field` `remove` |
| `status.transition-allowed` | `transition-not-allowed` | `edit-field` `set` |
| `status.unreachable-state` | `status-unreachable` | `edit-config` `set` |
| `status.delivery-agreement` | `delivery-not-done` | `run-command` |
| `status.delivery-agreement` | `delivery-without-evidence` | exempt: no single correct value exists and `urzua fix` writes nothing for it; the repair is adding evidence, which may be a locator or a source back-pointer |
| `date.field-valid` | `date-invalid` | `edit-field` `set` |
| `date.field-order` | `date-order` | `edit-field` `set` |
| `citation.resolution` | `citation-unresolved` | exempt: the fix edits prose located by line, and no action kind addresses a line in prose yet |
| `citation.resolution` | `non-reference-malformed` | `edit-field` `set` |
| `citation.resolution` | `non-reference-uncited` | `edit-field` `remove` |
| `citation.path-exists` | `path-missing` | `edit-section` `set` |
| `citation.issue-exists` | `issue-missing-in-section` | `edit-section` `set` |
| `citation.issue-exists` | `issue-missing-in-field` | `edit-field` `set` |
| `citation.issue-exists` | `snapshot-unusable` | exempt: regenerating the snapshot with `urzua refs refresh` needs the network and a token, so the command is named in the message and not offered as an action |
| `boundary.published-path-reference` | `published-path-reference` | exempt: the edit is to a published non-record file, and no action kind addresses a non-record file yet |
| `boundary.published-path-reference` | `published-path-uncovered` | `edit-config` `set` |
| `section.required` | `section-missing` | `edit-section` `add` |
| `section.shape` | `section-shape` | `edit-section` `set` |
| `index.freshness` | `index-stale` | `run-command` |
| `index.freshness` | `index-unregenerable` | exempt: `urzua index --apply` cannot regenerate this artifact (not text, a missing marker or an unparsable format), so the artifact or the declaration is edited by hand |
| `diff.source-requires-record` | `source-without-record` | `add-record` |
| `diff.source-and-records-mixed` | `source-and-records-mixed` | exempt: process advice, no file edit |
| `config.scope-matches-nothing` | `scope-matches-nothing` | `edit-config` `set` |
| `waiver.matches-nothing` | `waiver-expired` | `edit-field` `set` |
| `waiver.matches-nothing` | `waiver-ineffective` | `edit-field` `set` |
| `waiver.matches-nothing` | `waiver-unknown-rule` | `edit-field` `set` |
| `waiver.matches-nothing` | `waiver-matches-no-finding` | `edit-field` `set` |
<!-- action-table:end -->

## Exit codes

| Code | When |
|---|---|
| 0 | no findings at or above the configured blocking severity |
| 1 | blocking findings; under `fail_on: warning` (or `--fail-on warning`), also any unwaived warning (`SPEC-3`) |
| 2 | the check could not run — bad config, unreadable path, explicit paths matching zero files |

`--fail-on error|warning` overrides the configuration's `fail_on` for one run. It moves only the exit
code: `blocking` and the rest of the report are unchanged.

2 is distinct on purpose. A tool that cannot run and a tool that ran and found problems are
different states, and collapsing them is how "0 errors" comes to mean "never executed".

## Testing

- **Every rule ships with a planted-violation test** (RFC-11 §5). An absence assertion carries no
  information until its failure has been observed; a rule with no such test is unverified, not
  passing.
- **Corpus tests run against SPEC-4's anonymized corpora**, not synthetic fixtures, for every
  bug in SPEC-1's acceptance suite.
- **The no-silent-no-op property is itself tested**: a run over an empty selection must produce
  `not-run`, and a run with a rule set of zero must not report `ok`.
- **A derived rule reports the evidence it found, not only the findings it emitted.**
  Files-examined and rules-executed are not sufficient: four documented derivations each returned a
  clean, plausible zero, and **three of them would have passed a
  files-and-rules report** — the records were examined and the rules did run. What was empty was the
  evidence the rule had to reason over. A history-reading rule that resolved no history for any
  record reported a fully clean corpus; the cause was a `git log` invocation that stops at a rename,
  in a corpus that had just renamed its record homes.

  So a rule whose findings depend on derived input reports that input's size:
  `transition: 91 records, 0 with readable history` is self-refuting on its face, and `0 findings`
  is not.

  The defect is `eligible: N, examined: 0` — the rule was handed candidates and reached a verdict on
  none of them. That is §5 of RFC-11 applied to a rule's input rather than its output. `eligible: 0`
  is a *different* state and not a defect: it is the cold start of enabling a rule before its scope
  exists, which any rule declared before its scope exists occupies. Neither is a
  `check` verdict — what gets checked is the adopter's declaration (`ADR-53`) — and the separation
  between disclosing the population and judging it is `MILE-106`'s.
- **Fixture containment.** The rule tests deliberately contain record-shaped strings, and a
  self-referential fixture that the checker then scans is a documented trap. Fixtures live outside
  the discovered record set, and that exclusion is asserted rather than assumed.

## Out of scope

- `--fix` in any form. See RFC-8; needs the revision log first.
- **Writing.** `audit` shares `relation.supersession-reciprocity` with `check` (ADR-30) and adds
  dangling-reference reporting on top, but both commands are read-only. The data-loss incident this
  spec originally cited was about a bulk cross-reference *rewrite*, not about reading reciprocity —
  `check` running the same read-only rule `audit` does was never actually the risk.
- Judging whether a section's *content* covers the scope it claims. SPEC-1 records this as a
  permanent ceiling on mechanical checking, not a maturity gap. `check` is never sold as replacing a
  human reader.

## References

- SPEC-1 — v0 CLI: success criteria, acceptance suite, the permanent ceiling.
- SPEC-3 — configuration, which supplies record types, severities and boundaries.
- SPEC-4 — the corpora this is tested against.
- RFC-3 (output), RFC-10 (header), RFC-11 (boundaries and scope), RFC-13 (status-scoped
  checks), ADR-3 (identity).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Bumped to `0.10` (`BUG-21`). §Output contract's JSON block rewritten to the real shipped shape: `snake_case` fields, the real 3-value `ReportStatus`, `rules_executed` as the real per-rule array (not a count) with its `population`/`status` shape, `records_read_by_any_rule`, `notices`, and a `Finding`'s real fields (no `suggestedAction`, which never existed). The 2026-09-11 entry below named this drift and filed it rather than fixing it; this revision is that fix. | **substantive** |
> | 2026-09-21 | Criterion 2 no longer says "in both output formats", and re-keyed the zero-population rule in §Acceptance. **Why:** two claims that had gone untrue. There is one output format -- `ADR-23` and `ADR-26` removed the second -- so the criterion was unsatisfiable as written and no amount of correct implementation could have met it. And "a rule reporting a zero population is `not-run`" was written when a rule reported one number; now that a population carries `eligible` and `examined` separately, the two zeros mean opposite things, and treating `eligible: 0` as a fault makes enabling a rule before its scope exists fail the build (`BUG-84`). The criterion also now requires the unit, because a count without one held three different denominators (`BUG-40`). | **substantive** |
> | 2026-08-20 | Split out of SPEC-1, which retains the cross-cutting rules. | **structural** |
> | 2026-09-07 | Added the Embodiment consistency, drift, and locator-promotion rules (ADR-18/ADR-32) to the rule set — previously implemented but never listed here. Noted that the rest of this section's rule names predate and don't match the actual shipped rule ids, as a named gap rather than silently compounding it. | **substantive** |
> | 2026-09-20 | Criterion 1 and Phase A now say `urzua check`, not `urzua check docs/`. **Why:** the scope argument selects which findings are reported, so naming `docs/` excluded every finding about a file outside it -- `claim.status-agreement` reports on the claim file in `.changeset/`, and the gate therefore could never fail on a false claim, which is the one thing that rule exists to catch (`BUG-86`). The criterion's intent is that the tool checks this repository and fails on a broken record; `docs/` was the whole corpus when it was written and no longer is. | **substantive** |
> | 2026-09-09 | Rewrote §Rules for MILE-90/ADR-44: `pointer.resolution`/`header.pointer-field-clean` are now config-driven (`pointer_fields`/`narrative_fields` per type, no hardcoded field list); `blocked-on.stale` renamed `narrative-field.stale`, generalized beyond `Blocked-on`; added the three new config-level rules (`config.pointer-declaration-missing`, `config.pointer-field-not-known`, `config.pointer-narrative-overlap`) and the two rows this table had never listed at all (`type.no-declared-spec`, `header.deprecated-shape`) despite both already shipping. Corrected the stale "ten rules" framing to the real, current seventeen. **Why:** this table drifting behind the actual shipped rule set is the exact recurring gap this spec's own 2026-09-07 entry already named once; MILE-90 touched every one of these rule functions directly, making this the natural point to close the gap rather than let it recur a third time. | **structural** |
> | 2026-09-07 | Rewrote §Rules as a complete, accurate list of all ten actually-shipped rule ids (adding `header.layout-consistency`/ADR-38 and `header.field-set-consistency`/ADR-39, neither previously mentioned at all), separated from the design language that's still unbuilt. Corrected §Out of scope's claim that cross-record reconciliation is `audit`-exclusive -- `check` has run `relation.supersession-reciprocity` directly since ADR-30, and the actual data-loss risk this spec was guarding against was a bulk *rewrite*, never a read-only reciprocity check. **Why:** per ADR-14's amendment adopted earlier the same day, a spec's body must stay a complete, replayable specification of its subject at every revision, not accumulate "not yet reconciled" notes as a substitute for actually updating it -- leaving this stale on the very day that policy was adopted would have been an immediate, visible contradiction. | **substantive** |
> | 2026-09-07 | Added `blocked-on.stale` as an 11th rule (ADR-42); `pointer.resolution`'s row updated to include `Parent`/`Blocked-on`, which it had already gained (ADR-40) without this table being updated. **Why:** the same "spec must stay complete" policy applies to every rule addition, not just the ones made on the day the policy was adopted -- letting this table go one rule stale again immediately would have repeated the exact drift this spec was just corrected for. | **substantive** |
> | 2026-09-08 | Reworded the header-format-consistency seed rule's rationale (§"The two seed rules") from a present-tense claim about the corpus's current state to historical framing. **Why:** ADR-33/ADR-42's corpus-wide `yaml-frontmatter` migration made the original wording -- "this corpus of 22 records carries three header formats" -- false the moment it landed; caught by adversarial review before the migration shipped, rather than left as another stale-prose instance for a future pass to find. | **substantive** |
> | 2026-09-08 | Bumped to `0.5`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-09-11 | Noted `notices` as a real, shipped addition to the output shape (ADR-46), and named -- rather than silently compounded -- the pre-existing drift between §Output contract's illustrative RFC-3-era JSON block and the real shipped shape (camelCase vs. `snake_case`, stderr rendering that no longer exists, a `status` enum that doesn't match `ReportStatus`). Filed as its own bug rather than fixed in this revision. | **substantive** |
> | 2026-09-17 | Config moves from `.urzua/config.toml` to `.urzua/config.yaml` (`ADR-52`, shipped in the same change). The described mechanism changes, not just its rendering. | **substantive** |
> | 2026-09-18 | `Status: Draft` → `Accepted`, with `Embodiment`/`Realized-by` declared. **Why:** `check` is the most-built command in the tool and has its own integration-test file. This spec reached `Version: 0.8` -- eight revisions -- while still marked as a draft of something unbuilt. Verified against the binary before flipping rather than flipped in bulk -- `BUG-26` asked for exactly that, and it is why `SPEC-4` and `SPEC-5` are not flipped with these. | **substantive** |
> | 2026-09-19 | Rule count corrected from seventeen to twenty-one, and `ALL_RULES` named as the source this list must match. **Why:** `MILE-80` and the fixes after it added `pointer.target-status`, `field.pending`, `claim.status-agreement` and `embodiment.locator-exists` without updating the spec that calls itself *"the complete, current set"* -- so `check`'s own spec understated what `check` runs by four. Nothing checks the two agree, which is filed separately. | **substantive** |
> | 2026-09-19 | The four rules shipped since `MILE-80` added to the table: `pointer.target-status`, `field.pending`, `claim.status-agreement`, `embodiment.locator-exists`. **Why:** the count was corrected to twenty-one in the same change that left the list at seventeen rows, so the section contradicted itself in adjacent lines and the rule this repository declares `error` was undocumented in the spec calling itself complete. `BUG-52` is the missing mechanism. | **substantive** |
> | 2026-09-19 | `embodiment.locator-exists`'s row states the on-disk requirement. **Why:** it read *"absent from the git-tracked set"*, which was the rule's first form and silent on a staged deletion -- `git rm` drops a path from `ls-files` while leaving it in `diff --cached`. The spec described a check the code no longer performs. | **substantive** |
> | 2026-09-23 | Rule count corrected from twenty-one to twenty-nine; the eight rows `ALL_RULES` had gained since without this table being updated added (`type.dir-matches-nothing`, `type.record-outside-declared-dir`, `identity.collision`, `config.scope-matches-nothing`, `field.untrimmed-value`, `header.field-case-mismatch`, `config.header-none-has-no-required-fields`, `config.relation-field-not-known`). **Why:** found by a `RFC-42`/`ADR-61` review, which noticed this list's own self-declared invariant ("the complete, current set... `ALL_RULES` is the source it must match") had gone untrue by seven rows before its own change added an eighth. `BUG-52` remains open as the actual mechanism gap — nothing enforces this agreement automatically, so this is a manual sync, not a fix. | **substantive** |
> | 2026-09-23 | `argv` marked produced, scoped to a directly-named file; the Discovery prose states the directory-argument exception explicitly. **Why:** `BUG-24` fixed the gap this table had marked unbuilt -- an explicit argv path naming one file now overrides the tracked-only filter, while a directory argument still respects "never a raw directory walk" (`ADR-6`), which the table's blanket "explicit paths ... override discovery" line did not previously distinguish. | **substantive** |
> | 2026-09-23 | The rule table is now generated, not hand-maintained: `urzua_core::rules::RULE_METADATA` is the single source, `scripts/generate-rule-table.py` renders it between `<!-- rule-table:start/end -->` markers, `make rule-table-check` (wired into `make ci`) fails the build on drift. Backfilled the two rows `ALL_RULES` had gained since the last manual sync (`config.known-fields-declaration-missing`, `relation.target-status-undeclared`) and corrected `config.header-none-has-no-required-fields`'s description for its round-22 broadening. **Why:** `BUG-52` named this as the real fix -- not a comparison rule re-deriving the same fact a second time, but collapsing to one source so there is nothing left to drift. `BUG-52` is `Fixed`. | **substantive** |
> | 2026-11-02 | `pointer.target-status` gains the optional `fields` option, restricting the rule to named declared pointer/narrative fields; the rule's row in the generated table is updated. **Why:** one `not_in` list judged lineage (`Derives-from`) and dependency (`Implements`) fields alike, so a Superseded lineage target warned as stale (`BUG-172`). Omitted, the rule is unchanged. | **substantive** |
> | 2026-11-03 | `header.case-variant-key` added to the generated rule table: an opt-in, declaration-free rule reporting two keys in one record's header that differ only by case. **Why:** `MILE-113` -- a type declaring no vocabulary had nothing that flagged a `Status`/`status` pair, and `ADR-57` rules out making `duplicate_keys()` case-insensitive, so the signal is a separate rule that changes no exact-match verdict. | **substantive** |
> | 2026-11-03 | Adds `boundary.published-path-reference` to the rule table, with its required `published_paths` option, a list of path prefixes (never globs; a glob is treated literally), normalised at config load (leading `./`, trailing `/` and whitespace removed; an absolute entry, a `..` component or a backslash is a config error). An identifier split across lines is not detected. **Why:** `RFC-11` (decided in `ADR-9`) declares a set of paths that publish beyond the repository, and a record identifier cited from inside one is a violation (`MILE-125`). The rule's `level` is the boundary's severity; a declared path covering no tracked file is itself a finding (a prohibition fails closed), and a file that is not text is counted unreadable, never clean. | **substantive** |
> | 2026-11-04 | Criterion 5 now reads "one adopter's codebase" and "that adopter's hand-written linter" in place of "one source codebase"; criterion 6 is marked as part of the Phase 1 exit, so "all five hold" is now the full Phase 0 set (`SPEC-1`, Exit bar); `Version` `0.11` -> `0.12`. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | Bumped to `0.14`. `relation.pair-reciprocity` and `relation.cross-type-disallowed` added to the generated rule table; both require declared `relation_pairs` (`SPEC-3`) and `relation.supersession-reciprocity` is unchanged beside them. **Why:** `MILE-122` -- reciprocity over any declared pair of inverse fields, including across record types (`RFC-47`). | **substantive** |
> | 2026-11-04 | `citation.resolution` added to the generated rule table: an opt-in rule reporting a decision ID cited in a record's prose that resolves to no record, with a population counted in citations (new `citation` population unit). **Why:** `MILE-136` -- `pointer.resolution` reads only declared header fields, so a dangling prose citation passed every rule. | **substantive** |
> | 2026-11-04 | Bumped to `0.17`. `index.freshness` added to the generated rule table: an opt-in rule comparing a committed index of one record type (a whole file, or a list between declared `begin`/`end` marker lines in a README) with the records, in a declared line `format` with `{id}`, `{file}` and `{title}` placeholders. It reports a record with no entry, an entry naming no record, a duplicated entry, a stale file name or title, a line the format does not match, a missing artifact and a missing marker pair; it never regenerates anything and does not compare order. Its required `indexes` option (`path`, `type`, `format`, optional `begin`/`end`) is validated at config load, and its population counts the artifacts compared. **Why:** `MILE-139` -- a repository that commits an index had nothing that stopped it going stale. | **substantive** |
> | 2026-11-03 | Bumped to `0.13`. `git-diff` marked produced: `urzua check --base <ref>` reports only on records changed since the merge base of the ref and `HEAD` (working tree included), `scope` gains an always-present `base` (`null` unless `git-diff`), and §Discovery states the mode's contract, including that it is refused alongside an untracked named file. **Why:** `MILE-123` -- a pull-request gate over a large corpus should not sweep every record, and the table had reserved the value while marking it unbuilt so no consumer would branch on it before a run could produce it. | **substantive** |
> | 2026-11-04 | `git-diff` scope examines the changed records **plus their dependents**, not the changed files alone: unchanged records that name a changed or deleted record's id, share a changed id, cite changed or deleted `Realized-by` evidence, or share a normalised `Realized-by` locator with a changed or deleted record. A configuration change puts every record in scope; an empty diff is `ok` with an info notice rather than `not-run`; `scope` gains `changed_paths`, `dependents` and `whole_corpus_reason` on `git-diff` runs; a ref with no merge base names that in its error. `Version` `0.13` stays (the `--base` row above is the same unreleased change). **Why:** review of `MILE-123` reproduced a hole -- deleting a target left its referrer unexamined, so `check --base` reported nothing where a full check reported `pointer.resolution` -- and an empty diff that could not be told from a clean pass of the whole corpus. | **substantive** |
> | 2026-11-04 | `git-diff` scope also treats as dependents the records covered by a changed or deleted waiver's old and new `Scope`; splits header values on commas when finding references; keeps `type.record-outside-declared-dir` findings for changed files; treats a symlinked configuration as changed by either path; an empty diff with no rule enabled is `not-run`. §Output `status` states the empty git-diff exception. **Why:** late review of `MILE-123` reproduced each as a case where `check --base` hid or mis-stated what a full check reports. | **substantive** |
> | 2026-11-04 | Bumped to `0.18`. `section.required` and `section.shape` added to the generated rule table: opt-in rules reading a type's declared `sections.required` and `exempt_statuses` (`SPEC-3`), with a population counted in `(record, required section)` slots (new `section` population unit). **Why:** `MILE-4`, `MILE-5`, `MILE-7` and `MILE-121` -- SPEC-2 named required sections per type as a rule category and nothing built it. | **substantive** |
> | 2026-11-04 | `Embodiment` recomputed by `urzua fix`: `Drift detected`. **Why:** every file named in `Realized-by` has changed since this line was last verified, and this pass did not re-verify it, so the record states the drift instead of a stale `Verified`; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/commands/check.rs, test:rust/crates/urzua-cli/tests/check_integration.rs` -- stated `Verified` disagreed with the computed value. | **structural** |
> | 2026-11-04 | Bumped to `0.16`. Adds `diff.source-requires-record` and `diff.source-and-records-mixed` to the generated rule table, a new `not-applicable` `RuleStatus` and `changed-path` population unit, and the `source_paths`, `application_paths` and `exclude_paths` rule options (`SPEC-3`'s path-option normalisation). **Why:** `MILE-124` and `MILE-142` -- two diff checks an adopter's hand-written linter makes, source change without a record change and a change mixing records with application source, need a declared home; they run only on `--base` and say so otherwise rather than passing. | **substantive** |
> | 2026-11-04 | Bumped to `0.15`. `header.field-set-consistency` gains the optional `allow_extension_prefix` option, off unless declared `true`: a header key beginning `x-` in any case is exempt from the rule, and an unprefixed undeclared key is still a finding. The option is declared per rule, not per type, so a type's `known_fields` still lists every field the rule judges by default; `pointer.resolution` and `narrative-field.stale` read only declared pointer and narrative fields, so a prefixed key is never one of them unless declared. The 'Not yet built' entry for the `x-` escape hatch is removed, and the rule-table row is regenerated from `RULE_METADATA`. **Why:** `MILE-141` -- the adopter's linter admits `x-` keys through its closed key set, so without an equivalent a repository either declares every local extension in config or loses typo detection on real keys. | **substantive** |
> | 2026-11-04 | `filename.identity-pattern` added to the generated rule table: an opt-in rule reporting a file directly in a type's `dir` whose name yields no identity under the type's declared `identity.pattern` (or the `TYPE-NNNN-slug.md` convention when none is declared). When every file of a type is off-pattern it reports once on the directory with each file's expected name; `audit` runs it too. **Why:** `MILE-120` -- such a file was a loaded record every identity-reading rule skipped in silence, and `RFC-46` was rejected, so renaming is the supported route. | **substantive** |
> | 2026-11-04 | `status.undeclared-value`, `status.required-field-missing` and `status.forbidden-field-present` added to the generated rule table: opt-in rules reading a type's declared `statuses` (`SPEC-3`). `field.pending`, `narrative-field.stale` and `claim.status-agreement` now read a declared type's own pending vocabulary and success/failure terminals in place of the compiled-in token and the rule's hand-copied status list; a type declaring nothing is judged as before. **Why:** `MILE-132` Phase 0, closing `BUG-65` and `BUG-178`. | **substantive** |
> | 2026-11-04 | Bumped to `0.19`. `embodiment.back-pointer-resolution` added to the generated rule table, with its required `back_pointer_paths` option (path prefixes, normalised at config load like `published_paths`): an `Implements: <id>` source comment is code-tier evidence for the cited record in `embodiment.consistency` and `urzua fix`, and a back-pointer citing no record is reported. Evidence is collected only while the rule is enabled; its findings are kept under `check <path>` scoping (`RULES_REPORTING_OUTSIDE_THE_CORPUS`). **Why:** `MILE-128` -- a first adopter relies on such comments, so the evidence model needed a place for them (`ADR-66`). | **substantive** |
> | 2026-11-04 | Bumped to `0.20`. `embodiment.consistency` gains three opt-in refinements, off by default: a `@sha256:` pin on a `Realized-by` locator replaces the git-history heuristic for that locator, `drift_grace_days` holds back drift newer than the window, and `claim_max_age_days` warns on an old claim; the generated rule table row carries them. **Why:** `MILE-129` -- unconditional drift had no sanctioned way to say a change was re-verified or not yet due. | **substantive** |
> | 2026-11-04 | Bumped to `0.24`. `embodiment.requires-accepted-status` added to the generated rule table: an opt-in rule reporting a record whose `Status` is not an accepted status but whose stated `Embodiment` is `Implemented` or `Verified`. Accepted means the type's declared `statuses.success`; a type declaring none uses the rule's optional `accepted_statuses` option, and with neither it is not examined. Its population counts the records the rule could judge. **Why:** `MILE-138` -- `embodiment.consistency` never read `Status`, so a proposed record could claim to be built and pass. | **substantive** |
> | 2026-11-04 | Bumped to `0.23`. `date.field-valid` and `date.acceptance-by-status` added to the generated rule table: opt-in rules that validate declared date fields as real `YYYY-MM-DD` calendar dates and require the acceptance date exactly on accepted-type statuses, with the population counting the date values or records examined. **Why:** `MILE-137` Phase 0: a replacement linter must reject `2026-13-45` and an acceptance date on a record nobody accepted. | **substantive** |
> | 2026-11-04 | Bumped to `0.21`. `embodiment.back-pointer-decision-accepted` added to the generated rule table: an opt-in rule reporting a source back-pointer that cites a record whose `Status` is not an accepted one, where accepted is the cited type's declared `statuses.success` or, for a type declaring none, the rule's optional `accepted_statuses`; with neither the status cannot be judged and that is reported. It takes the same `back_pointer_paths`, reads back-pointers exactly as `embodiment.back-pointer-resolution` does, and on a `--base` run judges only the files the diff changed. Its findings are kept under `check <path>` scoping. **Why:** `MILE-133` -- implementation citing a decision must not land ahead of it. | **substantive** |
> | 2026-11-04 | Bumped to `0.22`. `field.quality`, `field.pending` and `status.required-field-missing` gain the optional `field_levels` option: a finding for a listed field has that field's `warn` or `error` level, any other finding has the rule's `level`, and `check` keeps applying the rule level to every rule that declares no per-field level. **Why:** `MILE-140` -- one rule needs two severities for two fields. Off unless declared; `SPEC-3` states the option. | **substantive** |
> | 2026-11-04 | Bumped to `0.24`. `date.acceptance-by-status` removed from the generated rule table. **Why:** maintainer decision of 2026-11-04 (`MILE-137`): the acceptance date by status is expressed with `statuses.require` / `statuses.forbid` and `date.field-valid`, so one mechanism owns status-dependent fields. | **substantive** |
> | 2026-11-04 | Bumped to `0.25`. `embodiment.back-pointer-decision-accepted` now judges a back-pointer against the cited type's declared `statuses.implementable` rather than its terminal `success`; a type declaring none uses the rule's `accepted_statuses`, and with neither the status is reported as unjudgeable. Rule id unchanged. **Why:** the engine holds no type knowledge, so a terminal success status cannot stand for "implementation allowed" (`MILE-133`, `SPEC-3` 0.13). | **substantive** |
> | 2026-11-04 | Bumped to `0.26`. `embodiment.requires-accepted-status` also allows the claim at the type's declared `statuses.claim_retained`, not only `success`. Rule id unchanged. **Why:** maintainer decision of 2026-11-04, `MILE-138`: a record may keep an Implemented or Verified claim in a status reached only after acceptance. | **substantive** |
> | 2026-11-04 | Bumped to `0.27`. Inside `drift_grace_days`, `embodiment.consistency` accepts either the tier value or a stated `Drift detected` (the window only holds back reporting, so it never contradicts a record that already says so) and `urzua fix` leaves such a record alone; drift outside the window still computes `Drift detected`. The generated rule table row says so. **Why:** `BUG-179`. | **substantive** |
> | 2026-11-04 | Bumped to `0.28`. Exit code 1 also covers an unwaived warning when `fail_on: warning` or `--fail-on warning` is in force; the report itself is unchanged. **Why:** `MILE-145` -- a repository at zero warnings needs warnings to fail its own build. | **substantive** |
> | 2026-11-04 | Bumped to `0.29`. `citation.resolution` takes an optional `external_ids` option (exact ids or inclusive numeric ranges under a declared prefix); a matching citation is counted out of scope and not resolved. The rule table is regenerated. **Why:** `MILE-156` -- a public standard number sharing a record prefix (`RFC-4122`) was reported as dangling. | **substantive** |
> | 2026-11-04 | Bumped to `0.30`. `status.required-field-missing` also counts a whole value of the type's declared `statuses.pending` as missing. **Why:** `MILE-153` -- the declared placeholder vocabulary was read by `field.pending` only, so an accepted record reading `Unknown` satisfied a requirement. | **substantive** |
> | 2026-11-04 | Bumped to `0.31`. Documented that a file a type lists in `exclude_files` is dropped at discovery, and that every finding's `file`, a config-level one included, is repository-relative. `filename.identity-pattern`'s generated row says its census omits an excluded file. **Why:** `MILE-150`, `BUG-183` and `BUG-185`. | **substantive** |
> | 2026-11-04 | Bumped to `0.32`. `relation.pair-reciprocity` reports an unechoed claim from a claimant at a pair's `provisional.statuses` at the provisional level (default `warn`) and any other claimant at the rule's level. **Why:** `MILE-154` -- one severity for a Proposed record's claim and a broken accepted claim was either too loud or too quiet. | **substantive** |
> | 2026-11-04 | Bumped to `0.33`. The back-pointer matcher reads a full id list and trailing markers: `Implements: ADR-1 (note), ADR-2` yields both ids, a marker after code on the same line (`//`, `#`, `--`, `/*`, `<!--`) is read, and reading still stops at a segment that does not open with an id (`ADR-1 and ADR-2` yields `ADR-1`). `embodiment.back-pointer-decision-accepted` judges the same widened set. **Why:** `MILE-147` -- a matcher that stopped at the first non-id token or ignored a same-line marker dropped citations an author wrote, so a record lost code evidence it had. | **substantive** |
> | 2026-11-04 | Bumped to `0.34`. `embodiment.back-pointer-resolution` gains the optional `test_paths` option and reads `Verifies:` in those files as test-tier evidence; `embodiment.consistency` computes `Verified` from it, judges a record with no `Realized-by` field that a back-pointer cites (the finding names the evidence), and `urzua fix` repairs the same records. Rule table regenerated. **Why:** `MILE-146` and `MILE-149` -- evidence is read from the artifact instead of restated beside it. | **substantive** |
> | 2026-11-04 | Bumped to `0.35`. `relation.pair-reciprocity` excludes a claim to a record type outside a pair's declared `types` as out of scope. **Why:** `MILE-155` -- a pair between one type's records erred on a claim to a type that cannot hold the inverse field. | **substantive** |
> | 2026-11-04 | Bumped to `0.36`. `embodiment.consistency` gains the optional `sticky_states` option: a record stating a listed `Embodiment` value is examined and not reported as drift or as an old claim, and `urzua fix` leaves it alone. Rule table regenerated. **Why:** `MILE-148` -- `Deprecated` and `Inactive` are states a human sets and nothing can compute, so a finding the author cannot resolve by changing the artifact only trains readers to ignore the rule. | **substantive** |
> | 2026-11-04 | Bumped to `0.37`. The diff rules' path options accept globs, and `diff.source-requires-record` also takes `exclude_paths`; an excluded changed path is eligible and unexamined. **Why:** `MILE-158` -- prefix-only options could not say "every source file except tests", so a test-only change tripped the gate. | **substantive** |
> | 2026-11-04 | Bumped to `0.38`. Added the opt-in `citation.path-exists` rule: a backtick-quoted repository path in a declared reference section must exist in the working tree (population unit: the cited path). The rule table is regenerated. **Why:** `MILE-143` -- `embodiment.locator-exists` covers only `Realized-by`, so a stale path cited in References passed every rule. Cited issue and pull request numbers remain unbuilt. | **substantive** |
> | 2026-11-04 | Bumped to `0.39`. Added `date.field-order` to the generated rule table: an opt-in rule over a `date_order` entry (`Earlier <= Later`). **Why:** `MILE-157` -- cross-field date ordering, with well-formedness left to `date.field-valid`. | **substantive** |
> | 2026-11-04 | Bumped to `0.40`. `index.freshness` reads multi-line entries (a `format` of several lines), accepts a `format` with `{file}` and no `{id}`, and `{title}` now excludes the record's own id as well as a leading number; the rule table is regenerated. A new `urzua index [--apply]` command regenerates the declared indexes. **Why:** `MILE-152` -- index checking read one line per entry and required a visible id, and nothing regenerated an index. | **substantive** |
> | 2026-11-04 | Bumped to `0.41`. `type.dir-matches-nothing` also reports a type's `exclude_files` entry that names no tracked file directly in its `dir`. Rule table regenerated. **Why:** `MILE-150` -- a stale exclusion is a declaration that silently does nothing. | **substantive** |
> | 2026-11-04 | Bumped to `0.42`. `relation.pair-reciprocity` counts a claim to a record type outside a pair's declared `types` as eligible and unexamined instead of out of scope, so `config.scope-matches-nothing` no longer warns on it. **Why:** `MILE-155` -- the claim is legitimate; the configuration is not what failed to reach it. | **substantive** |
> | 2026-11-04 | Bumped to `0.43`. `citation.path-exists` takes an optional `skip_tables` option (a table whose rows are history is not read), and `citation.resolution` counts a declared external id as eligible and unexamined rather than out of scope; the rule table is regenerated, and the `citation.resolution` row no longer writes its numeric-range example as a citable id. **Why:** `MILE-143` -- revision-log rows name paths as they were; `BUG-186` -- a declared external id was reported as a misconfiguration. | **substantive** |
> | 2026-11-04 | Bumped to `0.44`. Removed the `header.layout-consistency` and `header.deprecated-shape` rules from the generated rule table: with the `blockquote` and `bold-list` shapes gone there is no layout to declare and no deprecated shape a config can hold. A record still written in a removed form is reported by `header.required-fields` with the form named, and naming either removed rule under `rules:` is an unknown-rule load error. **Why:** `MILE-85`. | **substantive** |
> | 2026-11-04 | Bumped to `0.45`. Added `waiver.matches-nothing` to the generated rule table: an opt-in rule reporting an expired waiver, a waiver naming an unknown rule, and an active waiver whose rule ran over the whole corpus and found nothing in its scope; a path-scoped or `--base` run does not judge a live waiver. **Why:** `MILE-168` -- a stale waiver stayed on file silently, the no-silent-no-op principle applied to the waiver record. | **substantive** |
> | 2026-11-04 | Bumped to `0.46`. `embodiment.consistency` reports a shallow clone and failing git calls as `embodiment-drift-git` notices instead of reading them as no drift. **Why:** `MILE-159` -- a shallow clone once produced a false zero. | **substantive** |
> | 2026-11-04 | Bumped to `0.47`. `revision-log.change-class-required` finds the log by the type's declared `revision_log` identity (default the line `**Revision log**`), not by any line containing the text; `citation.path-exists` takes `skip_revision_log` in place of `skip_tables`. Rule table regenerated. **Why:** `MILE-167` -- prose mentioning the literal made the rule read the wrong table. | **substantive** |
> | 2026-11-04 | Bumped to `0.48`. `section.required` and `section.shape` count a record exempted by `exempt_statuses` as eligible and unexamined instead of out of scope; the rule table is regenerated. **Why:** `BUG-187` -- the exemption is declared, and `config.scope-matches-nothing` warned about it as a misconfiguration. | **substantive** |
> | 2026-11-04 | Bumped to `0.49`. `embodiment.back-pointer-resolution`'s `test_paths` accepts globs as well as prefixes, and a glob matching no scanned file is a finding. **Why:** `MILE-160` -- test files named by suffix cannot be named by a prefix list. | **substantive** |
> | 2026-11-04 | Bumped to `0.50`. `embodiment.consistency` judges a record stating `Verified` or `Implemented` with no `Realized-by` field and no citing back-pointer, reporting it against the computed `Not started`; a record stating no tier stays unexamined. **Why:** `MILE-161` -- a claim with no evidence passed silently. | **substantive** |
> | 2026-11-04 | Bumped to `0.51`. `embodiment.consistency` takes the optional `drift_audit_field`: the `drift_grace_days` window counts from that declared last-audit date on a record when it has one, else from the cited file's last change, and a malformed date is a warning. **Why:** `MILE-166` -- a file changes for reasons unrelated to its evidence. | **substantive** |
> | 2026-11-04 | Bumped to `0.52`. `field.pending` and `status.required-field-missing` also read the pattern entries of `statuses.pending`; the rule table is regenerated. **Why:** `MILE-162` -- a placeholder family is declared once. | **substantive** |
> | 2026-11-04 | Bumped to `0.53`. `relation.pair-reciprocity` does not report a provisional claim whose peer names the claimant in the pair's declared `provisional.acknowledged_by` field; the rule table is regenerated. **Why:** `MILE-163`. | **substantive** |
> | 2026-11-04 | Bumped to `0.54`. `index.freshness` also compares declared-field placeholders (`{Status}`, `{Date}`) with the record's header and reports a differing or missing value. Rule table regenerated. **Why:** `MILE-164`. | **substantive** |
> | 2026-11-04 | Bumped to `0.55`. `section.required` and `section.shape` follow a required section's declared `aliases` and read a marked `y-statement` from the preamble when `preamble: true`; `check` adds an info notice per alias in use. Rule table regenerated. **Why:** `MILE-165`. | **substantive** |
> | 2026-11-04 | Bumped to `0.56`. `citation.resolution` takes the optional `skip_revision_log` option; the rule table is regenerated. **Why:** `MILE-156` -- append-only revision-log history cites ids as they were. | **substantive** |
> | 2026-11-04 | Bumped to `0.57`. `field.pending` gains the optional `pending_until_required` option, off unless declared `true`; the rule-table row is regenerated from `RULE_METADATA`. **Why:** `MILE-169`; `SPEC-3` states the option. | **substantive** |
> | 2026-11-04 | Bumped to `0.58`. Added the opt-in `citation.issue-exists` rule: an issue or pull request cited in a declared reference section (`#123`, or a link to the declared repository) must be in the committed snapshot written by `urzua refs refresh`; an absent, unreadable or foreign snapshot is one finding on the snapshot path and every citation is counted eligible and unexamined; every finding names the snapshot's generated date and `check` adds an info notice with it. `check` reads the file and never the network. The rule table is regenerated and the snapshot finding is exempt from path scoping. **Why:** `MILE-143` -- the issue and pull request half. | **substantive** |
> | 2026-11-04 | Bumped to `0.59`. Added the opt-in `status.delivery-agreement` rule to the generated rule table: a `Status` held to the delivery evidence the type declares in `statuses.delivery`, both ways. **Why:** `MILE-183`. | **substantive** |
> | 2026-11-04 | Bumped to `0.60`. `citation.issue-exists` also reads the header fields named by the optional `issue_refs.fields` (a finding names the field and its line); the rule-table row is regenerated. Off unless declared. **Why:** `MILE-184` -- numbers in header fields such as a BUG's `Found-in` were unchecked. | **substantive** |
> | 2026-11-04 | Bumped to `0.61`. The shipped rule set is `RULE_TABLE`, one entry per rule carrying its description and properties; `RULE_METADATA`, `RULES_REPORTING_OUTSIDE_THE_CORPUS`, `IDENTITY_DEPENDENT_RULES` and `RULES_REQUIRING_RELATION_PAIRS` are derived from it. No rule or output changes. **Why:** `MILE-171`: a rule's properties lived in parallel lists. | **structural** |
> | 2026-11-04 | Bumped to `0.62`. The rule table, `urzua rules` and `rules_executed` list rules in one order: the order `check` runs them, with `config.scope-matches-nothing` and `waiver.matches-nothing` last. `check` output is unchanged. **Why:** `MILE-171` step 2: `check` now runs the table, so the table's order is its execution order. | **structural** |
> | 2026-11-04 | Bumped to `0.63`. A path argument and `--base` keep or drop a finding by the anchor its rule's table entry declares (a record, the configuration, a file outside the corpus, a tracked path, a type's directory, or a record plus a file the rule's options declare), replacing the rule ids `check` named. No finding is kept or dropped differently. **Why:** `MILE-171` step 3: each new rule that reported on a non-record path had to be added to a list by hand or be silently dropped from a narrowed run. | **structural** |
> | 2026-11-04 | Bumped to `0.64`. `config.relation-field-not-known` also checks the `date` and `author` fields a type declares under `relation_fields` (`MILE-176`): one not in the type's `required_fields` or `known_fields` is a finding, as an undeclared relation role already was. **Why:** `urzua new` fills those fields by name, so an undeclared one was written to a header the rest of the engine never examines. | **substantive** |
> | 2026-11-04 | Bumped to `0.65`. A type's records are the files directly in its `dir` and each `extra_dirs` entry; `type.dir-matches-nothing` also reports an `extra_dirs` entry that does not exist. Rule table regenerated. **Why:** `MILE-130` -- one identity namespace across several directories. | **substantive** |
> | 2026-11-04 | Bumped to `0.66`. `section.required` and `section.shape` read a required section declared at a deeper `level`, optionally `under` a named parent, so a `###` heading is visible to them; the rule table is regenerated. **Why:** `MILE-98`. | **substantive** |
> | 2026-11-04 | Bumped to `0.67`. Added `config.companion-declaration-missing` to the generated rule table: a type declaring one field of a declared `companion_fields` pair without the other is reported. **Why:** `BUG-41` -- a type declaring `Embodiment` without `Realized-by` let records claim a state `embodiment.consistency` could not check. | **substantive** |
> | 2026-11-04 | Bumped to `0.68`. The rule table gains `status.transition-allowed`, which compares a changed record's `Status` at the merge base with the change under check and is not-applicable without `--base`, and `status.unreachable-state`. **Why:** `MILE-144`, `ADR-72`. | **substantive** |
> | 2026-11-04 | Bumped to `0.69`. `citation.resolution` takes an optional `non_reference_field` option: a declared id in the named header field is not resolved or reported in that record, a declared id never cited and a malformed entry are findings, and `check` prints one aggregated Info notice for declared ids that now match a real record. The rule table is regenerated. **Why:** `BUG-190`, decided by `ADR-75`. | **substantive** |
> | 2026-11-04 | Bumped to `0.70`. A finding may carry `suggested_action` (`ADR-73`, `MILE-44`): a `kind` from a vocabulary closed per release, an `op` where the kind has several, a flat `target` and a `value` only where one is derivable, each omitted rather than `null` when not computed. The output contract reproduces `ADR-73`'s per-kind table and compatibility rule; each rule's table entry declares an action or a reasoned exemption for every finding variant it emits, `urzua rules` lists them as `actions`, and the generated action table below the contract is read from the same entries. **Why:** `MILE-44` -- a finding said what was wrong and never what to do; additive to `check` JSON. | **substantive** |
> | 2026-11-04 | Bumped to `0.71`. `field.quality` gains the optional `template_placeholders` option, off unless declared `true`; the rule-table row is regenerated from `RULE_METADATA`. **Why:** `BUG-22`; `SPEC-3` states the option. | **substantive** |
> | 2026-11-04 | Bumped to `0.72`. `sequence.cycle` and `sequence.unresolved-blocker` added to the generated rule table; both read only `relation_pairs` entries that set `acyclic: true` (`SPEC-3`), and enabling either with no such pair is a config load error. **Why:** `MILE-27` -- the sequencing relation `RFC-48` section 1 proposes. | **substantive** |
> | 2026-11-04 | Bumped to `0.73`. Added `revision-log.version-bump-required` to the rule table (regenerated from `RULE_METADATA`): a diff rule that compares a changed record's revision log and version field against the merge base's copy, read by the CLI and handed to the rules as data. **Why:** `BUG-51`; `SPEC-3` states the declaration. | **substantive** |
> | 2026-11-05 | Bumped to `0.74`. `waiver.matches-nothing` gains the finding variant `waiver-ineffective` (`edit-field` `set` on the first missing field), reported on a waiver that lacks a field its type's `effective_requires` names; the waiver suppresses nothing. **Why:** `BUG-177`, maintainer decision of 2026-11-05. | **substantive** |
