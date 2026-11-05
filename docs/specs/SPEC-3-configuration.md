---
Version: '0.62'
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
    header_shape: "yaml-frontmatter"
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
  the wrong lines. `header_shape` defaults to `"yaml-frontmatter"` when omitted. `"yaml-frontmatter"`
  (RFC-16) is genuinely structured data, deserialized rather than line-recognized, and is the one
  shape with a reader, alongside `"none"` for a type with no header. The `"blockquote"` and
  `"bold-list"` shapes were removed (`MILE-85`): a config declaring either fails to load with an
  error naming the shape, and a record still written in one is reported as unreadable with the form
  named, so neither is mistaken for a record with no header. `header_layout` and the
  `header.layout-consistency` and `header.deprecated-shape` rules went with them.
- **A record type may declare how its revision log is found: `revision_log`** (`MILE-167`). Exactly
  one of `marker` (a whole line, blockquote marks aside, such as `**Revision log**`) or `heading`
  (the text of a markdown heading of any level, such as `Revision log`):

  ```yaml
  record_types:
    adr:
      dir: docs/adr
      revision_log: {heading: "Revision log"}   # or {marker: "**Revision log**"}
  ```

  The section opens at the first line that *is* the marker or heading, outside a fenced code block;
  a line that merely mentions the text (`The **Revision log** below ...`) does not open it, so prose
  cannot make a record read the wrong table. Omitted, the type uses the marker `**Revision log**`,
  so an existing repository is unchanged. A declaration naming both or neither, a blank identity,
  or one spanning lines is a load-time error naming the type. The declaration also takes
  `required: true|false` (`MILE-159`; `true` when omitted, so behaviour is unchanged): whether
  `urzua fix --apply` needs the section. Required, a record with no section fails that write with a
  message naming the identity looked for; `required: false` writes the field without a log row.
  `required` may stand alone (`revision_log: {required: false}`), leaving the identity at its
  default; an empty `{}` is still an error. `revision-log.change-class-required`,
  `urzua fix`'s revision row and the `skip_revision_log` option of `citation.path-exists` and `citation.resolution` all read it.
  The declaration also takes `version_bump: {field, classes}` (`BUG-51`), absent for every type
  unless written: a field name and the change classes (the revision log's Class column) that require
  that field to change. `revision-log.version-bump-required` reports a changed record that adds a row
  of one of those classes while its field equals the merge base's. `field` must be non-blank and
  `classes` a non-empty list of non-blank names, else the config fails to load; `version_bump` may
  stand alone like `required`. Enabling the rule when no type declares one is a load error.
- **A record type may declare that it is the suppression type: `role: suppression`** (`BUG-74`).
  The records of that type suppress the findings they name (`SPEC-10`); the engine reads this
  declaration and knows no type name:

  ```yaml
  record_types:
    exception:            # any name
      dir: docs/exceptions
      role: suppression
  ```

  At most one type declares it (two are a load error naming both), and `suppression` is the only
  value; any other is refused at load. **Fallback:** when no type declares a role, the type named
  `waiver` is the suppression type, so a config written before the declaration existed keeps its
  suppression unchanged. Once a type declares the role, a type merely named `waiver` suppresses
  nothing. A config with neither a declaring type nor a type named `waiver` has no suppression.
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
  `?`, `[`) is a load-time error naming the record type, because it would exclude nothing. An entry that
  names no file directly in the type's `dir` is reported by `type.dir-matches-nothing` as a warning
  naming the type and the entry, so a stale exclusion is visible. Omitted or empty, the type loads
  as it always did.
- **A record type may own several directories: `extra_dirs`** (`MILE-130`). A list of directories
  beside `dir`, each a directory in the same sense as `dir` (its `.md` files, not its subtree,
  `RFC-35`):

  ```yaml
  record_types:
    adr:
      dir: docs/adr
      extra_dirs: [docs/adr-archive, packages/web/adr]
      prefix: ADR
  ```

  `dir` and every `extra_dirs` entry are one identity namespace. Records in any of them are the
  type's records; ids, pointers, `identity.collision` and `filename.identity-pattern` see one
  population, so `ADR-7` in `docs/adr-archive` resolves from a pointer in `docs/adr` and collides with an
  `ADR-7` in `docs/adr`. `urzua new` writes to `dir` and numbers past the highest id in any of the
  directories, and past a number claimed in any of them on another local or remote-tracking ref. An
  entry is validated like `dir` (it names a directory below the repository root); a directory listed
  twice, and a directory owned by two types, are load errors; an `extra_dirs` entry that does not
  exist is reported by `type.dir-matches-nothing`, and `urzua new` numbers without it and says so on
  stderr. `exclude_files` names files in any of the directories. Two types sharing a prefix remain
  refused: the namespace is one type's.
- **A record type may declare its document model: `identity`, `fields` and `sections`**
  (`RFC-33` layer 1, `MILE-98`). Three optional keys under a record type, each a mapping with a
  required `from`:

  ```yaml
  record_types:
    adr:
      dir: doc/adr
      identity: {from: filename, pattern: '^(?P<number>\d+)-(?P<slug>.+)$'}
      fields:   {from: prefix-lines}      # or yaml-frontmatter, none
      sections: {from: headings, depth: 2, items: true}
  ```

  `identity.pattern` is a regular expression over the file name without `.md`; it must compile and
  must capture the record number as `(?P<number>\d+)`. `sections.depth` counts heading levels from `##`
  (1 is `##` only, 2 adds `###`, up to 5) and defaults to 1; `items` defaults to `false`. An unknown
  `from`, an unknown key, an out-of-range `depth`, a non-boolean `items`, a blank or non-compiling
  `pattern` or a key present with no value is a load-time error naming the record type and key.

  **`identity` and `fields` drive record loading; `sections` is read by `section.required` and
  `section.shape` only, down to the declared `depth`.** A type that declares none of the three loads, and is checked, exactly as it
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
  - **`fields.from`.** Decides how header fields are read, overriding `header_shape`: `yaml-frontmatter`
    and `none` are the existing shapes (`blockquote` and `bold-list` were removed and fail the load,
    `MILE-85`), and `prefix-lines` reads bare
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
    counted. A heading inside a fenced code block is not a section. A requirement is at the `##`
    level unless it declares otherwise (see `level` and `under`).
    - `y-statement` reads one block and requires `In the context of`, `facing`, `we decided`,
      `to achieve` and `accepting` in it, matched case-insensitively on word boundaries (the phrase
      set is fixed, not declarable). With a `marker`, the block starts at the first line of the
      section containing it and runs to the next blank line or list item; a section without the
      marker, and a marker with no block after it, are each their own finding. With no `marker`,
      the block is the section's first paragraph. A leading `>` blockquote marker on a line is not part of
      the block, so a phrase that wraps across quoted lines still matches. Phrases outside the block never count.
    - `aliases` (optional, any shape; `MILE-165`) lists other `##` heading texts that satisfy the
      requirement, matched exactly after trimming: `{name: Decision, aliases: [Resolution]}`. The
      `name` is tried first, then each alias in order. Both rules follow the alias, and `urzua check`
      reports one `info` notice per (section, alias) naming the records that used it, so an alias
      in use is visible and never moves the exit code. A blank alias, one repeating the `name` or
      another alias, or a heading that two requirements claim, is a load-time error.
    - `preamble: true` (optional; `MILE-165`) lets a `y-statement` whose `marker` is declared be read
      from the preamble, the text before the first `##` heading, when the section itself is absent:
      `section.required` is then satisfied and `section.shape` judges the preamble with the same
      check, reporting its findings as the preamble's. It applies only when the section and its aliases
      are absent and the marker is in the preamble; a preamble without the marker leaves the
      section missing. It needs shape `y-statement` with a `marker` (without one, the block would be
      the H1's first paragraph) and is a load-time error otherwise.
    - `level` and `under` (optional, any shape; `MILE-98`) make a deeper heading visible to the section
      rules. `level` is the heading level the requirement sits at, from 2 (`##`, the default) to 6;
      `under` names the heading one level up it must sit beneath: `{name: Pros and cons, level: 3,
      under: Options}` asks for a `### Pros and cons` inside the `## Options` section, and a
      `### Pros and cons` under another `##` does not satisfy it. A `###` is a section of its own
      range, ending at the next heading of the same or a shallower level, and is also part of the
      `##` body around it. A requirement without `under` is satisfied under any parent at its level,
      and an `##` heading never satisfies a `###` requirement or the reverse. A `level` the type's
      `sections.depth` does not reach (`depth` must be at least `level - 1`), a `level` outside 2 to 6,
      a blank `under`, or an `under` on a `##` requirement is a load-time error rather than a
      requirement that can never match. The same name at a different level or under a different
      parent is not a duplicate. A type that declares no `level` is read exactly as before whatever its
      `depth`.
    - `table-min-rows` requires a table with at least `min_rows` (an integer of 1 or more) data rows;
      a data row with every cell empty does not count, the largest table in the section decides, and
      a table inside a fenced code block is not read.
    - `exempt_statuses` lists statuses of a record that made no decision (`MILE-7`). Such a record is
      counted eligible and unexamined by both rules, not asked for the sections: set aside on purpose,
      so `config.scope-matches-nothing` does not report it (`BUG-187`). It is matched exactly against
      the type's declared status field. A record whose header cannot be read is counted unreadable,
      never assumed exempt, and a record with no status is not exempt. Declaring it without
      `required`, a blank status, a duplicate section name, an unknown `shape`, `min_rows` missing
      from or `marker` misplaced on a shape, or an unknown key, is a load-time error.
    - Presence and shape are all either rule knows: neither judges whether a section's content is
      true or sufficient.

  `section::parse_sections_to_depth` (`section.rs`) splits a record body into its sections down to
  `sections.depth`, each with its level, its parent and its line range (`parse_sections` is the
  depth-1 reading, `##` only).

- **A record type may declare companion field pairs: `companion_fields`** (`BUG-41`). An optional
  list under a record type, each entry exactly two distinct relation role keys (`status`,
  `embodiment_state`, `embodiment_locator`, `supersession`); anything else is refused at load. The
  fields are the ones the type's `relation_fields` names for those roles (default literals
  otherwise). `config.companion-declaration-missing` reports a type that declares one field of a
  declared pair in `required_fields`/`known_fields` without the other, or neither. No pair is known
  to the engine; this repository declares `[embodiment_state, embodiment_locator]` for `adr` and
  `spec`.

  ```yaml
  record_types:
    spec:
      known_fields: [Embodiment, Realized-by]
      companion_fields:
        - [embodiment_state, embodiment_locator]
  ```

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
        initial: [Open]                   # where a new record may start (with transitions)
        transitions:                      # which statuses may follow each status
          Open: [Fixed, WontFix]
          Fixed: [Open]
        pending: [pending, "not yet written", {pattern: "^\\(pending"}]
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
  with no value is a load-time error naming the record type and `statuses`.

  `transitions` and `initial` (`MILE-144`, `ADR-72`) declare the type's lifecycle. `transitions`
  maps a declared status to the declared statuses that may follow it; a status with no entry is
  terminal, nothing may follow it. `initial` is the list of statuses a new record may start at, and
  is also where the engine starts when it asks which declared statuses can be reached. The two come
  together: either without the other is a load error, as is a status that is not in `values`, a list
  that repeats an entry, or a status listed as following itself. A type that declares neither has no
  lifecycle and no transition is judged. Two opt-in rules read it: `status.transition-allowed`
  compares a changed record's `Status` at the merge base with its `Status` in the change under check
  and reports an edge the graph does not list, or a new record outside `initial`; it needs `--base`
  and reports not-applicable without it. `status.unreachable-state` reports a declared status that no
  path from `initial` reaches; it judges the declaration and needs no `--base`. A record whose
  committed status is no longer declared is counted unreadable by the first rule, not judged. This repository's policy is that an ADR or RFC enters at `Proposed` or `Draft` and is accepted in a later change, so its `initial` is that one status; a bug or milestone may be filed already finished, because the PR that fixes or delivers it files it. The
  derived `blocked` value is not part of this declaration (`MILE-27`).

  A `pending` entry is a whole value (a string) or a pattern for a placeholder family that shares a
  stem and differs in a tail (`MILE-162`): the structured form `{pattern: "^\\(pending"}`, an ordinary
  regular expression searched in the trimmed value, so the author anchors it (`^`, `$`) and writes
  `(?i)` for case-insensitivity. A whole value is still compared case-insensitively. A value that
  matches no entry and no pattern is a real value. A pattern that is not a valid expression, is blank
  or repeated, or an entry with any other key (`{pattern, flags}`) is a load-time error naming the
  type and `statuses`.

  `statuses.delivery` (optional, `MILE-183`) holds a type's `Status` to its delivery evidence:

  ```yaml
  record_types:
    milestone:
      known_fields: [Realized-by]
      statuses:
        values: [Planned, InProgress, Blocked, Done, WontDo]
        success: [Done]
        failure: [WontDo]
        delivery:
          delivered_tiers: [Implemented, Verified]   # computed tiers that say the work exists
          unevidenced_tiers: ["Not started"]         # computed tiers that say nothing was delivered
          not_done: [Planned, InProgress, Blocked]   # statuses that say it is not done
          set_status: Done                           # what `urzua fix --apply` writes
  ```

  The tier is computed as `Embodiment` is, from the record's evidence field (the type's
  `Realized-by`, or its declared name for that role) and from source back-pointers, with drift
  ignored: drift says the evidence is stale, not that nothing was delivered. All four keys are
  required. A tier must be one of `Verified`, `Implemented`, `Specified`, `Not started` and may not
  be in both tier lists; `not_done` must be declared `values` and none of them a `success` or
  `failure` status; `set_status` must be a `success` status; the evidence field must be in the
  type's `required_fields` or `known_fields`. Any breach is a load-time error naming the type and
  `statuses`. A type declaring no `delivery` is not examined by `status.delivery-agreement`.

  A type that declares no `statuses` behaves exactly as before. A declaration is read by:

  - `status.undeclared-value`: a `Status` outside `values`.
  - `status.required-field-missing` / `status.forbidden-field-present`: a `require` field that is
    blank, unedited template text or, when the type declares `pending`, one of its whole pending
    values (`MILE-153`) at the record's status, and a `forbid` field that carries a value. Both read
    the status by exact comparison, as every other rule does. `pending` is the type's one declared
    placeholder vocabulary: `field.pending` and this rule read the same list, so `Unknown`, a dash
    or `none recorded` can be declared once and no longer satisfies a requirement. A type that omits
    `pending` keeps the old reading, where such a value is present.
  - `status.delivery-agreement`: `delivery`, as above. Evidence computing to a delivered tier under a
    `not_done` status is an error, and so is a `success` status whose evidence computes to an
    unevidenced tier. `urzua fix --apply` writes `set_status` for the first (`SPEC-8`); the second has
    no single correct value and is repaired by adding evidence.
  - `field.pending`: when `pending` is declared, a field is pending when its whole value (trimmed,
    case-insensitive) equals one of the whole-value entries or matches one of the pattern entries. It **replaces** the compiled-in match, which treats
    any value containing `pending` as pending (`BUG-65`); a value that merely mentions the word is a
    real value. A type that omits `pending` keeps the compiled-in match. The rule's optional
    boolean `pending_until_required` (`MILE-169`, `false` by default) defers a pending value in a
    field some `statuses.require` entry names, until the record's status is one that requires it:
    the slot stays eligible and is counted unexamined. A field no `require` entry names is judged
    as before, and the option is refused on any other rule at load.
  - `field.quality`: the optional boolean `template_placeholders` (`BUG-22`, `false` by default). Declared
    `true`, a required field whose value is identical after trimming to the type's template text for that
    field is a Placeholder, where the template is `.urzua/templates/<type>.md` and its header is read for
    `field: value` pairs. The compiled-in placeholder tokens and the `nnnn` reference heuristic no longer
    apply to the rule, and a value is Pending only when the type's declared `pending` vocabulary says so. A
    type with no template is judged by that vocabulary alone. Declaring the option when no record type has a
    template is reported as a warning notice rather than a rule that reads nothing, and it is refused on any other
    rule at load. A template must therefore not carry a real default (a `Phase: 0` would flag every record
    that legitimately holds it); write the placeholder text a person is meant to replace.
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

- `citation.resolution` also takes the optional boolean `skip_revision_log` (`false` by default),
  with the meaning it has for `citation.path-exists`: an id cited in a record's revision log, the
  table under the type's declared `revision_log` identity, is not read and is not in the population.
  The rest of the record is still read (`MILE-156`).
- An entry is an exact id (`PREFIX-N`) or an inclusive numeric range (`PREFIX-LOW..HIGH`); numbers
  compare by value, so `RFC-04122` matches `RFC-4122`.
- The prefix must be one a record type declares. An entry under no declared prefix, a malformed
  entry, or a range whose start is after its end is a load-time error: it could never match, and a
  declaration that cannot fire is a silent no-op.
- A matching citation is counted in the rule's population as eligible and unexamined, not as out of
  scope, so it is visible in the report and never hidden, and `config.scope-matches-nothing` does not
  read a declared external id as a configuration that fails to reach its candidates (`BUG-186`). An id the list does not name is still resolved against the
  records and reported when it resolves to none. A declared external id wins over a record of the
  same id, so declare only ids no record answers to.

## Named, not cited: `citation.resolution`'s `non_reference_field`

A record can name an id without citing it as a reference: an example, a number minted and
retracted, a hypothetical corpus. `citation.resolution` takes an optional `non_reference_field`
naming a header field in which a record lists those ids (`BUG-190`, `ADR-75`):

```yaml
rules:
  citation.resolution:
    level: error
    non_reference_field: "Non-reference-ids"
```

```yaml
---
Status: Accepted
Non-reference-ids: ADR-72
---
```

- The field holds a comma-separated list of exact ids (`PREFIX-N`); there are no ranges. Ids compare
  by value (`ADR-072` is `ADR-72`) and the prefix is case-sensitive. Id parsing is the exact-id
  parsing of `external_ids`, so the two cannot disagree on what an id is.
- In a record that declares an id, a prose citation of it is counted as eligible and unexamined,
  neither resolved nor reported. In any other record the same id resolves as before. `external_ids`
  takes precedence. "Cited" means the prose the rule reads: an id appearing only in the header, a
  fenced block, or a revision-log row while `skip_revision_log` is on counts as uncited.
- A declared id the record never cites, and an entry that is not an exact id of a declared prefix,
  are findings on the record, naming the field, the entry and the line of the field.
- The option must name a field some record type lists in `known_fields` (or `required_fields`), and
  not a `pointer_fields`, `narrative_fields` or relation-role field, since those rules would resolve
  the ids as references; either failure is a load-time error. Other rules do not take the option.
- When a declared id resolves to a real record, `check` prints one aggregated `Info` notice naming
  the records and ids. A notice never changes the exit code.
- Adding the field to an Accepted record is a `structural` revision-log row, not an amendment.

## Cited issues and pull requests: `citation.issue-exists`

`citation.issue-exists` (`MILE-143`) checks that an issue or pull request number cited in a record's
references is in a snapshot file committed to the repository, so `check` never needs the network.
It requires `reference_sections` (as for `citation.path-exists`) and the structured option
`issue_refs`; there is no string mini-language (`ADR-64`):

```yaml
rules:
  citation.issue-exists:
    level: warn
    reference_sections: ["References"]
    skip_revision_log: true   # optional
    issue_refs:
      snapshot: ".urzua/known-refs.json"
      repository: "owner/name"
      forms: [number, url]
      fields: ["Found-in"]   # optional
```

- `snapshot` is the file, relative to the repository root. `repository` is the GitHub repository it
  describes, `owner/name`, compared without regard to case. Both are load-time errors when absent,
  empty, absolute, containing `..`, or malformed; `forms` must be non-empty with no repeat, and
  only `number` and `url` exist. The option belongs to this rule alone.
- `number` reads `#123` in the section's prose outside inline code, not `owner/repo#123` (another
  repository), not an anchor link `[x](#123)`. `url` reads `https://github.com/owner/name/issues/123`
  and `/pull/123` of the declared repository only; another repository's link is not read. A form
  not listed is not read. Fenced blocks are never read.
- A number is held to the snapshot's issues and pull requests together, since GitHub shares one
  number space and redirects either kind of URL to the other.
- `skip_revision_log` behaves as for `citation.path-exists`.
- `fields` (optional, absent by default) names header fields, as a type declares them, whose values
  are read for the same `forms` in addition to the `reference_sections`: a number in `Found-in` is
  held to the snapshot as one under `## References` is. A finding names the field and its header
  line. Each name must be non-blank, listed once and declared (`required_fields` or `known_fields`)
  by some record type; anything else is a load-time error, so a misspelt field cannot read nothing.
  A record without the field is not examined. Body prose outside `reference_sections` is not
  readable by an option: a bare `#N` there is as often an example, a heading counter or a quoted
  number as a citation, and a repository that wants a prose section checked names its heading in
  `reference_sections`.
- The snapshot is `{schema_version, repository, generated_at, issues, pulls}`, written by
  `urzua refs refresh` (`SPEC-1`) and committed. If it is absent, unparsable, of another
  `schema_version` or of another repository, the rule reports one finding on the snapshot path and
  counts every citation eligible and unexamined: it does not pass for want of its input.
- Staleness is visible, not silent: each finding names the snapshot's `generated_at` date and
  suggests the refresh, and `check` adds an info notice giving the date and the snapshot's size. A
  number cited before it exists in the snapshot is the expected case after opening an issue; refresh
  and commit.

## Cited paths: `citation.path-exists`

`citation.path-exists` (`MILE-143`) checks that a repository path cited in a record's references
exists. Where to look is configuration, not engine knowledge, so the rule requires
`reference_sections`:

```yaml
rules:
  citation.path-exists:
    level: warn
    reference_sections: ["References"]
    skip_revision_log: true   # optional
```

- `reference_sections` lists `##` heading texts, matched exactly after trimming. Only backtick-quoted
  code spans inside those sections are read; a record with no such section is out of scope.
- `skip_revision_log` is optional and `false` by default, so behaviour is unchanged unless declared.
  When `true`, each record's revision log (the line opening it under the type's declared
  `revision_log` identity, below, and the pipe table that follows, blockquoted or not) is not read,
  and the paths in it are not in the population. It exists for append-only history, whose rows name
  a path as it was when the row was written; the rest of the section is still read. It belongs to
  this rule and `citation.resolution` alone (`MILE-143`, `MILE-167`, `MILE-156`).
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
        format: "- [{id}]({file}) \u2014 {Status}, {Date}\n  {title}"
        begin: "<!-- urzua:index:begin -->"
        end: "<!-- urzua:index:end -->"
```

- `format` writes one entry. Its lines are separated by `\n`, so an entry may span several lines
  (a YAML mapping, or a link followed by a description); the artifact's non-blank lines are read
  that many at a time, and a block scalar's trailing newline is ignored. A blank line inside the
  format is refused, since blank lines are skipped when an index is read.
- Placeholders: `{id}`, `{file}` (the record's file name), `{title}` and `{Field}` for a declared
  header field, written by its exact field name (`{Status}`, `{Date}`; `MILE-164`). A field placeholder
  is filled from the record's header and checked against the index: a listed value that differs, or a
  record without the field, is a finding. The field must be in the index type's `required_fields` or
  `known_fields`: a placeholder naming anything else is a configuration error at load (a typo would
  otherwise compare nothing), and a name that is `id`, `file` or `title` is always the built-in one.
  Each placeholder appears at most once in the whole entry. An entry must name its record, by `{id}` or, when no id is written, by `{file}`;
  `{title}` alone is refused as ambiguous.
- `{title}` is the record's H1 without its identifier: a leading number (`# 12 -- Title`) or the
  record's own id (`# ADR-12: Title`, compared by value) is dropped with the separator after it.
- Only records with an identifier are the type's records: a README in the type's directory is not
  listed unless it is also declared as one.
- `urzua index` is read-only by default and reports each artifact as `current`, `stale` or `missing`;
  `--apply` rewrites the lines between `begin` and `end` (everything else is kept byte for byte), or
  the whole file when no markers are declared, and creates a missing artifact. Entries are in record
  id order, so a second run changes nothing and reports `current`. A record that cannot fill a
  placeholder (no H1 for `{title}`, no `Date` header for `{Date}`), a file that is not text and a marker that is absent are
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
    acyclic: false            # optional (MILE-27), default false
    types: [rfc]              # optional (MILE-155), default: every type declaring a side
    provisional:              # optional (MILE-154)
      statuses: [Draft]       # claimant statuses at which an unechoed claim is provisional
      level: warn             # warn (default) | error
      acknowledged_by: "Amended-by-pending"   # optional (MILE-163), a field on the peer
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
level, unless the pair declares `provisional.acknowledged_by` (`MILE-163`): then a provisional claim
whose peer names the claimant in that field is settled by the peer's own declaration and is not
reported. The field is read only for a provisional claim; a claim from any other status still needs the
inverse relation. It must be a non-blank field in the `required_fields` or `known_fields` of a type the
pair carries, and not itself a pair field, or the load is refused. A status listed that no record type declares in
`statuses.values`, a blank or duplicated status, an empty list, or a `level` other than `warn` and
`error` is refused at load, so the split cannot silently apply to nothing.

The narrowest reading of what `RFC-47` leaves open, each a decision recorded here until an ADR
revisits it:

- **Per-type binding** (`MILE-155`). Optional. A pair's two names are global by default: every type
  that declares a name in `required_fields` or `known_fields` carries that side, and a claim to a
  type that does not declare the echoing field is an unechoed claim. A pair that declares
  `types: [rfc]` is carried by those record types only: a record of another type is not that pair's
  business (its fields are not slots), and a claim whose target is another type is not judged: it is
  counted in the population as eligible and unexamined (not out of scope), not reported as unechoed, because that type cannot hold the
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

### Sequencing: `acyclic` (`MILE-27`, `RFC-48`)

`acyclic: true` declares the pair a sequencing relation that must not loop. The graph has one edge per
resolved claim, drawn the way the field declares it: `A.forward = B` is `A -> B` ("A cannot finish
until B does"), and `A.inverse = B` is `B -> A`, so a record that writes only the inverse side still
takes part. Two rules read acyclic pairs, and only those:

- `sequence.cycle` reports every record on a cycle, with a shortest cycle through it
  (`MILE-1 -> MILE-2 -> MILE-1`). A record that names itself is a cycle of one. The cycle may cross
  record types where the pair allows it. A record is reported even when it wrote none of the claims
  that close the loop.
- `sequence.unresolved-blocker` reports a value in either field that names no record: a reference
  that resolves to no discovered record, and a comma-separated entry that is not a record reference
  (prose), so prose cannot sit in the field and be dropped from the graph unseen. An entry is a
  reference only when the whole trimmed entry is one record reference: `MILE-4 MILE-5` and
  `MILE-5 (see note)` are prose. The nothing-here sentinel and an unwritten field are not findings.
  A target whose header cannot be read is disclosed under its own entry, not blamed on the citer.
  A claim a pointer rule already reports is not reported again (see Overlap below).

Readings of what `RFC-48` leaves open, decided in `ADR-76`:

- **`reciprocity` is independent of `acyclic`.** `none` is the intended spelling for a pair whose
  inverse is computed, never written (`Depends-on` / `Blocks`); `required` also works and adds the
  echo check.
- **`provisional` and `types` apply as for any pair.** A claim to a type outside `types` is no edge.
  `provisional` does not soften either sequence rule.
- **A written `Blocks` is an edge.** `RFC-48` calls `Blocks` derived and never written; it is declared
  only because a pair must name two fields some type declares. A repository follows `RFC-48` by writing
  `Depends-on` only. A `Blocks` that is written still counts, because reading the forward field alone
  would hide a loop through it.
- **Edges are only resolved claims.** A claim that resolves to nothing is `sequence.unresolved-blocker`'s,
  and does not hide a real cycle among the claims that resolve. An edge into a record whose header
  cannot be read is dropped too, so a cycle through such a record is not seen; the record is disclosed
  in the rule's population.
- **One graph per pair.** A loop that crosses two acyclic pairs is not detected by `sequence.cycle`.
- **One finding per record on a cycle.** A large cycle yields one finding per member, by design, so a
  diff-scoped run that examines one member still sees it.
- **Overlap.** When a pair field is also one of the type's `pointer_fields`, `sequence.unresolved-blocker`
  skips what the pointer rules already report: an unresolved target when `pointer.resolution` is
  enabled, and a prose entry when `header.pointer-field-clean` is enabled. In a repository that follows
  `RFC-48` (`Depends-on` a pointer field, both rules on) it therefore reports nothing and acts as a
  guard for the roadmap: the graph is only computed when no claim was dropped.
- **Not built here.** `sequence.phase-order`, `sequence.failed-dependency`,
  `sequence.terminal-parent-open-child`, the `condition` record type, the derived `blocked` value
  and `urzua roadmap` are `RFC-48`'s other parts and belong to other milestones (`MILE-144`,
  `MILE-182`).

Enabling either sequence rule while no pair declares `acyclic: true` is a load error, for the reason
the pair rules refuse to run with no pair: it would examine nothing.

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
- `drift_audit_field` (a field name some record type declares; needs `drift_grace_days`): a record
  that carries a `YYYY-MM-DD` date in that field is held back while that date is newer than the
  window, in place of the cited file's last commit, so the window counts from the day the evidence
  was last audited. A record without the field, or with a blank one, counts from the file's last
  commit as before; a malformed date is a warning from `embodiment.consistency` naming the value
  and falls back the same way. A blank name, an undeclared field, or the option without
  `drift_grace_days` is a load error.
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
  there would be scanned), and one covering no tracked file is a finding. An entry containing a glob
  character (`**/*.test.*`, `crates/*/tests`) is matched as a glob, the same matcher the diff rules'
  path options use; any other entry stays a prefix. A glob names files wherever they sit, so it is
  read only among files under a `back_pointer_paths` entry, is exempt from the containment check,
  must match at least one such file (else a finding), and a glob that cannot compile is a load error.
- `embodiment.consistency` judges a record with `Embodiment` but no `Realized-by` field when a
  back-pointer cites it, computing from the back-pointers alone; the finding says so. A record
  neither cited nor carrying `Realized-by` is judged only when it states `Verified` or
  `Implemented`: that claim has no evidence of either kind, so it is reported against the computed
  `Not started`. A record stating any other value there is not judged and stays unexamined in the
  census. `urzua fix` repairs the cited records only.

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
- A listed value that evidence computes (a state of the declared `embodiment` vocabulary, below) is
  a load error: it would make that tier's drift unreportable. A blank entry
  is refused like any other blank option entry.
- Omitted, every stated value is held to the computed one, as before. The option belongs to
  `embodiment.consistency` alone.

### The Embodiment vocabulary (`MILE-144`)

Which state names evidence computes to is declared, not compiled in. The top-level `embodiment` key
lists them highest tier first, with the category of evidence that lifts a record to each:

```yaml
embodiment:
  drift: Drift detected          # the alarm state; overrides every tier
  tiers:                         # highest first
    - {state: Verified, evidence: test, claims_built: true}
    - {state: Implemented, evidence: code, claims_built: true}
    - {state: Specified, evidence: spec}
  none: Not started              # no evidence at all
```

- `evidence` is `test`, `code` or `spec`: a `Realized-by` locator of that kind, or a citing source
  comment (`Verifies:` is `test`, `Implements:` is `code`). The first tier whose evidence exists wins,
  so the list order is the tier order.
- `claims_built: true` marks a state that says the work was built. A record in the type whose `Status`
  is not accepted cannot state one (`embodiment.requires-accepted-status`), and a record stating one
  with no `Realized-by` field and no citing back-pointer is judged (`MILE-161`).
- Omitted, the vocabulary above is used, so a repository that declares nothing is unchanged. The
  declaration is validated at load: no blank or repeated state name (across `drift`, every tier and
  `none`), at least one tier, and one tier per evidence category.
- `check`, `fix`, `status.delivery-agreement` (its `delivered_tiers` and `unevidenced_tiers` name
  states from this vocabulary other than `drift`) and `sticky_states` all read this one declaration.

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
    date_order:
      - {earlier: Date, later: Accepted-on}   # `earlier` may not be later than `later`
```

- `date_order` (required, non-empty): each entry is a mapping with exactly the keys `earlier` and `later`, two distinct field names (an unknown key or a string entry is refused).
  Both must be declared (`required_fields` or `known_fields`) by one record type, which is the only
  kind of record the entry applies to; anything else is a load-time error, as is the option on any
  other rule.
- Equal dates pass. A value that is absent, blank, placeholder, pending or not a real `YYYY-MM-DD` date
  is not compared: `date.field-valid`, `field.quality` and `status.required-field-missing` own those
  states, so one defect is not reported twice. Such a slot is eligible and unexamined in the
  population.
- It adds no second notion of "a date": it reads the same field states and the same date validity as
  `date.field-valid`.

`waiver.matches-nothing` (`MILE-168`) takes no options; enable it as `waiver.matches-nothing: warn`. It
reports, on the waiver record, a waiver that is expired, names a rule this build does not have, or is
active but matches no finding of a rule that ran over the whole corpus. A path-scoped or `--base`
run, and a waiver whose target rule is off, leave a live waiver unjudged (eligible, unexamined).

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
> | 2026-11-04 | Bumped to `0.29`. An `exclude_files` entry that matches no file directly in the type's `dir` is reported as a `type.dir-matches-nothing` warning. **Why:** `MILE-150` -- an exclusion that silently does nothing hides a renamed or removed file and violates the no-silent-no-op principle. | **substantive** |
> | 2026-11-04 | Bumped to `0.30`. `date_order` entries are mappings `{earlier, later}` instead of an `Earlier <= Later` string; a relation pair's `types` scoping counts a claim to an uncarried type as eligible and unexamined, not out of scope. **Why:** `ADR-64` (structured in, structured out) for `MILE-157`, and `MILE-155` -- a legitimate cross-type claim is not a configuration the tool cannot reach. | **substantive** |
> | 2026-11-04 | Bumped to `0.31`. Documented the optional `skip_tables` option of `citation.path-exists` (markers naming tables whose rows are history and are not read; empty by default), and that a declared `external_ids` citation is counted eligible and unexamined, not out of scope. **Why:** `MILE-143` -- a revision log under `## References` names paths as they were; `BUG-186` -- a declared external id was reported by `config.scope-matches-nothing` as a misconfiguration. | **substantive** |
> | 2026-11-04 | Bumped to `0.32`. Removed the `blockquote` and `bold-list` header shapes: `header_shape` defaults to and accepts only `yaml-frontmatter` and `none`, `fields.from: blockquote` and `bold-list` fail the load naming the form, and the `header_layout` option is gone. **Why:** `MILE-85` (maintainer decision of 2026-11-04: full cleanup at switchover; resolves `BUG-184`). | **substantive** |
> | 2026-11-04 | Bumped to `0.33`. Documented the opt-in `waiver.matches-nothing` rule (no options). **Why:** `MILE-168` -- a stale or expired waiver stayed on file silently. | **substantive** |
> | 2026-11-04 | Bumped to `0.34`. Added the per-type `revision_log` declaration (`marker` or `heading`, default the marker `**Revision log**`), matched as the opening line and never by containing the text; `citation.path-exists`'s `skip_tables` marker list became the boolean `skip_revision_log`, which skips the declared revision log (the list was unreleased). **Why:** `MILE-167` -- a hardcoded literal contradicted `ADR-64`, and prose containing it made the parser read the wrong table. | **substantive** |
> | 2026-11-04 | Bumped to `0.35`. A record exempted by `sections.exempt_statuses` is counted eligible and unexamined, not out of scope. **Why:** `BUG-187` -- the exemption is a declared input, and `config.scope-matches-nothing` warned that the rule could not reach it. | **substantive** |
> | 2026-11-04 | Bumped to `0.36`. `test_paths` accepts globs as well as prefixes, matched by the diff rules' glob matcher. **Why:** `MILE-160` -- test files named by suffix cannot be named by a prefix list. | **substantive** |
> | 2026-11-04 | Bumped to `0.37`. Documented that `embodiment.consistency` reports a record stating `Verified` or `Implemented` with no `Realized-by` field and no citing back-pointer. **Why:** `MILE-161` -- a claim with no evidence passed silently. | **substantive** |
> | 2026-11-04 | Bumped to `0.38`. Documented `drift_audit_field` of `embodiment.consistency`: the `drift_grace_days` window counts from a declared last-audit date on the record when present, else from the cited file's last change; a malformed date is reported. **Why:** `MILE-166` -- a file changes for reasons unrelated to its evidence, so its last change mis-dates the audit. | **substantive** |
> | 2026-11-04 | Bumped to `0.39`. `statuses.pending` accepts the structured pattern entry `{pattern: "..."}` beside whole values, for a placeholder family; an invalid, blank or repeated pattern is a load error. **Why:** `MILE-162` -- an exact-value list needs an entry per variant and misses the next one. | **substantive** |
> | 2026-11-04 | Bumped to `0.40`. Removed the `Last-audited: 2026-11-04` field from this spec's header. **Why:** it was added only to exercise `MILE-166` and no audit took place, so it attested to something that did not happen; the `Last-audited` declaration and `drift_audit_field` in the configuration are unchanged. | **structural** |
> | 2026-11-04 | Bumped to `0.41`. A pair's `provisional` takes an optional `acknowledged_by` field: a provisional claim whose peer names the claimant in it is settled. **Why:** `MILE-163` -- a peer records the acknowledgement of a pending claim under a field of its own naming. | **substantive** |
> | 2026-11-04 | Bumped to `0.42`. An index `format` may use `{Field}` placeholders naming a declared header field (`{Status}`, `{Date}`), filled from and checked against the record; a placeholder naming no declared field of the type fails the config load. **Why:** `MILE-164` -- real indexes list status and date beside the title, and a misspelled column would otherwise silently compare nothing. | **substantive** |
> | 2026-11-04 | Bumped to `0.43`. `sections.required` entries take `aliases` (other headings satisfying the requirement, disclosed as an info notice per alias) and `preamble: true` (a `y-statement` with a `marker` read from the text before the first `##` when its section is absent). **Why:** `MILE-165` -- adopter records name a section differently or carry the Y-statement in the preamble. | **substantive** |
> | 2026-11-04 | Bumped to `0.44`. `citation.resolution` takes the optional `skip_revision_log` option, with the meaning it has for `citation.path-exists`. **Why:** `MILE-156` -- revision logs are append-only history, so an id they cite as it was must not be reported or rewritten. | **substantive** |
> | 2026-11-04 | Bumped to `0.45`. `field.pending` takes the optional `pending_until_required` option, off unless declared `true`: a pending value in a field the type's `statuses.require` ties to certain statuses is not reported before the record reaches one of them, and the skipped slot stays eligible and unexamined. **Why:** `MILE-169` -- a placeholder `Regression-test` is legitimate on an `Open` bug and a gap on a `Fixed` one; `status.required-field-missing` already judges the latter. | **substantive** |
> | 2026-11-04 | Bumped to `0.46`. Documented the opt-in `citation.issue-exists` rule and its structured `issue_refs` option (`snapshot`, `repository`, `forms: [number, url]`); `reference_sections` and `skip_revision_log` are also accepted by it. **Why:** `MILE-143` -- the issue and pull request half; `SPEC-1` documents the `urzua refs refresh` command that writes the snapshot. | **substantive** |
> | 2026-11-04 | Bumped to `0.47`. A type's `revision_log` takes `required: true|false` (default `true`), which `urzua fix --apply` reads; it may stand alone. **Why:** `MILE-159` -- records without a revision log section could never be written by `fix --apply`. | **substantive** |
> | 2026-11-04 | Bumped to `0.48`. A type's `statuses` declaration gains an optional `delivery` mapping (`delivered_tiers`, `unevidenced_tiers`, `not_done`, `set_status`), validated at load, and the opt-in rule `status.delivery-agreement` reads it. This repository declares it on `milestone`, which now knows `Realized-by`, and its `implementable` list gains `Done`. **Why:** `MILE-183` -- no rule compared a stated `Status` with delivery evidence, so about thirty built milestones read `Planned`; the mapping is the type's declaration, not engine policy. | **substantive** |
> | 2026-11-04 | Bumped to `0.49`. `citation.issue-exists`'s `issue_refs` takes the optional `fields` list of header field names to read for citations, off unless declared, validated against the types' declared fields. **Why:** `MILE-184` -- a wrong number in a header field such as a BUG's `Found-in` went unchecked. | **substantive** |
> | 2026-11-04 | Bumped to `0.50`. `relation_fields` takes `date` and `author`, the fields `urzua new` fills with today's date and the author, defaulting to `Date` and `Author`. **Why:** `MILE-176` -- `new` named them by literal, so a type calling them `Created` and `Owner` got `null`. | **substantive** |
> | 2026-11-04 | Bumped to `0.51`. A field named under `relation_fields.date` or `relation_fields.author` must be in the type's `required_fields` or `known_fields`, reported by `config.relation-field-not-known` as the four relation roles already were. Which rules accept which options is declared on the rule's table entry, not in a list of owners in the config loader; the load errors are unchanged. **Why:** `MILE-176` left the two new fields unchecked, and `MILE-171` moved option ownership to the rule table. | **substantive** |
> | 2026-11-04 | Bumped to `0.52`. No change to the option reference. The `RuleSetting` option set is one definition (`accepted_statuses` now carries the `serde(default)` its siblings did), and a rule reads its options as a typed value. An option that is declared in the option tables but not accepted by the struct, or the reverse, fails a test. **Why:** `MILE-174` -- the set was defined four times, so a new option could be silently ignored. | **structural** |
> | 2026-11-04 | Bumped to `0.53`. Documented the per-type `extra_dirs` declaration: further directories of one type, one identity namespace with `dir`; duplicate and shared directories are load errors. **Why:** `MILE-130` -- a prefix may not be shared (`BUG-173`), which left monorepo, archive and mid-migration layouts with no correct configuration. | **substantive** |
> | 2026-11-04 | Bumped to `0.54`. Documented the top-level `embodiment` declaration: the drift state, the tiers in order with the evidence category and `claims_built` flag of each, and the no-evidence state; it replaces the state names compiled into `compute`, `fix` and the consistency rules, and `sticky_states` and `statuses.delivery` are validated against it. Omitted, the built-in vocabulary applies. **Why:** `MILE-144` -- the Embodiment vocabulary was hardcoded in three places, against `ADR-64`. | **substantive** |
> | 2026-11-04 | Bumped to `0.55`. `sections.depth` now drives the section reading: a type declaring `depth: 2` has its `###` headings parsed as sections nested under their `##`, and a required section may declare `level` and `under` to be asked for one. A type declaring no `level` is read as before. **Why:** `MILE-98` -- this repository's own `###` headings were invisible to the section rules and a corpus that checks a considered option's subsection needs `###` to be checked at all. | **substantive** |
> | 2026-11-04 | Bumped to `0.56`. Documented the per-type `companion_fields` declaration: a list of pairs of relation role keys (`status`, `embodiment_state`, `embodiment_locator`, `supersession`) whose fields the type declares together, resolved through `relation_fields`; an entry that is not two distinct role keys is refused at load, and `config.companion-declaration-missing` reports a pair the type declares one half of or neither. **Why:** `BUG-41` -- the one companion pair checked was hardcoded. | **substantive** |
> | 2026-11-04 | Bumped to `0.57`. A type's `statuses` declaration gains `initial` and `transitions`, its lifecycle graph, validated at load, read by the new opt-in rules `status.transition-allowed` (needs `--base`) and `status.unreachable-state`. The `embodiment` declaration must mark at least one tier `claims_built`. **Why:** `MILE-144`, `ADR-72`. | **substantive** |
> | 2026-11-04 | Bumped to `0.58`. Documented the optional `non_reference_field` option of `citation.resolution`: a header field listing the exact ids a record names without citing as a reference, set aside in that record only, with a finding for a declared id never cited and for a malformed entry, a load error for a field that is unknown or a pointer, narrative or relation-role field, and an aggregated Info notice for ids that now match a real record. **Why:** `BUG-190`, decided by `ADR-75`. | **substantive** |
> | 2026-11-04 | Bumped to `0.59`. Documented the `field.quality` option `template_placeholders`: off unless declared `true`, it reads a required field whose value is identical after trimming to its type's template text for that field (`.urzua/templates/<type>.md`) as a placeholder, in place of the compiled-in placeholder tokens. **Why:** `BUG-22`, maintainer decision of 2026-11-04: a token list transcribed from one repository's templates protects no other corpus. | **substantive** |
> | 2026-11-04 | Bumped to `0.60`. Documented `acyclic` on a `relation_pairs` entry and the rules `sequence.cycle` and `sequence.unresolved-blocker` that read it, with the readings `ADR-76` decides for each question `RFC-48` leaves open, including that a written `Blocks` is an edge and that `sequence.unresolved-blocker` yields to enabled pointer rules. Enabling either rule with no acyclic pair is a load error. This repository declares the `sequencing` pair (`Depends-on` / `Blocks`). **Why:** `MILE-27` -- the declared sequencing relation with cycle detection. | **substantive** |
> | 2026-11-04 | Bumped to `0.61`. Documented the `revision_log.version_bump` declaration: a field name and a list of change classes whose added revision-log rows require that field to change, read by `revision-log.version-bump-required`. **Why:** `BUG-51`, maintainer decision of 2026-11-04; `SPEC-18` requires a `Version` bump with every substantive or structural entry and nothing checked it. Off for every type unless declared. | **substantive** |
> | 2026-11-05 | Bumped to `0.62`. Documented the per-type `role: suppression` declaration: the engine reads which type suppresses findings from it instead of the type name `waiver`; at most one type declares it; the type named `waiver` is the suppression type only when no type declares the role. **Why:** `BUG-74`, maintainer decision of 2026-11-05 -- the literal made suppression silently absent for a corpus that named its exception record anything else. | **substantive** |
