---
Version: '0.28'
Date: 2026-08-20
Status: Accepted
Embodiment: Drift detected
Realized-by: code:rust/crates/urzua-core/src/config.rs, test:rust/crates/urzua-core/src/config.rs
Author: beauwilliams
Subject: '`.urzua/config.yaml` -- the schema for record types, required/known fields, and per-type behavior.'
Implements: RFC-1, RFC-11
Parent: SPEC-1
---
# SPEC-3 — Configuration: `.urzua/config.yaml`

## Purpose

Config-driven is the founding claim, and this file is where it is falsifiable. The same binary must
serve two codebases with different layouts, record types and role requirements without either
forking it. If v0 needs a code change to work in the second codebase, the "build it once instead of
three times" claim is in trouble — and that is a finding about the schema, not a config bug.

That makes this spec's real job to be **specific enough to fail**. A config vague enough to express
anything proves nothing.

**Phasing.** SPEC-2 Phase A needs only record types and their directories — a real file, so the
config path exists from the first commit, but not this surface. Required fields, roles, boundaries
and rule severities land in Phase B; `doctor` with it. The falsifiable claim above is tested in
Phase C, against the second codebase, and cannot be tested before then.

## Resolution

`.urzua/config.yaml`, written by `urzua init` (SPEC-5); `--config` overrides. The nearest
`.urzua/` at or above the cwd wins, so the tool works from a subdirectory of a monorepo without
arguments.

The config sits in a directory rather than at the root because it is not the only thing the tool
owns: templates live beside it at `.urzua/templates/`, and derived state at `.urzua/cache/`, which
is gitignored. Templates in particular must not live inside the corpus — a template is invalid *as
a record* by construction, so a checker discovers it and reports errors on a file that is correct.
SPEC-5 §Layout carries the full argument.

**One config per repo, not per package.** Per-package configs would let two packages disagree about
what a record *is*, and the cross-package questions — does this identifier resolve, do these scopes
overlap — become unanswerable when each side is validated under different rules. Per-package
variation is expressed as scoped overrides within the one file, which keeps the disagreement
visible in a single diff.

## What is configurable

```yaml
record_type:
  - name: "adr"
    dir: "docs/adr"
    filename: "{number}-{slug}.md"
    statuses: ['Proposed', 'Accepted', 'Rejected', 'Withdrawn', 'Superseded', 'Deprecated']
    header_shape: "blockquote"
    required_fields: {'always': ['Status', 'Date', 'Author', 'Deciders'], 'Accepted': ['Accepted']}
    roles: {'author_may_self_review': False}
boundary:
  - paths: ['packages/sdk/**']
    kind: "published"
    reason: "Published to the registry; identifiers here resolve to nothing for external readers."
rules:
  header.duplicate-key: "error"
  scope.matches-nothing: "warn"
```

Configurable: record types and their directories, filename patterns, status enums, required fields
**per type and per status**, role requirements, back-pointer patterns, boundaries, and the severity
of every rule.

`init` materializes these from a built-in profile rather than referencing one by name, so every
rule in force is a visible line someone can disagree with (SPEC-5 §Type selection).

Two details carry weight:

- **Required fields are keyed by status, not only by type.** A field that only exists once a
  decision has been accepted is the common case, and a config that can only say "required" or "not"
  forces every corpus to choose between a false error and no check at all.
- **Every rule's severity is configurable, but no rule can be configured out of existence.** A rule
  set for a corpus is a claim about that corpus; silently dropping a rule and reporting `ok` is the
  failure this project exists to eliminate. Downgrade to `off` is permitted and is *reported* in
  `rulesExecuted` as skipped, never omitted.
- **The header shape is declared per record type, never auto-detected.** RFC-10 rejects sniffing a
  shape from content, for the same reason a rule can't be voted into existence by majority: a
  misconfigured shape must fail loud (`no header-shaped region found`) rather than silently matching
  the wrong lines. `header_shape` defaults to `"blockquote"` when omitted, so an existing config
  needs no change. `"yaml-frontmatter"` (RFC-16) is genuinely structured data, deserialized rather
  than line-recognized, and is what a from-scratch record generates by default; the other shapes stay
  what an adopted, pre-existing corpus is read as.
- **A record type may declare the files in its directory that are not its records:
  `exclude_files`** (`MILE-150`). A list of bare file names, matched exactly against the files
  directly in the type's `dir`, for a file that lives beside the records without being one (an index
  `README.md`, a `_template.md`):

  ```yaml
  record_types:
    adr:
      dir: docs/adr
      exclude_files: ["README.md"]
  ```

  A named file is never loaded as a record, so it is in no rule's population and not in
  `filename.identity-pattern`'s file census, and a change to it is not a change to a governed record
  for the diff gate. Only the named file is excluded: any other file in the directory that is not a
  record is still a record that failed to match, and `filename.identity-pattern` still reports it.
  An entry that is empty, `.`, `..`, or holds a path separator or a glob character (`/`, `\`, `*`,
  `?`, `[`) is a load-time error naming the record type, because it would exclude nothing. Omitted
  or empty, the type loads as it always did.
- **A record type may declare its document model: `identity`, `fields` and `sections`**
  (`RFC-33` layer 1, `MILE-98`). Three optional keys under a record type, each a mapping with a
  required `from`:

  ```yaml
  record_types:
    adr:
      dir: doc/adr
      identity: {from: filename, pattern: '^(?P<number>\d+)-(?P<slug>.+)$'}
      fields:   {from: prefix-lines}      # or yaml-frontmatter, blockquote, bold-list, none
      sections: {from: headings, depth: 2, items: true}
  ```

  `identity.pattern` is a regular expression over the file name without `.md`; it must compile and
  must capture the record number as `(?P<number>\d+)`. `sections.depth` counts heading levels from `##`
  (1 is `##` only, 2 adds `###`, up to 5) and defaults to 1; `items` defaults to `false`. An unknown
  `from`, an unknown key, an out-of-range `depth`, a non-boolean `items`, a blank or non-compiling
  `pattern` or a key present with no value is a load-time error naming the record type and key.

  **`identity` and `fields` drive record loading; `sections` is read by `section.required` and
  `section.shape` only.** A type that declares none of the three loads, and is checked, exactly as it
  did before the declaration existed.

  - **`identity`.** The pattern, not the `TYPE-N-slug.md` convention, decides a record's identifier:
    `{prefix}-{number}`, with the type's `prefix` and the `number` capture as written, so
    `ADR-0001-slug.md` is `ADR-0001` and compares equal to `ADR-1`. The pattern must capture both
    `(?P<prefix>...)` and `(?P<number>...)`: a pattern without a `prefix` group is a load-time error
    naming the record type, because a bare `0001-slug.md` gets no identity (`RFC-46` was rejected; such
    a corpus is renamed to `TYPE-NNNN-slug.md`, and `MILE-120` reports the mismatch). A file name the
    pattern does not match, or whose `number` is not ASCII digits within a `u64`, has no identifier.
    The `prefix` capture also declares the type's H1 title form (`RFC-44`): the title opens
    `PREFIX-N`. A declaring type that breaks its own title form is a `filename.title-consistency`
    finding. `urzua new` reads the declaration too: it numbers by the pattern, fits the file name's
    zero-padding to it and opens the H1 with `PREFIX-N` (`SPEC-12`, `MILE-151`).
  - **`fields.from`.** Decides how header fields are read, overriding `header_shape`: `yaml-frontmatter`,
    `blockquote`, `bold-list` and `none` are the existing shapes, and `prefix-lines` reads bare
    `Key: value` lines between the title and the first `##` heading (a key is an ASCII letter then
    letters, digits, `-` or `_`; the colon is followed by a space or ends the line). `none` gates as
    `header_shape: none` does (`RecordTypeConfig::has_no_header`).

  - **`sections.required`** (`MILE-4`, `MILE-5`, `MILE-121`). A list of the `##` sections a record of
    the type must carry, each `{name: ...}` matched exactly against the heading text after trimming,
    with an optional content `shape`:

    ```yaml
    sections:
      from: headings
      required:
        - {name: Context}
        - {name: Options considered, shape: table-min-rows, min_rows: 2}
        - {name: Decision, shape: y-statement, marker: 'TL;DR'}
      exempt_statuses: [Withdrawn]
    ```

    `section.required` reports a missing section; `section.shape` checks the shape of a present one.
    Both are opt-in and a type that declares no `required` has nothing eligible. Their population
    unit is `section`: one `(record, required section)` slot, so a record missing a section is
    counted. A heading inside a fenced code block is not a section, and sections are the `##` level
    only (`parse_sections` is not yet driven by `depth`).
    - `y-statement` reads one block and requires `In the context of`, `facing`, `we decided`,
      `to achieve` and `accepting` in it, matched case-insensitively on word boundaries (the phrase
      set is fixed, not declarable). With a `marker`, the block starts at the first line of the
      section containing it and runs to the next blank line or list item; a section without the
      marker, and a marker with no block after it, are each their own finding. With no `marker`,
      the block is the section's first paragraph. A leading `>` blockquote marker on a line is not part of
      the block, so a phrase that wraps across quoted lines still matches. Phrases outside the block never count.
    - `table-min-rows` requires a table with at least `min_rows` (an integer of 1 or more) data rows;
      a data row with every cell empty does not count, the largest table in the section decides, and
      a table inside a fenced code block is not read.
    - `exempt_statuses` lists statuses of a record that made no decision (`MILE-7`). Such a record is
      counted out of scope by both rules, not asked for the sections. It is matched exactly against
      the type's declared status field. A record whose header cannot be read is counted unreadable,
      never assumed exempt, and a record with no status is not exempt. Declaring it without
      `required`, a blank status, a duplicate section name, an unknown `shape`, `min_rows` missing
      from or `marker` misplaced on a shape, or an unknown key, is a load-time error.
    - Presence and shape are all either rule knows: neither judges whether a section's content is
      true or sufficient.

  `section::parse_sections` (`section.rs`) splits a record body into its `##` sections with line
  numbers; it is not yet driven by `sections.depth`.

- **A record type may declare its status vocabulary: `statuses`** (`MILE-132`, `BUG-59`, `BUG-65`,
  `BUG-178`). An optional mapping under a record type. The field it describes is the type's `Status`
  role field (`relation_fields.status`, default `Status`):

  ```yaml
  record_types:
    bug:
      statuses:
        values:  [Open, Fixed, WontFix]   # every status the type allows; required
        success: [Fixed]                  # terminal, the work was done
        failure: [WontFix]                # terminal, it ended without being done
        implementable: [Open]             # statuses at which code may implement the record
        claim_retained: []                # statuses that may keep an Implemented/Verified claim
        pending: [pending, "not yet written"]
        require: [{when: [Fixed], fields: [Regression-test]}]
        forbid:  [{when: [Open], fields: [Fixed-in]}]
  ```

  `values` must list at least one status; `success`, `failure` and `implementable` are optional and
  must be subsets of `values`; `success` and `failure` must not overlap. `implementable` names the
  statuses at which implementation of the record is permitted. It is independent of the terminals
  (an in-progress milestone is implementable and not terminal; an accepted decision is both), and
  the engine never infers it from `success`. `claim_retained` (optional, a subset of `values`) names statuses reached only after acceptance that may still carry an `Implemented` or `Verified` embodiment claim, such as `Superseded`; `embodiment.requires-accepted-status` allows the claim at `success` and `claim_retained` statuses. `require` and `forbid` are lists of `{when: [status, ...], fields:
  [field, ...]}`: each `when` entry must be in `values`, and each field must be in the type's
  `required_fields` or `known_fields` when the type declares `known_fields`. A field may not be both
  required and forbidden at one status. A duplicate or blank entry, an unknown key, or a key present
  with no value is a load-time error naming the record type and `statuses`. Transitions between
  statuses are not declared here (`MILE-144`).

  A type that declares no `statuses` behaves exactly as before. A declaration is read by:

  - `status.undeclared-value`: a `Status` outside `values`.
  - `status.required-field-missing` / `status.forbidden-field-present`: a `require` field that is
    blank, unedited template text or, when the type declares `pending`, one of its whole pending
    values (`MILE-153`) at the record's status, and a `forbid` field that carries a value. Both read
    the status by exact comparison, as every other rule does. `pending` is the type's one declared
    placeholder vocabulary: `field.pending` and this rule read the same list, so `Unknown`, a dash
    or `none recorded` can be declared once and no longer satisfies a requirement. A type that omits
    `pending` keeps the old reading, where such a value is present.
  - `field.pending`: when `pending` is declared, a field is pending when its whole value (trimmed,
    case-insensitive) equals one of the entries. It **replaces** the compiled-in match, which treats
    any value containing `pending` as pending (`BUG-65`); a value that merely mentions the word is a
    real value. A type that omits `pending` keeps the compiled-in match.
  - `narrative-field.stale` and `claim.status-agreement`: when the *target's* type declares
    `statuses`, `success` and `failure` together are the finished statuses, in place of the rule's
    `terminal_statuses` / `closed_statuses` list, which stays the vocabulary for every target whose
    type declares none (`BUG-178`). `pointer.target-status`'s `not_in` is a separate policy and is
    unchanged.
  - `embodiment.back-pointer-decision-accepted`: a source back-pointer is judged against the cited
    type's `implementable` list. A type that omits it uses the rule's `accepted_statuses` option;
    with neither, the status is reported as unjudgeable, never passed (`MILE-133`).

## External ids: `citation.resolution`'s `external_ids`

A public standard number can share a record prefix (`RFC-4122` against an `RFC` record prefix) and
would otherwise be reported as a dangling citation. `citation.resolution` takes an optional
`external_ids` list of ids that are not records of this repository (`MILE-156`):

```yaml
rules:
  citation.resolution:
    level: error
    external_ids: ["RFC-4122", "RFC-4000..4999"]
```

- An entry is an exact id (`PREFIX-N`) or an inclusive numeric range (`PREFIX-LOW..HIGH`); numbers
  compare by value, so `RFC-04122` matches `RFC-4122`.
- The prefix must be one a record type declares. An entry under no declared prefix, a malformed
  entry, or a range whose start is after its end is a load-time error: it could never match, and a
  declaration that cannot fire is a silent no-op.
- A matching citation is counted in the rule's population as out of scope, not examined, so it is
  visible in the report and never hidden. An id the list does not name is still resolved against the
  records and reported when it resolves to none. A declared external id wins over a record of the
  same id, so declare only ids no record answers to.

## Cited paths: `citation.path-exists`

`citation.path-exists` (`MILE-143`) checks that a repository path cited in a record's references
exists. Where to look is configuration, not engine knowledge, so the rule requires
`reference_sections`:

```yaml
rules:
  citation.path-exists:
    level: warn
    reference_sections: ["References"]
```

- `reference_sections` lists `##` heading texts, matched exactly after trimming. Only backtick-quoted
  code spans inside those sections are read; a record with no such section is out of scope.
- A span is read as a path only when it is plainly one: no whitespace, a `/` present, ending in a
  file name with an extension or in `/`, not a URL or host-led path, not absolute, and no glob or
  placeholder character. A bare file name without a directory (`Makefile`) is not read. A trailing
  `:line` or `#anchor` is dropped; `./` and `../` resolve from the record's directory.
- A file exists when git tracks it and it is on disk; a directory exists when a tracked file is below
  it. The check is offline and deterministic. Cited issue and pull request numbers are not covered by
  this rule.

## Index artifacts: `index.freshness`'s `indexes`

`index.freshness` compares each declared index with the records of its type (`MILE-139`); `urzua
index` regenerates it (`MILE-152`):

```yaml
rules:
  index.freshness:
    level: error
    indexes:
      - path: "docs/adr/README.md"
        type: adr
        format: "- [{title}]({file})"
        begin: "<!-- urzua:index:begin -->"
        end: "<!-- urzua:index:end -->"
      - path: "docs/specs/README.md"
        type: spec
        format: "- [{id}]({file})\n  {title}"
        begin: "<!-- urzua:index:begin -->"
        end: "<!-- urzua:index:end -->"
```

- `format` writes one entry. Its lines are separated by `\n`, so an entry may span several lines
  (a YAML mapping, or a link followed by a description); the artifact's non-blank lines are read
  that many at a time, and a block scalar's trailing newline is ignored. A blank line inside the
  format is refused, since blank lines are skipped when an index is read.
- Placeholders: `{id}`, `{file}` (the record's file name) and `{title}`. Each appears at most once in
  the whole entry. An entry must name its record, by `{id}` or, when no id is written, by `{file}`;
  `{title}` alone is refused as ambiguous.
- `{title}` is the record's H1 without its identifier: a leading number (`# 12 -- Title`) or the
  record's own id (`# ADR-12: Title`, compared by value) is dropped with the separator after it.
- Only records with an identifier are the type's records: a README in the type's directory is not
  listed unless it is also declared as one.
- `urzua index` is read-only by default and reports each artifact as `current`, `stale` or `missing`;
  `--apply` rewrites the lines between `begin` and `end` (everything else is kept byte for byte), or
  the whole file when no markers are declared, and creates a missing artifact. Entries are in record
  id order, so a second run changes nothing and reports `current`. A record that cannot fill a
  placeholder (no H1 for `{title}`), a file that is not text and a marker that is absent are
  failures naming the artifact, never a partial write.

## Scope declaration

Per RFC-11 §6, a record's scope is a declared field on the record, not a config key — config
supplies only the **default** scope for a record type and the resolver's globbing rules.

The distinction matters: a default written into a new record by `urzua new` is a value an author can
see and correct; a default applied silently at read time is a fact nobody reviewed.

## Exit threshold: `fail_on`

A top-level `fail_on` sets the lowest unwaived finding severity that makes `urzua check` and
`urzua audit` exit 1 (`MILE-145`):

```yaml
fail_on: warning   # error (default) | warning
```

- **`error`** (the default, and the behaviour when the key is absent): only an error finding exits 1.
- **`warning`**: an unwaived warning also exits 1. A waived finding never does.
- Any other value, including a different case, is refused at load, and the command exits 2.
- `urzua check --fail-on error|warning` overrides the key for one run, in either direction.
- Only the exit code changes. The report is byte-identical under either value: `blocking` and
  `status` keep meaning "an error", so a run that exits 1 on a warning shows `"blocking": false`.
  Read the exit code, not `blocking`, to know whether the threshold was crossed.

## Diff rule path options

`diff.source-requires-record` (`source_paths`, `exclude_paths`) and `diff.source-and-records-mixed`
(`application_paths`, `exclude_paths`) take path entries that are either a **prefix** or a **glob**
(`MILE-158`). An entry with none of `*`, `?` or `[` is a prefix, compared by whole path component
exactly as before, so a prefix-only configuration is unchanged. A glob is anchored to the whole
repository-relative path with forward slashes:

- `*` matches within one path component, `**` across components (`**/` also matches no directory),
  `?` one character other than `/`, `[abc]` and `[!abc]` a character class.
- A glob matches the path itself and anything below it, so `rust/crates/*/tests` covers every file
  in each crate's `tests` directory, as a prefix would.
- `exclude_paths` removes matching paths from the rule's source set. A changed path under a source
  entry that an exclusion also matches is not a trigger. It stays in the rule's population as
  eligible and unexamined, so a test-only diff shows what was set aside, without
  `config.scope-matches-nothing` reading a deliberate exclusion as an unreachable configuration.

```yaml
rules:
  diff.source-requires-record:
    level: error
    source_paths: ["rust/crates"]
    exclude_paths: ["rust/crates/*/tests", "**/*.test.*"]
```

A glob that cannot compile (an unclosed `[`) is refused at load. `published_paths` and
`back_pointer_paths` stay prefixes: a `*` in them is a literal character.

## Relation pairs

A top-level `relation_pairs` list declares pairs of inverse relation fields (`RFC-47`, `MILE-122`).
Each entry:

```yaml
relation_pairs:
  - name: resolution          # unique, non-blank
    forward: "Resolves"       # this record -> the record it resolves
    inverse: "Resolved-by"    # the resolved record -> this one
    reciprocity: required     # required | none
    cross_type: true          # optional, default true
    types: [rfc]              # optional (MILE-155), default: every type declaring a side
    provisional:              # optional (MILE-154)
      statuses: [Draft]       # claimant statuses at which an unechoed claim is provisional
      level: warn             # warn (default) | error
```

A claim `A.forward = B` is echoed by `B.inverse = A`, and `A.inverse = B` by `B.forward = A`.
Direction is the field name, so two records each claiming the `forward` side do not satisfy each
other. `relation.pair-reciprocity` checks `reciprocity: required` pairs; `relation.cross-type-disallowed`
reports a claim whose target is another record type in a `cross_type: false` pair, which the
reciprocity rule excludes (counted out of scope) rather than skipping silently.

`provisional` (`MILE-154`, from the `MILE-127` decision) splits `relation.pair-reciprocity` by the
**claimant's** status, read from its `Status` role field by exact comparison. A claim from a record
whose status is listed in `provisional.statuses` and that the peer does not echo is reported at
`provisional.level` (`warn` by default), with the message saying so; a claim from any other status
keeps the rule's own level, so it is an error once the claimant is accepted. A claim already echoed
by the peer's inverse field is settled and is not reported at either level. A claim echoed only in a
differently named, unpaired field (a pending field) is not an echo, so it is held to the provisional
level; the engine does not read that field. A status listed that no record type declares in
`statuses.values`, a blank or duplicated status, an empty list, or a `level` other than `warn` and
`error` is refused at load, so the split cannot silently apply to nothing.

The narrowest reading of what `RFC-47` leaves open, each a decision recorded here until an ADR
revisits it:

- **Per-type binding** (`MILE-155`). Optional. A pair's two names are global by default: every type
  that declares a name in `required_fields` or `known_fields` carries that side, and a claim to a
  type that does not declare the echoing field is an unechoed claim. A pair that declares
  `types: [rfc]` is carried by those record types only: a record of another type is not that pair's
  business (its fields are not slots), and a claim whose target is another type is excluded as out
  of scope, counted in the population, not reported as unechoed, because that type cannot hold the
  inverse field. Each listed type must be a configured record type declaring at least one of the
  pair's fields; an empty or duplicated list is refused at load. `relation_fields` is unchanged and
  still binds only the fixed roles.
- **Config shape.** Top-level `relation_pairs`, not per type.
- **Fixed `RelationRole` set.** Unchanged. `Supersession` keeps its combined
  `Supersedes / Superseded-by` key and `relation.supersession-reciprocity` stays beside the general
  rule: folding it in would change findings (it reads one combined key with no direction), so it is
  not replaced. This repository declares no pair.
- **Combined keys.** Not supported by `relation_pairs`; a `combined` key is rejected as an unknown
  key. A pair names two distinct fields.
- **`Amends` one-way.** Not decided. A corpus that wants `Amends` / `Amended-by` echoed declares that
  pair; one that treats `Amends` as one-way declares none. This repository declares none, so `BUG-17`
  stays `Open`.
- **`advisory`.** Refused at load. The rule's declared level is the repository's severity
  (`MILE-80`), so a per-pair severity has nothing to set.
- **Direction-mismatch.** `relation.direction-mismatch` is not built; an unechoed claim already
  reports a mismatched direction, because the echo is read from the opposite field.

Load-time errors: a blank or duplicated pair name, a blank or whitespace-padded field name, a field
declared by no type (the pair could never examine a record), a field used by two pairs or twice by one
pair, an unknown `reciprocity`, and enabling either pair rule while no pair is declared. A pair field
declared as a `pointer_fields` entry is also resolved by `pointer.resolution`, so an unresolved
reference reports twice; keep pair fields outside `pointer_fields`, as `Supersedes / Superseded-by` is.

## `urzua doctor`

**Specified in full by SPEC-15.** Reports on the tool's own configuration and invocation health —
a real, standalone feature area with its own output shape and bug history, not a detail of how
config is structured.

## Success criteria

1. Both target codebases run green under one binary with no code change and no escape hatch.
2. Their two configs are diffable against each other, and the diff is a readable statement of how
   the corpora genuinely differ.
3. Every rule in SPEC-2 is reachable from config, and `doctor` (SPEC-15) lists the ones that are
   off. The first half holds: `MILE-80` shipped the `rules` table, every rule is opt-in, and each
   carries a declared level. The second does not — `doctor` emits no rule information at all, so a
   corpus cannot learn from it which checks are not running (`BUG-40`).
4. If (1) fails, the failure is recorded as a schema finding with the specific rule that could not
   be expressed — not patched around with a code path for one repo.

Criterion 4 is the point of the spec. The claim is only worth something if we would notice it being
false.

## Open questions

- **Can scoped overrides express per-package variation without becoming a second config format?**
  The moment overrides gain their own resolution order, this is per-package configs with extra
  steps.
- **Does the config need a version field?** It has no consumers yet, so a version costs nothing now
  and is unaddable later without a migration.
- **Where do back-pointer patterns live** — config, or fixed by the record format (ADR-2)? Making
  them configurable is what lets a corpus keep its existing `Implements:` convention; fixing them is
  what makes cross-repo resolution possible (roadmap Phase 4).

## References

- SPEC-2 — the rules this configures.
- RFC-11 §1 and §6 — boundaries and scope.
- ADR-4 — the layout this config sits at the root of.

## Rules

A repository declares which rules run and at what level. A rule that is not named does not run
(`ADR-53`), and appears in `rules_executed` as `not-enabled` rather than being omitted (`ADR-7`) --
"off" and "ran clean" must stay distinguishable.

```yaml
rules:
  header.required-fields: error
  embodiment.consistency: warn
  pointer.target-status:
    level: warn
    not_in: ["Superseded"]
```

A level is `off`, `warn` or `error`, written bare or as a table when the rule takes options. The
declared level **replaces** whatever severity the rule body chose: one rule, one level.

Two things are load-time errors rather than silent no-ops, because a check that never runs is
indistinguishable in the report from one that ran clean:

- a rule name this build does not ship -- the error names the valid set
- an option on a rule that does not take it

### Severity per field (`MILE-140`)

`field.quality`, `field.pending` and `status.required-field-missing` report one finding per field.
Each takes an optional `field_levels` map from a field name to `warn` or `error`. Without it, the
rule's `level` is every finding's severity, as before.

```yaml
rules:
  status.required-field-missing:
    level: warn                                      # the level for every field not listed
    field_levels: {Deciders: error, Reviewers: warn} # a missing Deciders blocks; Reviewers only warns
```

- A finding for a listed field has that field's level; a finding for any other field has the rule's
  `level`. The rule's `level` still decides whether the rule runs: `off` runs nothing, whatever
  `field_levels` says.
- A field name matches exactly. It must be a `required_fields` or `known_fields` entry of some
  record type.
- A level is `warn` or `error`. `off` is refused (a field is switched off by not declaring it), as
  are an empty map, a field no type declares, and the option on any rule not named above.

### Evidence pins, a grace window and a claim age (`MILE-129`)

`embodiment.consistency` takes two optional day-count options and reads one optional locator
suffix. Each is off until written, and a repository that writes none sees no change.

```yaml
rules:
  embodiment.consistency:
    level: warn
    drift_grace_days: 7      # a locator changed fewer than 7 days ago is not yet drift
    claim_max_age_days: 180  # warn when the Realized-by line was last touched over 180 days ago
```

- `drift_grace_days` (whole days, `0` or more): drift is not reported while the cited file's last
  commit is newer than the window, so a code change can land before its record catches up. Once the
  window passes the drift reports.
- `claim_max_age_days` (whole days): a warning when the commit that last touched the record's
  `Realized-by` line is older than the limit, whether or not the evidence changed. It reports in
  addition to any disagreement, and a record whose line git cannot date is not warned.
- A locator written `code:path/to/file.rs@sha256:<64 lowercase hex>` is pinned to that content hash
  (`sha256sum path/to/file.rs`). A pinned locator drifts exactly when the file's current hash
  differs, and git history is not consulted for it, so an unrelated commit never reports and a
  change made after the record was written does. Any other suffix is not a pin and stays part of the
  path. Re-pinning is a hand edit of the hash on the `Realized-by` line.

Both options belong to `embodiment.consistency` alone; on any other rule they are the usual
option-on-a-rule-that-does-not-take-it load error.

### Source back-pointers as evidence (`MILE-128`, `MILE-146`, `MILE-149`)

`embodiment.back-pointer-resolution` reads source comments as realization evidence and takes one
optional option beside its required `back_pointer_paths`.

```yaml
rules:
  embodiment.back-pointer-resolution:
    level: error
    back_pointer_paths: ["src", "tests"]
    test_paths: ["tests"]      # the test class, each entry inside a back_pointer_paths entry
```

- `Implements: ADR-12` in any file under `back_pointer_paths` is code-tier evidence for the record
  it cites. `Verifies: ADR-12` in a file under `test_paths` is test-tier evidence, which computes
  `Verified` exactly as a `test:` locator does, so a record needs no restated `test:` list. A
  `Verifies:` comment outside `test_paths` is not read, and with no `test_paths` declared none is.
  One record can be cited at both tiers.
- Both markers are read the same way: at the start of a comment line or in a trailing comment after
  code, an id list in which a note may sit between ids (`ADR-1 (why), ADR-2`).
- A `test_paths` entry that is not inside any `back_pointer_paths` entry is a load error (nothing
  there would be scanned), and one covering no tracked file is a finding.
- `embodiment.consistency` judges a record with `Embodiment` but no `Realized-by` field when a
  back-pointer cites it, computing from the back-pointers alone; the finding says so. A record
  neither cited nor carrying `Realized-by` is still not judged. `urzua fix` repairs the same
  records.

The option belongs to `embodiment.back-pointer-resolution` alone; on any other rule it is the usual
option-on-a-rule-that-does-not-take-it load error.

### Sticky Embodiment states (`MILE-148`)

Some `Embodiment` values are a human's decision that no evidence can reproduce, such as
`Deprecated` or `Inactive`. The repository declares them; the engine names none.

```yaml
rules:
  embodiment.consistency:
    level: warn
    sticky_states: [Deprecated, Inactive]
```

- A record stating a listed value is examined (it counts in the rule's census) and produces no
  finding: neither a disagreement with the computed tier nor a `claim_max_age_days` warning, since
  a sticky state is not a verification claim. `urzua fix` does not rewrite it.
- A listed value that evidence computes (`Verified`, `Implemented`, `Specified`, `Not started`,
  `Drift detected`) is a load error: it would make that tier's drift unreportable. A blank entry
  is refused like any other blank option entry.
- Omitted, every stated value is held to the computed one, as before. The option belongs to
  `embodiment.consistency` alone.

### Date fields and the acceptance date (`MILE-137`)

One opt-in rule validates dates, `date.field-valid`. Which fields are dates is a rule option; the
engine names no field itself.

```yaml
rules:
  date.field-valid:
    level: error
    date_fields: [Date, Accepted-on]     # each must hold a real YYYY-MM-DD date
```

- `date_fields` (required, non-empty): a value that is not `YYYY-MM-DD`, or is not a real calendar
  date (`2026-02-30`, `2026-02-29` outside a leap year), is an error. A blank, placeholder or
  pending value is not a date value and is not examined. A missing option is a load-time error, as
  is the option on any other rule.

`date.field-order` (`MILE-157`) compares two date fields of one record:

```yaml
rules:
  date.field-order:
    level: error
    date_order: ["Date <= Accepted-on"]   # the first may not be later than the second
```

- `date_order` (required, non-empty): each entry is `Earlier <= Later`, two distinct field names.
  Both must be declared (`required_fields` or `known_fields`) by one record type, which is the only
  kind of record the entry applies to; anything else is a load-time error, as is the option on any
  other rule.
- Equal dates pass. A value that is absent, blank, placeholder, pending or not a real `YYYY-MM-DD` date
  is not compared: `date.field-valid`, `field.quality` and `status.required-field-missing` own those
  states, so one defect is not reported twice. Such a slot is eligible and unexamined in the
  population.
- It adds no second notion of "a date": it reads the same field states and the same date validity as
  `date.field-valid`.

There is deliberately no dedicated acceptance-date rule. The acceptance date is required on an
accepted-type status and forbidden on every other through the one mechanism for status-dependent
fields, `statuses.require` / `statuses.forbid` (`MILE-132`), with `date.field-valid` owning the
date's well-formedness:

```yaml
record_types:
  adr:
    known_fields: [Accepted-on]
    statuses:
      values: [Proposed, Accepted, Superseded, Rejected]
      success: [Accepted, Superseded]
      require: [{when: [Accepted, Superseded], fields: [Accepted-on]}]
      forbid:  [{when: [Proposed, Rejected], fields: [Accepted-on]}]
rules:
  status.required-field-missing: error   # Accepted without Accepted-on
  status.forbidden-field-present: error  # Proposed carrying Accepted-on
  date.field-valid: {level: error, date_fields: [Accepted-on]}
```

A blank or placeholder value counts as missing on the required side, and is tolerated on the
forbidden side. The integration test
`the_acceptance_date_by_status_is_expressed_with_statuses_require_forbid_and_date_field_valid`
proves this configuration produces the three findings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-08-20 | Config moves from `urzua.yaml` at the repo root to `.urzua/config.yaml`. The tool owns templates and derived state as well as config, and templates must sit outside the corpus they describe — a template is invalid as a record by construction, so a checker discovers it and reports errors on a correct file. The alternative, an ignore list, is the ad-hoc exclusion RFC-11 and RFC-13 both reject elsewhere. | **substantive** |
> | 2026-08-20 | Split out of SPEC-1's Configuration section and expanded. | **structural** |
> | 2026-09-07 | `urzua doctor` split out into its own spec (SPEC-15). **Why:** doctor is a real, standalone feature area with its own output shape and its own bug history (BUG-4, found the same day), not merely a detail of how config is structured — folding it into SPEC-3 by default rather than by a deliberate call was exactly the kind of inconsistency MILE-77's review named. Success criterion 3 corrected to note "which rules are off" isn't actually true yet (MILE-80). | **structural** |
> | 2026-09-08 | Bumped to `0.3`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-09-17 | Config moves from `.urzua/config.toml` to `.urzua/config.yaml` (`ADR-52`, shipped in the same change). The described mechanism changes, not just its rendering. | **substantive** |
> | 2026-09-17 | Documented the `rules` table: per-rule `level`, opt-in, and the two load-time errors. **Why:** `SPEC-1` listed "which checks are errors vs. warnings" as a v0 configuration surface and it was never built -- severity lived as 34 hardcoded literals in `rules.rs` with no config key at all (`MILE-80`). | **substantive** |
> | 2026-09-18 | `Status: Draft` → `Accepted`, with `Embodiment`/`Realized-by` declared. **Why:** Every command reads `.urzua/config.yaml` on every invocation, and this spec was revised again today to document the `rules` table `MILE-80` shipped. Verified against the binary before flipping rather than flipped in bulk -- `BUG-26` asked for exactly that, and it is why `SPEC-4` and `SPEC-5` are not flipped with these. | **substantive** |
> | 2026-09-18 | Criterion 3 corrected. **Why:** it still read *"not yet true today, since no per-rule enable/disable or severity config exists yet"* on the day after `MILE-80` shipped exactly that — and this record was marked `Accepted`/`Verified` in the same change, which is `BUG-26`'s own defect committed while fixing `BUG-26`. Caught by review. The `doctor` half is genuinely still unbuilt and now says so rather than being swept up with the half that shipped. | **substantive** |
> | 2026-11-03 | Bumped to `0.5`. Documented the per-type document model declaration (`identity`, `fields`, `sections`) as parsed and validated but read by no rule yet. **Why:** first slice of `MILE-98` (`RFC-33` layer 1): a record type can now declare how its parts are found, and a malformed declaration fails the load instead of being ignored. | **substantive** |
> | 2026-11-04 | Bumped to `0.7`. Documented the top-level `relation_pairs` declaration and the narrowest reading of each question `RFC-47` leaves open (global field names, no combined key, no `advisory`, fixed role set unchanged). **Why:** `MILE-122` -- reciprocity was hardcoded to one combined key, so a pair a repository declares itself, including across record types, could not be checked. | **substantive** |
> | 2026-11-04 | Bumped to `0.6`. The per-type `identity` and `fields` declarations now drive record loading: the identity pattern is compiled at load and decides a record's identifier and H1 title form (it must capture the type prefix), and `fields.from` decides how the header is read, with `prefix-lines`, `blockquote`, `bold-list` and `none` added beside `yaml-frontmatter`. **Why:** second slice of `MILE-98` (`RFC-33` layer 1, `RFC-44`): a declaration that no code reads is not the founding claim. | **substantive** |
> | 2026-11-04 | Bumped to `0.8`. Documented `sections.required` (with the `y-statement` and `table-min-rows` shapes) and `sections.exempt_statuses`, now read by `section.required` and `section.shape`; `sections` is no longer read by no rule. **Why:** `MILE-4`, `MILE-5`, `MILE-7` and `MILE-121` -- the declared `sections` model from `MILE-98` had no reader. | **substantive** |
> | 2026-11-04 | `Embodiment` recomputed by `urzua fix`: `Drift detected`. **Why:** every file named in `Realized-by` has changed since this line was last verified, and this pass did not re-verify it, so the record states the drift instead of a stale `Verified`; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-core/src/config.rs, test:rust/crates/urzua-core/src/config.rs` -- stated `Verified` disagreed with the computed value. | **structural** |
> | 2026-11-04 | Bumped to `0.8`. Documented the per-type `statuses` declaration: the status vocabulary, terminal success and failure statuses, fields required or forbidden at a status, and the type's own pending vocabulary. **Why:** `MILE-132` Phase 0: which statuses a type has is repository configuration, not engine policy (`BUG-59`), and the pending vocabulary (`BUG-65`) and finished-status lists (`BUG-178`) were compiled in or hand-copied. | **substantive** |
> | 2026-11-04 | Bumped to `0.9`. Documented the `drift_grace_days` and `claim_max_age_days` options of `embodiment.consistency` and the `@sha256:` locator pin. **Why:** `MILE-129` -- all three opt-in and off by default, so drift can be pinned to content, held back for a fresh change, and an old claim flagged. | **substantive** |
> | 2026-11-04 | Bumped to `0.11`. Documented the `date_fields`, `acceptance_field` and `accepted_statuses` options of the new opt-in `date.field-valid` and `date.acceptance-by-status` rules. **Why:** `MILE-137` Phase 0: which fields are dates and which is the acceptance date are repository configuration, read from rule options (the `fields` declaration names no field types) and from the declared `statuses.success`. | **substantive** |
> | 2026-11-04 | Bumped to `0.10`. Documented the `field_levels` option of `field.quality`, `field.pending` and `status.required-field-missing`. **Why:** `MILE-140` -- severity set per field within one rule, so a missing `Deciders` on an accepted record can be an error while a missing `Reviewers` is a warning. Off unless written. | **substantive** |
> | 2026-11-04 | Bumped to `0.12`. Removed the `acceptance_field` option and the `date.acceptance-by-status` rule from the date section; the acceptance date is now documented as a `statuses.require` / `statuses.forbid` declaration plus `date.field-valid`. `accepted_statuses` stays, owned by `embodiment.back-pointer-decision-accepted`. **Why:** maintainer decision of 2026-11-04 (`MILE-137`): one mechanism for status-dependent fields, not two that could disagree. | **substantive** |
> | 2026-11-04 | Bumped to `0.13`. A type's `statuses` declaration gains an optional `implementable` list: the statuses at which implementation of the record is permitted, validated as a subset of `values`. `embodiment.back-pointer-decision-accepted` judges a back-pointer against it, falling back to the rule's `accepted_statuses`. **Why:** the engine holds no type knowledge, so terminal success cannot stand for "implementation allowed" (maintainer decision of 2026-11-04, `MILE-133`). | **substantive** |
> | 2026-11-04 | Bumped to `0.14`. A type's `statuses` declaration gains an optional `claim_retained` list: statuses reached only after acceptance (such as `Superseded`) that may still carry an `Implemented` or `Verified` claim, validated as a subset of `values`. **Why:** maintainer decision of 2026-11-04, `MILE-138`: a superseded record's embodiment claim stays true history. | **substantive** |
> | 2026-11-04 | Bumped to `0.15`. Documented the top-level `fail_on` key (`error` or `warning`, default `error`) and `urzua check --fail-on`. **Why:** `MILE-145` -- warnings could never fail a build, so a repository at zero warnings had nothing to keep it there. | **substantive** |
> | 2026-11-04 | Bumped to `0.16`. Documented the optional `external_ids` option of `citation.resolution`: exact ids or inclusive numeric ranges under a declared record prefix that are skipped (counted out of scope) instead of resolved. **Why:** `MILE-156` -- a public standard number such as `RFC-4122` shares a record prefix and was reported as a dangling citation, with no way to say it is not ours. | **substantive** |
> | 2026-11-04 | Bumped to `0.17`. `statuses.require` counts a value in the type's declared `pending` vocabulary as missing, the same list `field.pending` reads. **Why:** `MILE-153` -- an Accepted record whose required field read `Unknown` passed; the vocabulary is declared once, in configuration. | **substantive** |
> | 2026-11-04 | Bumped to `0.18`. Documented the per-type `exclude_files` declaration: bare file names directly in a type's `dir` that are not its records, excluded from every rule's population and from `filename.identity-pattern`'s census. **Why:** `MILE-150` and `BUG-183` -- a `README.md` in a record directory was read as a record and failed every rule, with no way to say it is not one. | **substantive** |
> | 2026-11-04 | Bumped to `0.NEXT`. A relation pair may declare `provisional: {statuses, level}`: an unechoed claim from a claimant at one of those statuses is reported at the provisional level (default `warn`), any other claimant at the rule's level. **Why:** `MILE-154` / `MILE-127` -- one severity for a claim from a Proposed record and a broken accepted claim was either too loud or too quiet. Off unless written. | **substantive** |
> | 2026-11-04 | Bumped to `0.20`. Documented the `test_paths` option of `embodiment.back-pointer-resolution` and what source back-pointers compute: `Implements:` is code tier, `Verifies:` in a declared test file is test tier, and a record with no `Realized-by` field that a back-pointer cites is judged on that evidence. **Why:** `MILE-146` and `MILE-149` -- a record whose tests cite it should compute `Verified` without a hand-maintained `test:` list, and a field added only to make a checker look is restated evidence. | **substantive** |
> | 2026-11-04 | Bumped to `0.21`. A relation pair may declare `provisional: {statuses, level}`: an unechoed claim from a claimant at one of those statuses is reported at the provisional level (default `warn`), any other claimant at the rule's level. **Why:** `MILE-154` / `MILE-127` -- one severity for a claim from a Proposed record and a broken accepted claim was either too loud or too quiet. Off unless written. | **substantive** |
> | 2026-11-04 | Bumped to `0.21`. A relation pair may declare `types`, the record types that carry it; a claim to a type outside the list is out of scope instead of an unechoed claim. **Why:** `MILE-155` -- a pair is global, so a claim to a type that cannot carry the inverse field could only error. Off unless written. | **substantive** |
> | 2026-11-04 | Bumped to `0.22`. Documented the `sticky_states` option of `embodiment.consistency`: `Embodiment` values a human sets that no evidence computes are declared in configuration, are not reported as drift, and may not name a computed tier. **Why:** `MILE-148` -- the set of sticky states is repository vocabulary, not engine policy. | **substantive** |
> | 2026-11-04 | Bumped to `0.23`. Noted that `urzua new` numbers, names and titles a record by the type's declared `identity`. **Why:** `MILE-151` -- the generator must read the grammar the checker reads. | **substantive** |
> | 2026-11-04 | Bumped to `0.24`. The diff rules' `source_paths`, `application_paths` and `exclude_paths` entries may be globs, and `exclude_paths` now also narrows `diff.source-requires-record`. **Why:** `MILE-158` -- an exclusion and a pattern are needed to state "all source except tests". Prefix-only configurations behave as before. | **substantive** |
> | 2026-11-04 | Bumped to `0.25`. Documented the `reference_sections` option of the new opt-in `citation.path-exists` rule: the headings whose backtick-quoted repository paths must exist in the working tree. **Why:** `MILE-143` -- a cited path that no longer exists was never checked; where references live is repository configuration. | **substantive** |
> | 2026-11-04 | Bumped to `0.26`. Documented the opt-in `date.field-order` rule and its `date_order` option (`Earlier <= Later`). **Why:** `MILE-157` -- a record accepted before it was written passed, since no rule compared two date fields; `date.field-valid` still owns well-formedness. | **substantive** |
> | 2026-11-04 | Bumped to `0.27`. `y-statement` strips a leading `>` blockquote marker from each line before phrase matching, so a phrase wrapped across quoted lines is found. **Why:** `BUG-181`. | **substantive** |
> | 2026-11-04 | Bumped to `0.28`. Documented the `indexes` entry shape of `index.freshness`: multi-line `format`, `{file}` in place of `{id}`, `{title}` without the id, and regeneration with `urzua index`. **Why:** `MILE-152` -- a YAML or link-list index could not be declared, and nothing regenerated one. | **substantive** |
