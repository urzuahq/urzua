---
Stable-Id: 01M6R3NK5J3T1JRZ9DY1C0PZJF
Version: '0.127'
Date: 2026-08-20
Status: Shipped
Realized-by: code:rust/crates/urzua-core/src/config.rs@sha256:a9ed379356481d887473b4623f74559b8ca8b83fd5184c518c677535a78646ff, test:rust/crates/urzua-cli/tests/own_config_integration.rs@sha256:92a2b9ceca89133f30987a013734da66b9332c4e937a87e72e53571596e18093, test:rust/crates/urzua-cli/tests/field_levels_integration.rs@sha256:3c8aa671582772499ab7345cdbfb5a4dc2c50462882851fb20f853992acfd57c
Embodiment: Verified
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
  `revision-log.change-class-required` reads the same flag (`BUG-50`): a record of a type that
  declares `required: true` and has no such section is a finding, with the `edit-section` `add` action naming
  the identity to add; a type that does not declare `required: true` (omitted, or `false`) is not held to a log by that
  rule, and its records without one are eligible and unexamined.
  `required` may stand alone (`revision_log: {required: false}`), leaving the identity at its
  default; an empty `{}` is still an error. `revision-log.change-class-required`,
  `urzua fix`'s revision row and the `skip_revision_log` option of `citation.path-exists` and `citation.resolution` all read it.
  The declaration also takes `version_bump: {field, classes}` (`BUG-51`), absent for every type
  unless written: a field name and the change classes (the revision log's Class column) that require
  that field to change. `revision-log.version-bump-required` reports a changed record that adds a row
  of one of those classes while its field equals the merge base's. `field` must be non-blank and
  `classes` a non-empty list of non-blank names, else the config fails to load; `version_bump` may
  stand alone like `required`. Enabling the rule when no type declares one is a load error.
  `version_bump` also takes an optional `initial`, a non-blank version: `urzua new` writes it into
  the `field` of a new record (`MILE-74`), and without it the field starts blank. `relation_fields`
  takes an optional `title` beside `date` and `author`: the field `new` fills with the record's
  title, subject to the same `config.relation-field-not-known` check.
  The declaration also takes four optional keys (`ADR-94`, `MILE-213`), each of which may stand alone:

  ```yaml
  revision_log:
    classes: [substantive, structural]   # the vocabulary
    class_column: Class                  # a header cell's name
    date_column: Date                    # a header cell's name
    tool_class: structural               # what fix writes
  ```

  Undeclared, each keeps today's behaviour from defaults held in the engine: `classes` is
  `[substantive, structural]`, the class is the row's last cell, the date is its first cell, and
  `tool_class` is `structural`; a type declaring none behaves exactly as before and the
  `change-class-required` message reads as it always did. `classes` is a non-empty list of
  non-blank names, distinct after stripping emphasis (`*`) and whitespace and compared
  case-sensitively (`structural` and `**structural**` are one name, `Structural` and `structural`
  are two); the `change-class-required` finding lists them. `class_column` and `date_column` name a
  header cell: a cell matches when, emphasis and whitespace stripped, it equals the name exactly
  (case-sensitive), in any position. A log whose header lacks a declared column, or has it more than
  once, is one finding on the record (`revision-log-column-unreadable`, with an `edit-section`
  action) and none of its rows is judged; `revision-log.version-bump-required` reports such a
  record unreadable, in the log now or at the merge base, and never reads it as an empty log. A row
  shorter than the class column has a blank class. A pipe written `\|` stays inside its cell, as in GitHub-flavoured Markdown. A row wider than the header (a cell holding an unescaped pipe) is one `change-class-required` finding naming the row and its cell count when a column is declared, and its columns are not guessed; undeclared, the class is the row's last cell as before. `class_column` and `date_column` may not name the
  same header. When `classes` is declared, every `version_bump.classes` entry and `tool_class` must
  be in it, and `tool_class` may be omitted only when the vocabulary has `structural`; a violation
  is a load error naming the type and the entry or key. With `classes` undeclared, `version_bump.classes`
  is not checked against the default pair. A declared class absent from `version_bump.classes`
  requires no bump. These keys are additive and optional, so `schema_version` is unchanged.
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
- **The suppression type may declare the fields a suppression needs to take effect:
  `effective_requires`** (`BUG-177`). A list of field names the type declares (`required_fields` or
  `known_fields`); only on the type with `role: suppression`:

  ```yaml
  record_types:
    waiver:
      role: suppression
      effective_requires: [Reason]
  ```

  A suppression whose listed field is absent, blank, a placeholder or pending (the type's
  `statuses.pending` vocabulary when it declares one) takes no effect: the finding it targets is
  reported again, and with `waiver.matches-nothing` enabled the waiver itself is reported
  (`waiver-ineffective`, naming the field). Omitted, a suppression needs only `Rule` and `Scope`, as
  before. An empty list, a blank or repeated name, a name the type does not declare, or the key on a
  type without the role is a load error.
- **The suppression type may declare where a suppression can be permanent: `permanent`**
  (`ADR-86`). A list with one entry per rule; only on the type with `role: suppression`:

  ```yaml
  record_types:
    waiver:
      role: suppression
      permanent:
        - rule: status.delivery-agreement
          reason_field: Category        # a field the type declares
          categories:
            - {name: docs-or-workflow-only}
            - {name: test-removed-by-decision, cite: required}
          cite_field: Justified-by      # a pointer field the type declares
  ```

  Declared, a suppression without `Expires` takes effect only if its `Rule` has an entry, its
  `reason_field` holds one of that entry's categories, and, for a category with `cite: required`, its
  `cite_field` names a record that resolves. Otherwise it suppresses nothing, its finding is reported
  again, and with `waiver.matches-nothing` enabled the waiver is reported (`waiver-ineffective`,
  naming the unmet requirement). **Declaring the key closes permanence for every rule it does not
  list.** A suppression with an `Expires` is unaffected, and `check` adds an info notice
  (`permanent-waivers`) counting the permanent suppressions in effect by category. Omitted, a
  suppression without `Expires` is allowed for any rule, as before. The engine names no category, rule
  or field. A load error: the key on a type without the role, an empty list, a rule this build does
  not ship or one listed twice, a `reason_field` or `cite_field` the type does not declare, a
  `cite_field` that is not one of its `pointer_fields`, an empty, blank or duplicated category list,
  or a category with `cite: required` and no `cite_field`.
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
  entry is validated like `dir` (it names a directory below the repository root, written in canonical form: no `./` or empty component and no trailing `/`, so `./docs/adr`, `docs/adr/` and `docs//adr` are load errors that name `docs/adr`); a directory listed
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
    finding. The H1's id is read the way the file name's is (`BUG-32`): the declared `prefix`, a hyphen and an ASCII number, padded or not; any other leading token carries no id and is never reduced to digits. A type with no pattern keeps a strict read, a leading `PREFIX-N` or a bare `N`. `urzua new` reads the declaration too: it numbers by the pattern, fits the file name's
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
      never assumed exempt, and a record with no status is not exempt. A listed status that no
      record of the type holds sets nothing aside, and `type.dir-matches-nothing` reports it, naming
      the type and the status (a type with no records at all is left to that rule's other checks).
      Declaring it without
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
  committed status is no longer declared is counted unreadable by the first rule, not judged. A record the change renamed is matched to its merge-base copy by `Stable-Id` (`BUG-191`), so it is judged against the status it had before the rename; with no `Stable-Id` or no match it is new, and with several matches it is counted unreadable. This repository's policy is that an ADR or RFC enters at `Proposed` or `Draft` and is accepted in a later change, so its `initial` is that one status; a bug or milestone may be filed already finished, because the PR that fixes or delivers it files it. The
  The derived `blocked` value is the next declaration.

  `progress` (optional, `MILE-144`, `RFC-48`, `ADR-83`) names the statuses that claim the work has
  advanced, for example `InProgress` and `Done`; it must be a subset of `values`, deduplicated, and
  disjoint from `failure` and `blocked`. It may include a `success` status, because a record cannot
  finish before its dependencies do. `implementable` is not read for this: it means "code may cite
  it". `blocked` (optional) names the non-terminal statuses a person writes to say a record is
  waiting, for example `Blocked`; it must be a subset of `values` and disjoint from `success`,
  `failure` and `progress`. `RFC-48` computes `blocked` and never writes it, so a repository that
  follows it declares `progress` and no `blocked`, and no record states a waiting status; a type that
  still writes one declares `blocked` to have it checked both ways.

  With a sequencing relation declared in `roadmap.pairs` (below), a record is blocked when any record it
  depends on, in either field of a declared pair (a written `Blocks` counts), is not in a status its own
  type declares `success`. `success` is the one list of done statuses. A dependency in a `failure`
  status, with no `Status`, or whose type declares no `success` therefore blocks. The opt-in rule
  `sequence.status-agreement` reports a blocked record that states a `progress` status, and a status
  listed in `blocked` while nothing blocks the record; a blocked record in a status that claims no
  progress, `Planned` for instance, is correct. Only a type that declares `progress` or `blocked` is
  examined. The finding offers `edit-field` `set` on `Status`, carrying the declared blocked status as
  the value only when the type declares exactly one, the record is not in a `success` status, and the
  declared `transitions` (when any) allow the move. A value that cannot be computed is not guessed: a
  record in a `blocked` or `progress` status that would be reported unblocked is counted unreadable when
  it names a `Depends-on` value that resolves to nothing or to a record whose header cannot be read, or
  when a record that may write an inverse claim has an unreadable header; a type that declares only
  `progress` is guarded the same way. Enabling the rule with no `roadmap.pairs`, or with no type
  declaring `progress` or `blocked`, is a load error.

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
  - `header.field-set-consistency`: the optional boolean `allow_extension_prefix` (`MILE-141`, `false`
    by default) exempts header keys beginning `x-`, in any case. The optional map `extension_prefixes`
    (`MILE-141`) names a record type and the prefix its extension keys begin with, used for that type
    in place of `x-`, in any case. A type the map does not name keeps the rule-level behaviour. Load
    refuses a prefix that is empty, holds whitespace or starts a field the type declares (compared
    in any case), and a type that is not declared or declares no `known_fields`, since the rule would
    read nothing for it. Either option is refused on any other rule at load.
  - `field.pending`: when `pending` is declared, a field is pending when its whole value (trimmed,
    case-insensitive) equals one of the whole-value entries or matches one of the pattern entries. It **replaces** the compiled-in match, which treats
    a value containing `pending` as a whole word as pending (`BUG-65`); `spending` and a path such as
    `docs/pending-settlement/x.md` (a token with a `/`) are real values, and a value that merely mentions the word is a
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
    type declares none (`BUG-178`). The list is therefore optional once no record type needs it, that
    is when every type either declares `statuses` or declares no status field (a suppression type with no
    `Status`); a declared list is still validated, and the list is required again as soon as one type
    declares a status field and no `statuses`. `pointer.target-status`'s `not_in` is a separate policy and is
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

## Declared, not parsed: `claim.status-agreement`'s `closes`, `strict` and `verb_claims`

A changeset fragment says which records it closes as data, in a frontmatter key, and
`claim.status-agreement` checks the data instead of a sentence (`BUG-66`, `ADR-85`):

```yaml
rules:
  claim.status-agreement:
    level: error
    claim_paths: [".changeset"]
    closed_statuses: ["Fixed", "Done"]   # the verb reading's list; see below
    closes: closes                       # optional: the frontmatter key; none declared, nothing below applies
    strict: true                         # optional, default false, needs closes
    verb_claims: false                   # optional, default true, needs closes; false stops prose being read
```

```yaml
---
default: minor
closes: [BUG-60, BUG-61]
---
```

- **`closes`** names the key. The engine knows no field name, record type or status: the listed ids
  resolve against whatever types the repository declares. It must be one key, not empty, with no
  whitespace, `:` or `#`. Declaring neither option leaves the rule exactly as it was.
- **`strict`** (`false` by default) makes a claim file that has a frontmatter block and lacks the
  key a finding (`claim.field-missing`), so a fragment that names a record without saying whether it
  closes it is caught. `closes: []` is the declaration that it closes nothing. `strict` without
  `closes` is a load-time error, and so is either option on any other rule.
- **`verb_claims`** (`true` by default, `ADR-88`, `MILE-197`) set to `false` turns the closing-verb
  reading off: no line of a fragment, frontmatter or body, is read as a prose claim, so only the
  `closes` field is a claim, and `check` prints an `Info` notice that the reading is off instead of
  a count. It is independent of `strict`, and the two compose: `strict` requires the key,
  `verb_claims` stops prose being read. A repository that turns prose off should keep `strict` on,
  because a fragment without the key then has no claim at all; with `verb_claims: false` and
  `strict` off, `check` adds a notice saying so. `verb_claims` without `closes` is a load-time
  error whether the value is `true` or `false`, and so is the option on any other rule. Absent, the
  reading is unchanged. Removing the reading from the engine is a separate decision.
- **The value is one line:** a flow list `[A-1, B-2]`, a single id, or `[]`. Single or double
  quotes around an id are read as the id. The parser is deliberately small: it reads the `---`
  block the file begins with, reads only the declared key, and ignores every other key, including the
  release tool's own bump keys. A block list (`closes:` then `- ` lines), a list continued over
  several lines, a key written twice, an empty entry or a value that is not a record id is
  `claim.field-malformed`, naming the file and the key's line; it is never guessed at.
- **A listed id that resolves to no record** is `claim.id-unresolved`, an error. It applies only to
  ids from this field: a reference the closing-verb reading finds that matches no record is still
  skipped and counted as eligible and unexamined.
- **A listed record must be closed.** Closed is the type's declared `statuses` terminals (`success`
  and `failure`), never `closed_statuses`: a type declaring none has no declared close, so a record
  of it listed here is reported. An open record, or one whose status cannot be read, is an error
  naming the record and its status, with a suggested `edit-field` on that record's `Status`
  (`ADR-73`); the finding does not choose between closing the record and taking it off the list.
  Every claim file under `claim_paths` is read, with or without `--base`.
- **A file with no frontmatter block** (it does not begin with a closed `---` block) is not a
  fragment: it is counted out of scope in the rule's population and named in an `Info` notice, and
  never given a finding. That is how a `README.md` in the directory is excluded without the engine
  knowing its name. The exception is a file that tries to be a fragment and the release tool cannot
  read: one whose `---` fence is never closed, or with a blank line before the opening `---`. Under
  `strict` it is `claim.field-malformed` (the release tool rejects both with
  `could_not_read_changeset`); without `strict` it is read as no frontmatter, as before.
- **Prose is not read for this claim.** The frontmatter block is skipped by the closing-verb
  reading once `closes` is declared, so the key is not read again as the verb "closes". That reading
  still runs over the body, unchanged, as a deprecated convenience, unless `verb_claims: false` turns
  it off; `check` prints an `Info` notice with how many closing-verb claims it found, so a repository
  can see when it is safe to stop. Removing it is a separate decision.
- **The release tool must tolerate the key.** The pinned knope accepts a one-line list, a scalar
  and `[]`, ignores the key and gives the same bump and changelog, and rejects a block list and a
  list continued over two lines. `.github/scripts/knope-closes-compat.test.sh` runs the pinned knope
  over a fragment carrying the key (`make knope-closes-compat`; CI installs the pinned release) and
  fails if the installed version is not the pin. A workspace of several named packages was not
  tested; an adopter with one should run the same test before declaring the field.

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
  the whole file when no markers are declared, and creates a missing artifact; the file is replaced
  through a temporary file and a rename, so a failed write leaves the old artifact. Entries are in record
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
`urzua audit` (an alias of `check`, `ADR-79`) exit 1 (`MILE-145`):

```yaml
fail_on: warning   # error (default) | warning
```

- **`error`** (the default, and the behaviour when the key is absent): only an error finding exits 1.
- **`warning`**: an unwaived warning also exits 1. A waived finding never does.
- An `info` finding (`level: info`, `ADR-87`) exits 0 under both values and never sets `blocking`.
- `fail_on: info` does not exist: `info` is the level below the line `fail_on` draws, and a repository
  that wants every finding to fail uses `warning` with rule levels `warn` or `error`.
- Any other value, including `info` and a different case, is refused at load, and the command exits 2.
- `urzua check --fail-on error|warning` overrides the key for one run, in either direction.
- Only the exit code changes. The report is byte-identical under either value: `blocking` and
  `status` keep meaning "an error", so a run that exits 1 on a warning shows `"blocking": false`.
  Read the exit code, not `blocking`, to know whether the threshold was crossed.

## Override log: `override_log`

A top-level `override_log` declares the committed log of editor-hook overrides (`ADR-78` item 1,
`MILE-188`), a repository-relative file path:

```yaml
override_log: .urzua/override-log.md
rules:
  override.unreviewed: warn
```

- **Format.** A Markdown table with the columns `Date`, `Name`, `Record`, `Field`, `Rule`, `Reason`
  and `Reviewed-by`, in that order, the same table shape as a record's revision log. Prose and
  headings around the table are free. The hook writes the file's introduction and header when the
  file does not exist, and appends one row for each finding it lets through, `Reviewed-by` blank.
- **Reviewed means a person has named themselves.** An entry is reviewed once its `Reviewed-by` cell
  names a person: a blank cell, a placeholder (`TBD`, `todo`, `name`, or a cell of only dashes) and a
  value the repository's record types declare as pending (their `statuses.pending` vocabulary, the
  one `field.pending` uses) are not a review, and neither is the word `pending` where no type declares
  a vocabulary. Setting it is an edit to the committed file, so the change is visible in the pull request
  that makes it. It is the smallest form that needs no git history, no record type and no second
  file: an alternative that asks whether a later commit acknowledged the entry would make the verdict
  depend on history a shallow checkout cannot see (the failure `ADR-69` names).
- **`override.unreviewed`** (`SPEC-2`) reads the file and reports each entry not reviewed in that
  sense as a warning on the log, naming the override, the record, the rule and the reason. A
  row with the wrong number of cells is a finding, never skipped. A declared log that does not
  exist, or that is not text, is reported and counted unreadable: it is never read as an empty log.
  The hook creates the file on the first override, so a repository that declares `override_log`
  commits the file with its header.
- **Declared, never defaulted.** Enabling `override.unreviewed` without `override_log` is a load
  error (exit 2), as is a value that is blank, absolute, ends in `/` or has a `..` component. Declaring
  `override_log` without the rule is allowed: the hook still records overrides, and nothing reports them.
- The hook (`SPEC-8`) is the only other reader of the key: it takes the top-level `override_log:` line
  from `.urzua/config.yaml`, and when it is absent it says the override is not recorded.

## The agent harness: `harness` and `authority`

A top-level `harness` declares the agent harness bundle a repository installs (`ADR-89`, `MILE-198`;
`urzua harness`, `SPEC-19`; the request, verdict and adapter layout are `SPEC-24`). An absent key installs
and enables nothing: `urzua harness install` with no `--adapter` writes nothing and says so in a notice.

```yaml
harness:
  adapters: [claude-code]            # installed when --adapter is not given
  instructions_file: AGENTS.md       # carries the marked block; the engine assumes no file name
  outside_writes: shared             # allow | shared | deny; undeclared acts as allow
  guardrails:
    editor-hook:    {enabled: true, on_unjudged: allow}   # allow | deny
    session-start:  {enabled: true}
    post-command:   {enabled: false}
    acceptance:     {enabled: false, on_unjudged: deny}
    protected-paths: {enabled: false, on_unjudged: deny}
```

- **`adapters`** is a list of adapter names, each a directory of the bundle embedded in the binary
  (`urzua harness adapters` lists them). An adapter this build does not embed is refused at install.
- **`instructions_file`** is a repository-relative path. The install writes one block between
  `<!-- urzua-harness:begin -->` and `<!-- urzua-harness:end -->`, adds the markers once at the end of the
  file when absent, never touches the text outside them and, run twice, yields identical bytes. Without
  the key no block is written and the install says so.
- **`outside_writes`** decides how an edit-tool write outside the session's repository is judged
  (`ADR-89` amendment of 2026-11-09, outside writes). `allow`, and the key undeclared, leave it to the
  repository that holds the file, so a linked worktree at a commit with no gates is an unjudged place to
  write. `shared` also judges the file of a linked worktree of the session's repository (its git toplevel
  is not the session's own and shares the session's `git rev-parse --git-common-dir`; a worktree nested
  inside the session's directory is one, a plain subdirectory is not) by its path relative to that toplevel against the
  session's acceptance and protected-path gates (a request with no content is still judged by its
  path), and a deny wins; a separate clone shares no common
  directory and stays unjudged, an accepted risk. `deny` refuses an agent's write outside the session's
  repository while any guardrail is enabled, except under **`allow_paths`**, a list of absolute or
  `~`-relative directories that `doctor` reports as read by nothing under any other value. A value other
  than the three, or a relative `allow_paths` entry, is a load error. `urzua init` proposes `shared`;
  `urzua doctor` warns when a guardrail is enabled and the key is undeclared; `shared` is recommended.
- **`guardrails`** names which guardrails are on: a guardrail is on only when named with `enabled: true`.
  The names are `editor-hook`, `post-command`, `session-start`, `acceptance` and `protected-paths`. Only
  `editor-hook`, `acceptance` and `protected-paths` take `on_unjudged`: `allow` (the default for
  `editor-hook`; the edit proceeds with a loud message, `ADR-71`) or `deny` (the default for the other two,
  because an unjudged gate is not a gate; an agent's edit is refused while the guardrail cannot judge, a
  person's is allowed). A denying guardrail that cannot judge refuses an agent's edit to the protected
  paths too, including this file; a person repairs it by editing outside the agent harness (a plain editor
  or a terminal), where no hook runs. `urzua doctor` and the verdict's message say so.
- **A guardrail name the build does not ship is a load error.** A name under `harness.guardrails` that is
  not one of `editor-hook`, `post-command`, `session-start`, `acceptance`, `protected-paths` stops every
  command (`check` exits 2) with a message naming it and, when one is close, the nearest valid name
  (`acceptence`: did you mean `acceptance`?), because a misspelled gate leaves the one meant switched off
  while the file reads as protection (`ADR-89` amendment of 2026-11-07).
- **A key the build does not know, and a guardrail the chosen adapter cannot provide, are
  reported, never skipped.** The configuration still loads with an unknown `harness` key (or an unknown key
  inside a guardrail), so a typo does not stop every command; `urzua harness install` refuses (exit 2,
  nothing written), `urzua doctor` warns, and `urzua check` reports each one as a warning notice
  (`SPEC-2`; reported, not an error, so the exit code does not move). A guardrail turned on that no chosen adapter provides in this bundle version (`post-command`
  until its milestone lands) is installed around and reported by
  `install`, `status` and `doctor`. A value of the wrong type (`enabled: yes-please`, `on_unjudged:
  sometimes`, `adapters: claude-code`) is a load error.
- **`authority`** is the top-level table that says who may accept a record and who may change a protected
  path (`ADR-89` section 4, `MILE-200`); it has its own section below. `acceptance` or `protected-paths`
  enabled with no `authority` key, or with no gate of the kind it reads, is a load error (exit 2) naming
  the guardrail: a gate with nothing to enforce is not left to read as protection.
- The editor hook also reads `block_on` and `override_log` (above); the harness cores read them from this
  file, so the shell scripts that used to read them by line are no longer needed.

## Who may accept a record: `authority`

A top-level `authority` names decider groups, the host evidence that may name one, and the gates by type
and status and by path (`ADR-89` section 4, `MILE-200`). It is read in two places, and what each can do is
stated here and claimed nowhere beyond it.

```yaml
authority:
  deciders:
    maintainers: {logins: [beauwilliams]}   # the host's logins; compared without case
  evidence: [pr-author]                     # pr-author | pr-approver, any of; pr-author when omitted
  decider_table: base                       # base | head | both; head when omitted
  gates:
    - {type: adr, to: [Accepted, Rejected], by: maintainers}
    - {type: rfc, to: [Accepted, Rejected], by: maintainers}
    - {paths: [".claude", ".git-hooks", ".github/workflows", ".urzua/config.yaml"], by: maintainers}
```

- **`deciders`** maps a group name to `{logins: [...]}`. A group with no login, a bare login or a list of
  logins where a group belongs (a login in no group), and a key other than `logins` are load errors naming
  the group or login.
- **`evidence`** is the pull-request facts the host supplies, any one of which may name a decider:
  `pr-author` (the pull request's author) and `pr-approver` (a login whose approval is current for the head
  commit). The default is `pr-author`, never `pr-approver`: a host bars an author from approving their own
  pull request, so on a single-maintainer repository `pr-approver` could never be satisfied and no
  acceptance could merge. An empty list and an unknown kind are load errors.
- **`decider_table`** says which revision's table `diff.decider-required` reads (`ADR-89`, amendment of
  2026-11-09): `head` the checkout's, as before; `base` the configuration file at the `--base` merge base, so a
  pull request cannot add its author to a group, or remove a gate, in the change it asks to have approved;
  `both` judges each gated change under both tables, a gate in either applying. Undeclared is `head` (the
  rule is opt-in, no behaviour changes). The mode in force is the stricter of the head's and the base's
  declaration (`head` < `base` < `both`), so editing the key in a pull request does not loosen it. Under
  `base`, a base with no config file or no `authority` section holds no deciders and no gates; one whose
  config cannot be read is a warning notice and the head table is used. With no `--base` the head table is
  used and, under `base` or `both`, an `info` notice says so. `urzua init` proposes `base`, and `urzua
  doctor` warns (`authority-decider-table`) when gates are declared and the key is not. Any other value is a
  load error.
- **`gates`** is a list. `{type, to, by}` gates a move of a record of that type to one of those statuses;
  `{paths, by}` gates a change under those paths (a prefix or a glob, as in `diff.source-requires-record`).
  `by` names a declared group. A gate that names a type no record type declares, a status that type does not
  declare (or a type that declares no statuses), a group `deciders` does not declare, or that mixes the two
  forms, is a load error naming `authority.gates[N]`. A key that is present but empty is refused, not read
  as absent. A `paths` entry is read in the same canonical form as `published_paths` (a leading `./` and a
  trailing `/` dropped), so `./.agent` gates what `.agent` does; an entry that is absolute, has a `..`
  component or a backslash, or names no path (`.`), is a load error, since it could match nothing.
- **Locally, the edit tools are refused.** With the guardrail enabled under `harness.guardrails`
  (`acceptance` reads the gates by type and status, `protected-paths` the gates by path; an enabled
  guardrail with no gate of its kind is a load error), `urzua harness run edit` denies an `agent` actor's
  Write, Edit or MultiEdit that moves a record of a gated type to a gated status, compared with the copy on
  disk and, failing that, at `HEAD` (a record filed already at the status is a move; an edit that leaves
  the status where it was is not), or that writes a gated path. The deny is final: the override of
  `ADR-78` is not accepted for it, the output says so and nothing is written to the override log. A `human`
  actor is not affected, and neither is any route that does not go through the edit tools (a shell
  command is reported by the post-command check, not stopped). When the configuration does not load, the
  raw file still says what a denying guardrail protects: an agent's edit to the configuration, a gated
  path or a record in a gated type's directory is refused with the way out (a person edits outside the
  agent harness), unless the guardrail is set to `on_unjudged: allow`.
- **In CI, the evidence is authorship.** `diff.decider-required` (`SPEC-2`), run with `--base` and
  `--actors FILE`, requires that one of the declared `evidence` kinds names a login in the gate's group for
  every gated move (judged against the merge base's copy) and every changed gated path. It proves who
  opened and approved the pull request, not who decided: **with a single maintainer, and agents acting
  under the maintainer's token, it cannot tell an agent from the maintainer.** Its option
  `require_evidence` (`false` by default) makes a gated change with no actors file an error instead of
  `info`. Enabling the rule with no `authority` is a load error.
- **The actors file** is JSON the caller supplies and `check` never fetches: `{"schema": 1, "pr_author":
  LOGIN, "approvers": [LOGIN, ...]}`. The gate action writes it from the GitHub API, listing only the
  approvals given on the head commit, so an approval before a later push is not counted. A file that is
  unreadable, is not JSON or has another `schema` is a could-not-run (exit 2); `--actors` with the rule off,
  filtered out by `--rules`, or without `--base` is reported as a warning notice and not ignored; with
  `--stdin-path` it is refused.
- **Accepted risk.** The maintainer decided on 2026-11-05 to document and accept these routes past the
  guardrail: a Bash command that writes the status; editing the guardrail itself by shell or through a
  settings file the install does not read, and `git commit --no-verify`; a pull request that changes its
  own workflow; an agent acting as the maintainer. The required status check, branch protection and any
  `CODEOWNERS` file are the host's settings, which the engine can neither read nor verify: `urzua doctor`
  says so and does not claim them, and names a distinct agent identity with `CODEOWNERS` on the protected
  paths as a future option and not as configured.

## Editor hook blocking: `block_on`

A top-level `block_on` sets which errors the editor hook blocks on (`ADR-78`, amendment of 2026-11-05;
`MILE-195`), one of two values:

```yaml
block_on: introduced   # the default
```

- **`introduced`** blocks only on an unwaived error the edit introduces: one the record as it is on disk
  does not already report with the same rule and message. An error already on disk is listed, not blocked.
- **`all`** blocks on any unwaived error in the proposed content of the edited record, whether or not
  the record already had it. The hook then skips its second, on-disk `check` run, and lists nothing as
  already failing. The named override and the `override_log` work the same under both values.
- **Absent means `introduced`.** A configuration that does not declare the key behaves exactly as it did
  before the key existed, and `introduced` written out is byte-identical in the hook's output.
- **Any other value is a load error** (exit 2) naming the key and the value, never a silent default. The
  hook reads the key itself, from the top-level `block_on:` line, as it does `override_log`; a value it
  does not recognise gives the verdict `allow-unjudged` with a message naming the value (`SPEC-8`).
- The engine reads no other meaning from the key: `urzua check` and `check --stdin-path` report the same
  findings under both values, and the Claude Code adapter passes the hook's verdict through unchanged.
  This repository declares `introduced` (`ADR-78` amendment, item 3).

## Untracked records: `discovery`

A top-level `discovery` sets which files discovery reads beyond the ones git tracks (`ADR-92`, `MILE-206`):

```yaml
discovery:
  include_untracked: false   # the default
```

- **`include_untracked: true`** makes discovery include untracked, non-ignored files directly in a
  declared record directory (`dir` or any `extra_dirs` of a type) that the type owns, under the same
  filename rules the loader applies (`.md`, not an `exclude_files` entry, not underscore-prefixed). It is
  the choice of a repository that wants a record read before it is staged: a malformed draft is found
  before `git add`, and `files_examined` and `scope.untracked_included` count it. The price is that a
  scratch file in a record directory that looks like a record is governed: the author moves it, ignores
  it or finishes it. A file outside every declared directory is never included, and git's own rules
  (`.gitignore`, `.git/info/exclude`, the global excludes) decide what is ignored. This repository sets it.
- **`false`**, the default and what an absent key means, keeps the tracked-only corpus (`ADR-56`). A plain
  `check` then names the untracked records it skipped in an info notice (`SPEC-2`).
- Structured, not a string: the value is a boolean. An unknown key under `discovery`, or a value that is
  not a boolean, is a load error (exit 2), as for any other configuration object.
- It lives in committed configuration, not on the command line, so every checkout and CI agree on the
  corpus. Two checkouts of one commit with different untracked files still disagree on `files_examined`,
  as they already do for staged files.

## Numbering against open pull requests: `new`

A top-level `new` sets whether `urzua new` fetches pull-request head refs before numbering a record, so it skips
the numbers open pull requests already took (`ADR-93`, `MILE-89`; the mechanism is in `SPEC-12`):

```yaml
new:
  check_open_prs: false                    # the default; true makes new fetch and scan
  pull_ref_patterns: ["refs/pull/*/head"]  # the default, GitHub's layout
  remote: origin                           # the default
  fetch_timeout_seconds: 30                # the default
  lock_timeout_seconds: 10                 # the default
```

- **`check_open_prs: true`** is the first network step outside `refs refresh`, so it is opt-in: absent or
  `false`, `new` touches no remote and behaves as before. With it, `new` fetches, takes a lock and also reads
  the trees of the repository's other worktrees. A later command that wants the network needs its own opt-in.
- **`pull_ref_patterns`** is a non-empty list of ref patterns, each starting `refs/` and holding exactly one
  `*`, which stands for the pull request number. A repository on another host names its own (GitLab's is
  `refs/merge-requests/*/head`). Each pattern is fetched into its own namespace, `refs/urzua/pr/<segment>/*`.
- **`remote`** is a remote's name; a value that is empty, holds whitespace or starts with `-` is refused.
- **`fetch_timeout_seconds`** and **`lock_timeout_seconds`** are whole seconds, at least 1.
- **Load errors** (exit 2), never a silent default: an unknown key under `new`, a `check_open_prs` that is not a
  boolean, an empty `pull_ref_patterns` or an entry that is not a pattern as above, a `remote` as above, and a
  timeout of 0.
- **`identity.open-pr-collision`** (`SPEC-2`) is the opt-in `check` rule that reads the refs this fetches; it
  takes no option of its own and is a warning at its default level.

## Attestation identity: `attestation`

A top-level `attestation` sets where `urzua fix --apply` and `urzua fix --rebaseline --apply` take the
identity they attest under (`ADR-89` section 5, `MILE-201`):

```yaml
attestation:
  identity: gh-verified   # gh-verified | asserted; the default
  require_verified: false # the default
```

- **`gh-verified`** (the default, and what an absent key means) keeps the order `RFC-2` and `MILE-66` set: a
  verified `gh` login wins over `--by`, which wins over `git config user.name`. A `--by` the login replaced
  is named in the report (`identity.replaced_by`, and the `identity` notice) and never silently dropped.
  With `require_verified: true` a command that writes an attestation does not run when `gh` cannot verify
  (`not-run`, exit 2, nothing written): it does not fall back to `--by` or the git configuration.
- **`asserted`** takes `--by` as given, for a repository whose agents attest on behalf of a named person with
  no `gh` session. It reverses the verified-beats-typed order of `RFC-2` and `MILE-66`, so it is opt-in,
  never the default, and the row says `asserted`. The login `gh` verified, when it answers, is recorded
  beside the name (`identity.verified_login`, and in the row), not instead of it. With no `--by` the git
  configuration is the name (source `git-config`). `MILE-18` is the longer-term identity binding.
- **The source and the actor kind are recorded in every tool-authored revision row** (`SPEC-8`). The source
  is `gh-verified`, `asserted` or `git-config`; the actor kind is `agent`, `human` or `unspecified`, from
  `fix --actor agent|human`. The actor kind is the caller's claim, not proof, with the standing of `--by`: a
  verified login names the account whose token ran the command, not whether a person or an agent held it
  (`RFC-2`, `BUG-16`).
- **Load errors** (exit 2), never a silent default: an `identity` value other than `gh-verified` and
  `asserted` (the message names both), an unknown key under `attestation`, and `require_verified: true` with
  `identity: asserted`, which verifies nothing to require.
- **Absent means today's behaviour.** A configuration that does not declare the key resolves identity as
  before; only the row text and the report gain the source and the actor kind.

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

A glob that cannot compile (an unclosed `[`) is refused at load. `back_pointer_paths` stays a
prefix list: a `*` in it is a literal character. `published_paths` and a boundary's `paths` and
`exclude` take the same prefix-or-glob entries (see "Boundaries").

## Boundaries

`boundary.published-path-reference` reads **declared boundaries** (`RFC-11` sections 1 to 3,
`ADR-80`, `MILE-125`): paths that publish beyond the corpus, each with a kind, a reason and a
severity. A boundary is a declared fact, and the prohibitions follow from its kind.

```yaml
rules:
  boundary.published-path-reference:
    level: error                # enables the rule; each boundary's severity sets its findings' level
    ignore_marker: "record-id-ok"   # optional; declared, never compiled in
    boundaries:
      - paths: ["packages/*/src"]     # prefixes or globs (MILE-158)
        kind: published               # published | vendored | generated
        reason: "Published to the registry."   # required, non-blank
        severity: error               # error | warn
        exclude: ["packages/*/src/fixtures"]   # optional, the violation check only
    published_paths: ["pkg/sdk"]      # the compatible form, still supported
```

- **`paths`** (required, non-empty), **`kind`**, **`reason`** and **`severity`** are required. Any
  other `kind`, a blank `reason`, a `severity` other than `error`, `warn` or `info`, or a path that is
  absolute, contains `..` or a backslash, or is an invalid glob is refused at load.
- **A boundary's `severity` is the level of every finding it produces.** The rule's `level` only
  enables the rule: `level: warn` over a boundary declaring `error` reports errors, and
  `level: error` over a boundary declaring `warn` reports warnings. A rule set to `info` caps every
  boundary at `info`, and a boundary may declare `severity: info` under any rule level.
- **`published`:** a record identifier inside it is a violation. Any identifier carrying a declared
  type prefix counts, in prose as well as in a marker, resolvable or not.
- **`generated`:** a back-pointer inside it (an `Implements:` or `Verifies:` comment citing a
  record) is reported, because a human-authored marker there is overwritten. A plain record
  identifier is not reported.
- **`vendored`:** declared, with no prohibition. A file only a vendored boundary covers is counted
  eligible and unexamined.
- **`published_paths` is read as a published boundary** with no reason required and the rule's
  `level` as its severity. It is not deprecated; when both forms are declared both apply. The rule
  needs at least one boundary or one `published_paths` entry to load.
- **Violation and evidence are separate matchers.** The boundary check reads any record
  identifier; the evidence collector keeps its narrow `Implements:` / `Verifies:` reader and skips
  every file under a `published` boundary's `paths`, whether or not the file is also excluded. A
  marker inside a published path is therefore never evidence, and prose citing a record is never
  evidence anywhere. The skip applies only while the boundary rule is enabled.
- **Exceptions are declared, at the width of the thing excused.** A boundary's `exclude` entries
  (prefixes or globs) remove paths from that boundary's violation check only; the excluded files
  stay in the population as eligible and unexamined, not out of scope, so
  `config.scope-matches-nothing` does not read a deliberate exclusion as an unreachable boundary. The declared `ignore_marker` text excuses the single line it appears
  on and no other. The engine compiles in no marker. A back-pointer rule's marker (`embodiment.back-pointer-resolution`, `embodiment.back-pointer-decision-accepted`) is refused at load when it would sit on every back-pointer line: fewer than 3 non-whitespace characters, no letter or digit (`//`, `#`), part of a back-pointer keyword (`Implements:`, `Verifies:`), or part of a declared record id head (`ADR-` for a type with prefix `ADR`), or padding around those: a marker that leaves fewer than 3 letters or digits after the back-pointer keywords (`Implements:`, `Verifies:`), the declared id heads (`ADR-`, or the bare prefix, when they do not start a record id such as `ADR-1`), comment leaders and punctuation are removed (`// Implements:`, `# Verifies:`, `-- Implements: ADR`, `// ADR-`). `ADR-1`, `example-only` or `urzua:allow` load.
- **Nothing declared may match nothing silently.** Each of these is a finding at the boundary's
  severity: a `paths` entry covering no tracked file that is not excluded, an `exclude` entry
  matching no tracked file inside its own boundary, and an `ignore_marker` use on a line where
  it suppresses nothing. The uncovered `paths` entry carries an `edit-config` `set` action with no value, naming the entry, so a person picks the corrected path and coverage is never shrunk; the unused `exclude` entry carries an `edit-config` `remove` action naming it.

## The decision gate

`diff.source-requires-record` is opt-in, and by default asks only that some governed record changed
beside the source. Three further options (`MILE-133`) make it the decision gate: a change to
declared source paths must also change a decision at a decided status, a record linking one, or a
fix record. The gate is off unless `decision_types` is declared.

```yaml
rules:
  diff.source-requires-record:
    level: error
    source_paths: ["rust/crates"]
    decision_types:            # type -> the statuses at which one counts as decided
      adr: ["Accepted"]
      spec: ["Accepted", "Shipped"]
    fix_types: ["bug"]         # a record of one of these types exempts the change
    link_fields: ["Implements"]  # a changed record whose field names a decided decision
```

- `decision_types` maps a declared record type to a non-empty list of its statuses. The diff is
  released by a changed record of that type at one of them, judged by the record's own `Status`
  (the type's `Status` role field) in the working tree.
- `link_fields` names pointer or narrative fields. A changed record, of any type, whose field names a
  record of a `decision_types` type at a decided status releases the diff, so a milestone that
  `Implements` an Accepted ADR does. A reference that resolves to no record, or to a decision still at
  an undecided status, releases nothing.
- `fix_types` names record types whose changed records release the diff, so a mechanical fix lands
  with only a bug record (`AGENTS.md`).
- A changed decision still at an undecided status does not release the diff and is named in the
  finding. A deleted record, which has no status to read, satisfies nothing.
- With the gate declared, a changed record that is none of the three no longer releases the rule.
  The population and `exclude_paths` behave as without it.

A declaration that could never apply is refused at load (exit 2) rather than leaving the gate quietly
open: `fix_types` or `link_fields` without `decision_types`; a type that is not declared; a status
a type's `statuses.values` does not list; an empty list; a type named in both `decision_types` and
`fix_types`; a `link_fields` entry that is not a pointer or narrative field of any type; and any of
the three on a rule that does not take them.

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
- **One graph per pair, except `roadmap.pairs`.** `sequence.cycle` reads each `acyclic` pair on its own, so a loop that crosses two acyclic pairs is not detected, unless both pairs are named in `roadmap.pairs`: those are read as one graph and the loop is reported (below).
- **One finding per record on a cycle.** A large cycle yields one finding per member, by design, so a
  diff-scoped run that examines one member still sees it.
- **Overlap.** When a pair field is also one of the type's `pointer_fields`, `sequence.unresolved-blocker`
  skips what the pointer rules already report: an unresolved target when `pointer.resolution` is
  enabled, and a prose entry when `header.pointer-field-clean` is enabled. In a repository that follows
  `RFC-48` (`Depends-on` a pointer field, both rules on) it therefore reports nothing and acts as a
  guard for the roadmap: the graph is only computed when no claim was dropped.
- **Which pairs are dependencies: `roadmap.pairs`** (`ADR-83`). A pair can be `acyclic` without being a
  dependency: a containment pair (`Parent`/`Children`) is checked for cycles, but a child does not wait
  on its parent finishing. The top-level `roadmap: {pairs: [name, ...]}` names the `relation_pairs`
  entries that are dependencies, and the derived `blocked` value reads exactly those, so `check` and
  the roadmap command cannot disagree. Each name must be a declared pair that sets `acyclic: true`,
  listed once; anything else is a load error. `sequence.unresolved-blocker` reads every `acyclic` pair.
  `sequence.cycle` reads the pairs named here as one graph (`MILE-192`, `ADR-82`), so a loop that uses an
  edge of each of two of them is reported; an `acyclic` pair not named here, and every pair when
  `roadmap.pairs` is absent, is read on its own.
- **The derived `blocked` value** is read from these edges by `sequence.status-agreement` (see
  `statuses.progress` and `statuses.blocked`).
- **Not built here.** `sequence.phase-order`, `sequence.failed-dependency`,
  `sequence.terminal-parent-open-child` and the `condition` record type are `RFC-48`'s other parts and
  belong to other milestones. `urzua roadmap` is built (`MILE-182`, `ADR-82`) and reads the same
  `roadmap.pairs` as the derived `blocked` value, below.

### `roadmap` (`ADR-82`, `MILE-182`)

```yaml
roadmap:
  pairs: ["sequencing"]          # relation_pairs entries that are dependencies
  group_by: ["Phase", "Track"]   # header fields to group by, outermost first
  work_types: ["milestone", "bug"]  # record types that are work; absent means every type
```

`urzua roadmap` (output in `SPEC-2`) computes one graph over the pairs `pairs` names. A pair can be
`acyclic` without being a dependency: a containment pair (`Parent`/`Children`) is checked for cycles,
but a child does not wait on its parent finishing, so only the pairs named here order work. Both keys
are optional and default to empty, and so is `work_types`. Absent `pairs` means no roadmap: the command says so in a note and
`export --format json` carries no `roadmap` key.

- `pairs`: each name must be a declared `relation_pairs` entry that sets `acyclic: true`, listed once.
  `sequence.cycle` reads these pairs as one graph, the graph `urzua roadmap` reads, so `check` and the
  roadmap agree on a loop that crosses pairs (`SPEC-2`).
- `group_by`: each name must be a field some record type declares (`required_fields` or
  `known_fields`), listed once, with no surrounding whitespace. The engine names no grouping field.
  A field no open record carries is not an error; the roadmap notes it.
- `work_types` (`ADR-82` Amendment of 2026-11-05, `MILE-196`): the `record_types` keys whose records the
  roadmap orders. A record of any other type still blocks its dependents but is not in `waves`,
  `ready`, `critical_path`, `records` or `groups`, and the output gains a `waiting_on` list (`SPEC-2`).
  The engine names no type: a repository lists the types that are work, and leaves out any type that
  models a fact, a signal or an awaited decision (`SPEC-23`). Absent, every type is work and the output
  is unchanged. `sequence.cycle` and `sequence.status-agreement` never read it. With no `pairs` it is
  accepted and inert.

Load-time errors, reported as `roadmap: ...`: a `pairs` name that is not a declared pair, one that does
not declare `acyclic: true`, a repeated `pairs` or `group_by` entry, a blank or padded `group_by` name,
a `group_by` name no type declares, and a `work_types` entry that is not a `record_types` key, that
is listed twice, or that is blank or padded, or an empty `work_types` list (a declaration that nothing
is work orders nothing). A declaration that can never apply would read as a clean
roadmap, so it is refused (`ADR-7`).

Enabling `sequence.cycle` or `sequence.unresolved-blocker` while no pair declares `acyclic: true` is a load error, for the reason
the pair rules refuse to run with no pair: it would examine nothing.

Load-time errors: a blank or duplicated pair name, a blank or whitespace-padded field name, a field
declared by no type (the pair could never examine a record), a field used by two pairs or twice by one
pair, an unknown `reciprocity`, and enabling either pair rule while no pair is declared. A pair field
declared as a `pointer_fields` entry is also resolved by `pointer.resolution`, so an unresolved
reference reports twice; keep pair fields outside `pointer_fields`, as `Supersedes / Superseded-by` is.

### A record type declared from configuration alone: `condition` (`MILE-187`, `RFC-48`)

A record type needs no engine support when its shape is made of primitives this file already
documents. `condition`, the record for a fact nobody builds, is the first such type; its schema,
statuses and `Signal` section are specified by `SPEC-23`.

| Type | Spec | Directory and identity | Statuses | Required by status |
|---|---|---|---|---|
| `condition` | `SPEC-23` | `docs/conditions`, prefix `COND`, `COND-NNNN-slug.md` | `Open` (initial) moves to `Met` (success) or `Dropped` (failure), both terminal | `Met-by` in `Met` |

The declaration uses `dir`, `prefix`, `identity`, `statuses` (`values`, `success`, `failure`,
`initial`, `transitions`, `require`), `sections` and `spec`, and nothing else. No rule, option or
code path reads the name `condition`: the same declaration under another type name, prefix and
directory reports the same findings and the same roadmap (a fixture proves it). A `Depends-on` from
any record that declares the field resolves to a condition through the sequencing pair like any other
record, which needs `cross_type: true` on that pair as here. A type's effective prefix defaults to its
upper-cased name (`CONDITION`), so a type whose prefix differs from its name declares `prefix`.

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

## Supported schema versions

`schema_version` is the first key of every config and is required (`ADR-12`, `ADR-52`). A release
reads the current version `N` and the one before it, `N-1` (`ADR-77`, decided in `RFC-49`); a version
older than `N-1` or newer than `N` is outside the window. Each release is one row; its upgrade step is
the one from the row's older version to its newer.

| Release | Reads | Writes |
|---|---|---|
| `0.4.0` | 1, 2 | 2 |

`urzua check` and every other command that loads the config behave in one of three ways:

- **At `N`.** Read as written.
- **At `N-1`.** Upgraded in memory to `N` before any rule runs, so a rule only ever sees the current
  shape. `config.schema-version-outdated` (`SPEC-2`) reports one warning on the config file, naming the
  version found, the current version, the keys the upgrade dropped and the command that rewrites the
  file. Under `fail_on: warning` that fails a strict repository until the file is migrated. The rule is
  opt-in like every rule, and the upgrade enables it (the upgraded config and the file `migrate config`
  writes both name it at `warn`), so a config upgraded once is reported by the next release that moves
  the version again.
- **Outside the window.** The run does not start: exit 2, `status: not-run`, the usual `error`
  sentence, and a `schema_version_error` object:

  ```json
  {
    "found": 7,
    "supported": [1, 2],
    "current": 2,
    "migration": { "release": null, "command": null, "note": "this config is newer than this build; ..." }
  }
  ```

  `migration.release` is the oldest release that still reads `found` and `migration.command` the
  command to run with it (`urzua migrate config`); both are `null` when no release reads `found` (a
  version newer than `current`, or one no release ever read), and `note` says what to do either way.
  Nothing else in the file is read first, so a bad key in a config of the wrong version is not reported
  in place of the version.

`urzua migrate config` (`SPEC-14`) rewrites an `N-1` config to `N` in place, keeping its comments, and
lists every key it added, changed, renamed, moved or dropped. A second run reports `nothing-to-do`.
It never reads a version outside the window, so there is no chain of steps: an adopter who skips a
release installs the older release that reads their version, migrates, and moves on.

### Version 1

Version 1 is the shape `.urzua/config.yaml` had from its introduction (`ADR-52`) until `MILE-80` made
every rule opt-in. It was current on `main` for a day and no release read it as YAML: the `0.3.0`
release read `.urzua/config.toml` at `schema_version: 1`, a format nothing reads any more (`ADR-54`). The
upgrade to version 2 does exactly what that break did:

- **No `rules` table.** A version-1 build ran every rule, each at a severity the rule chose, and a
  version-1 config with a `rules` key is refused as not a version-1 config. The upgrade names every
  rule the version-1 build ran, at the severity it ran, so the config keeps running what it ran:
  `header.required-fields`, `config.pointer-declaration-missing`, `config.pointer-field-not-known`,
  `config.pointer-narrative-overlap`, `pointer.resolution`, `filename.title-consistency`,
  `revision-log.change-class-required` and `relation.supersession-reciprocity` at `error`;
  `header.field-set-consistency`, `type.no-declared-spec`, `header.pointer-field-clean`,
  `embodiment.consistency` and `embodiment.locator-promotion-candidate` at `warn`; and `field.quality`
  at `error` with `field.pending` at `warn`, the two halves of one version-1 rule.
- **Dropped, and reported as dropped.** The per-type `header_layout` key and the rule that read it
  (`header.layout-consistency`); `header.deprecated-shape`, whose header shapes no release reads;
  `narrative-field.stale`, which version 2 refuses without a `terminal_statuses` list a version-1
  config had no place to declare; and `pointer.target-status`, which version 1 ran by announcing the
  status of every resolved reference and which version 2 runs only for statuses a repository declares.
- **Not converted.** A version-1 config that names a removed `header_shape` (`blockquote`,
  `bold-list`) is not made valid by the upgrade: those shapes are no longer read (`MILE-85`), so the
  upgraded config fails to load with that error and `migrate config` leaves the file as it was.
- **Refused.** A version-1 record type that omits `header_shape` meant `blockquote`, and the default
  is now `yaml-frontmatter`. The upgrade does not carry that across silently, and it cannot write
  `blockquote`, which no release reads (`ADR-54`: a break is a break). It refuses, in memory and in
  `migrate config`, with an error naming each such type (`record_types.<name>`), the removed shape and
  the repair: rewrite the type's records as YAML frontmatter and declare `header_shape:
  yaml-frontmatter` (or `none`) on the type. The file is left as it was.

### When a version bump is required

`schema_version` changes only for a break: a removed or renamed key, a changed meaning for an existing
key, or a default that changes behaviour for an unchanged config. An additive optional key whose
default preserves behaviour is not a break and needs no bump. A reviewer applies this to a config
change; a change that is a break and does not bump the version is a defect in the change.

A bump to `N+1` is one change that: raises `CURRENT_SCHEMA_VERSION`; replaces the upgrade step in
`urzua-core::schema_version` with the one from `N` to `N+1` (the previous step is deleted, not kept);
adds a row to the table above and to `RELEASES`; enables `config.schema-version-outdated` in the new
step; and adds a fixture config at each supported version to the acceptance corpora, with a case
proving each loads to the same effective config.

## Open questions

- **Can scoped overrides express per-package variation without becoming a second config format?**
  The moment overrides gain their own resolution order, this is per-package configs with extra
  steps.
- **Does the config need a version field?** Answered: it has one, `schema_version`, and a window of
  versions a release reads (see "Supported schema versions").
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

A level is `off`, `info`, `warn` or `error`, written bare or as a table when the rule takes options. The
declared level **replaces** whatever severity the rule body chose: one rule, one level.

**`info`** (`ADR-87`) is an opt-in reporting level for an advisory rule: the rule runs, and its findings
carry `severity: "info"`, with rule, file, line, population and `suggested_action` exactly as at
`warn`. They are listed and counted in `findings` and `status`, and they never set `blocking` and never
fail `fail_on`, under either value. A waiver covers one like any other finding. A rule set to `info`
reports nothing above `info`: a `field_levels` entry or a boundary `severity` declared `warn` or
`error` does not lift it. Any other level string is refused at load, and the message lists the four
(`"off"`, `"info"`, `"warn"`, `"error"`); no value is ever read as `error` by default.

Two things are load-time errors rather than silent no-ops, because a check that never runs is
indistinguishable in the report from one that ran clean:

- a rule name this build does not ship -- the error names the valid set
- an option on a rule that does not take it

### Severity per field (`MILE-140`)

`field.quality`, `field.pending` and `status.required-field-missing` report one finding per field.
Each takes an optional `field_levels` map from a field name to `info`, `warn` or `error`. Without it, the
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
- A level is `info`, `warn` or `error`. `off` is refused (a field is switched off by not declaring it), as
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
  `Realized-by` line is older than the limit, whether or not the evidence changed. Age is counted in
  whole 24-hour days from the commit's timestamp to the current time, the same in every time zone;
  every date this engine compares, a suppression's `Expires` included, is a UTC date. It reports in
  addition to any disagreement, and a record whose line git cannot date is not warned.
- A locator written `code:path/to/file.rs@sha256:<64 lowercase hex>` is pinned to that content hash
  (`sha256sum path/to/file.rs`). A pinned locator drifts exactly when the file's current hash
  differs, and git history is not consulted for it, so an unrelated commit never reports and a
  change made after the record was written does. Any other suffix is not a pin and stays part of the
  path. Re-pinning is `urzua fix --rebaseline` (`SPEC-8`), or a hand edit of the hash on the `Realized-by` line.
  A locator may also carry a scope and be pinned to one item (below).

Both options belong to `embodiment.consistency` alone; on any other rule they are the usual
option-on-a-rule-that-does-not-take-it load error.

### Claim expiry: `embodiment.claim-expired` (`ADR-97`, `MILE-129`)

`embodiment.claim-expired` expires a claim after a period declared per record type, status and
evidence tier, at the level the repository declares. It is opt-in: a configuration that does not
declare it runs no new rule, reports nothing new and loads and reports as before, byte for byte.
`claim_max_age_days` (above) is unchanged and stays supported; declaring both is a load error that
names both, since they would report the same claim twice.

```yaml
rules:
  embodiment.claim-expired:
    level: warn               # info | warn | error; the ceiling for every entry below
    anchor_field: Last-audited
    require_pin: [code, test] # evidence categories whose locators must be pinned to be renewed
    drift: {code: block, test: block, spec: acknowledge} # what drift does to a renewal, per category
    max_batch: 10             # most records one `reverify --apply` may name
    periods:
      - {type: milestone, status: [Done],              days: 45, spread_days: 45}
      - {type: bug,       status: [Fixed],             days: 60, spread_days: 60}
      - {type: spec,      status: [Shipped, Accepted], days: 60, spread_days: 60}
      - {type: adr,       status: [Accepted],          days: 90, spread_days: 90}
      # a narrower entry wins: spec-tier evidence only, a longer period, advisory
      - {type: adr, status: [Accepted], tier: Specified, days: 180, level: info}
```

- **`periods`** is a list of maps (`ADR-64`, no string mini-language) and is required. An entry has
  `type` (a declared record type that declares its evidence locator field), optional `status` (a list
  of that type's `statuses.values`), optional `tier` (a state of the `embodiment` declaration, the
  *computed* one: what the record's locators compute to, not the stated `Embodiment`, so a milestone
  or bug that states none is judged), `days` (whole days, 1 or more), optional `spread_days` (whole
  days, 0 or more, default 0) and optional `level`.
- **The most specific entry wins.** `type`, `status` and `tier` outrank `type` and `status`, which
  outrank `type` alone. Two entries of equal specificity that can match one record are a load error.
  A record of a listed type that no entry matches is eligible and not examined.
- **`level` per entry** is `info`, `warn` or `error`, defaults to the rule's and is held to it: the
  rule's level is a ceiling, an entry cannot raise a finding above it (`ADR-87`). `off` is refused.
- **`anchor_field`** names a field every listed type declares (`required_fields` or `known_fields`).
  It is optional; without it a claim is dated by its `Realized-by` line only and `urzua reverify`
  cannot renew it.
- **`require_pin`** lists evidence categories as the `embodiment` declaration names them (`code`,
  `test`, `spec` by default); an unknown one is a load error. It defaults to empty. `max_batch` is a
  whole number, 1 or more, defaulting to 1. Both are read by `urzua reverify` (`SPEC-8`).
- **`drift`** (`ADR-97`, amendment of 2026-11-09) is a map from an evidence category, as the
  `embodiment` declaration names it (`code`, `test`, `spec` by default), to what drift of a locator of
  that category since the claim's `Realized-by` line was last touched does to a renewal by `urzua
  reverify`: `block` refuses the renewal until the locator is re-pinned (a pinned locator by `fix
  --rebaseline`, an unpinned one in a `require_pin` category by `reverify --pin-only`); `acknowledge`
  lets `reverify --apply` renew, and the `Re-verified` row names each drifted locator of the category
  (a cited record by its id and its newest revision-log date, any other file by its path, the
  prefix of the hash read and the commit that last changed it) together with the reviewer's
  `--reason`, and re-pins a drifted pinned locator of the category in the same act, recording old ->
  new hash; `ignore` leaves the category's
  drift out of renewal, unlisted. A category not named is `block`, so a configuration without `drift`
  behaves as before. An unknown category (the load error lists the declared ones), an unknown policy
  (it lists `block`, `acknowledge`, `ignore`) and the option on any other rule are load errors. It
  changes only what `reverify` refuses and writes: `embodiment.consistency` reports the same drift
  either way.
- **Nothing is dropped silently.** An unknown type, status or tier, `days` of 0, an unknown key, an
  `anchor_field` a listed type does not declare, a type with no locator field, an empty `periods`
  and every option on any other rule are load errors. A `periods` entry whose type holds claims
  (a record with a locator) and covers none of them is reported by `config.scope-matches-nothing`; an
  entry for a type with no claim yet is not, as an empty corpus reports nothing for any rule.
- **Which records are judged.** A record of a listed type that has the type's evidence locator field
  and whose status matches an entry. A record whose status is in the type's `claim_retained`, or that
  states a `sticky_states` value of `embodiment.consistency`, is not. Unlike that rule the record
  need not state `Embodiment`: a `Done` milestone's claim is its `Status`.
- **Age and the anchor.** Age is whole UTC days from the claim's anchor to today. The anchor is the
  date in `anchor_field` (`YYYY-MM-DD`, written only by `urzua reverify`), otherwise the date of the
  commit that last touched the record's `Realized-by` line (`claim_max_age_days`' measure; a line
  not yet committed is touched today). A malformed date, or one in the future, is reported
  (`claim-anchor-invalid`) and falls back to the line. A claim expires when its age exceeds `days`
  plus its offset.
- **`spread_days`, the first expiry.** A record with no valid `anchor_field` date has its period
  lengthened by a deterministic offset of 0 to `spread_days` days: the first 8 bytes of the SHA-256 of
  its `Stable-Id` text as a big-endian number, modulo `spread_days + 1`. Nothing is written to the
  record and no run date is stored, so every run and every machine agrees. The offset ends when the
  record carries a valid date (after its first `reverify`), delays and never hastens, and is named in
  the finding; a record with no `Stable-Id` gets 0 and the message says so. Absent `spread_days` gives
  no offset.
- **Drift and expiry are independent.** Both report on one record, `drift_grace_days` does not delay
  expiry, and neither suppresses the other. A waiver suppresses the finding and does not renew the
  claim; a waiver with no `Expires` is governed by the `permanent` declaration (`ADR-86`), which
  closes permanence for a rule it does not list.
- **`embodiment.claim-renewal-unattested`** is a second opt-in rule, declared beside it and needing
  its `anchor_field`. On `check --base` it reports a changed record whose anchor date is newer than
  its merge-base copy's and which gained no `Re-verified` revision row dated that day, or whose
  anchor the merge-base copy carried and the branch removed. It is not-applicable without `--base`,
  and cannot see a record that never carried the field or one new in the diff.

### Scoped evidence pins and locator resolvers (`ADR-74`, `MILE-185`)

A `Realized-by` locator may carry a scope between its path and its pin, so a pin hashes one item
and not the whole file. A repository that writes no scope and declares no resolver sees no change,
and a file-level pin keeps the meaning above.

**Grammar and parse order.** After the `spec:`, `code:` or `test:` prefix:

```text
locator := path [ "#" scope ] [ "@" anchor ]
scope   := range | symbol
range   := "L" N [ "-L" M ]      N and M decimal, no leading zero, 1 <= N <= M
symbol  := seg ( "::" ( seg | "<" seg ">" ) )*     seg := [A-Za-z_] [A-Za-z0-9_.-]*
anchor  := "sha256:" 64 lowercase hex  |  7 to 64 lowercase hex (a commit)
```

The parser takes, in order: (1) a trailing `@sha256:` and exactly 64 lowercase hex; (2) otherwise a
trailing `@` and 7 to 64 lowercase hex, the reserved commit anchor; (3) in what remains, the text
after the last `#` as the scope only if it matches `scope`, else the whole text is the path. A
scope that begins `L` and a digit is never a symbol: it is a range or malformed (`L10` is a range;
`L10x`, `L0`, `L010`, `L10-40` and `L40-L10` are malformed). `<Trait>` is never the first segment. A
locator with both anchors, a `@sha256:` that is not a pin, or a glob with a scope is malformed. A
path that contains `#` and fails the grammar is reported as malformed, never as a path missing from
the tree; there is no escape, so a file whose name ends `#word` cannot be cited. A commit anchor
parses and is reported `commit anchor not yet supported` (it is reserved for `MILE-84`).

**The hashed bytes.** A scope resolves to a contiguous span of whole lines. The pinned bytes are
those lines, each with at most one trailing `\r` removed, joined by `\n`, with no trailing newline,
read as UTF-8 with a leading U+FEFF dropped. Lines are split on `\n`; a final newline does not
start a line. A file that is not valid UTF-8 cannot carry a scoped pin. Nothing else is normalised:
a whitespace or comment edit inside the item is drift. The pin is the lowercase hex SHA-256 of those
bytes: `urzua explain <path>#<scope>` prints it, and for a span it reproduces with
`sed -n '<start>,<end>p' <path> | tr -d '\r' | sed -z 's/\n$//' | sha256sum` on a file without a BOM
or a bare `\r`. A file-level pin is unchanged and still hashes the raw bytes of the file.

**Resolvers.** `locator_resolvers` is a top-level list. The engine ships no language knowledge; a
symbol scope needs a resolver covering the file, and a line range needs none.

```yaml
locator_resolvers:
  - name: rust                  # required, unique
    glob: "**/*.rs"             # the files it covers; one resolver per file
    start: '^\s*(pub\s+)?(fn|struct)\s+{symbol}\b'   # first line of a plain symbol
    end: brace_balanced
    leading: '^\s*(#\[|///)'    # optional: contiguous lines above that are part of the item
    container:                  # optional: `Type::member` is found inside these spans
      start: '^\s*(impl(<.*?>)?\s+|(pub\s+)?struct\s+){symbol}(<[^{]*>)?\s*(where\b.*|\{.*)?$'
      end: brace_balanced       # brace_balanced, indent or heading
      spans: all                # `all`: every match is a candidate span; default `one`: exactly one
    member:                     # the item inside a container span, with its own start and end kind;
                                # one mapping, or a list when members end differently
      - start: '^\s*(pub\s+)?fn\s+{symbol}\b'
        end: brace_balanced
      - start: '^\s*(pub\s+)?{symbol}\s*:([^:]|$)'   # a struct field, up to the next field
        end: indent
    trait_container:            # `Type::<Trait>` and `Type::<Trait>::member`
      start: '^\s*impl(<.*?>)?\s+([A-Za-z_][A-Za-z0-9_]*::)*{trait}(<[^{]*>)?\s+for\s+{symbol}\b'
      end: brace_balanced
      spans: all
```

Patterns are regular expressions matched against one line. `{symbol}` (and `{trait}`) are replaced by
the regular-expression-escaped text from the record, so a `.` in a symbol is a literal dot. A
`start`, `container.start` and every `member.start` must contain `{symbol}`, a `trait_container.start`
also `{trait}`; a malformed pattern or glob, a missing placeholder, a `member` (any entry of a list) with no `container`
or `trait_container` or the reverse for `container`, a `container` end kind other than
`brace_balanced`, `indent` or `heading`, an unknown key, a repeated name and two resolvers with the
same glob are config-load errors. A file covered by two different globs is reported where a scope
in it is resolved, never resolved by the first.

**End kinds (normative, append-only).** An end kind finds the last line of an item from its first
line. A shipped kind never changes; a fix ships under a new name.

| Kind | The last line is | Blind to |
|---|---|---|
| `brace_balanced` | the first line at which the count of `{` minus `}`, counted over every character from the first line, returns to zero after a `{` has been seen | braces in strings and comments |
| `indent` | the last non-blank line before the next non-blank line indented no deeper than the first (leading whitespace characters counted; the end of file also ends it) | tabs, odd blank lines |
| `heading` | the last non-blank line before the next Markdown heading of the same or a higher level; the first line must be a heading (`#` to `######` and a space) | `#` lines in fenced code |
| `blank_line` | the line before the next blank (whitespace-only) line, or the last line | |
| `semicolon` | the first line, counting braces from the item's first line, at which the brace count is zero and the line ends in `;` (after trailing whitespace) | |

A resolver has one end kind for its plain `start`, so it should list only keywords whose items end
the same way. An item that ends at a `;` rather than a brace (`const`, `static`, `type`, `mod x;`
in Rust) is kept out of a `brace_balanced` start: under it the span would run on into the next
item, so it is left unaddressable by symbol (a not-found failure) and is pinned by line range, or
as a `semicolon`-ended `member` of its container (vector 18).

**Resolution.** A plain `Name` is the lines whose text matches `start`, anywhere in the file:
none is not found, more than one is ambiguous and reported with the line of each candidate, exactly
one is the item (extended upward over `leading`). `Type::member` finds the container spans of
`Type` (none: container not found; several without `spans: all`: ambiguous), then the lines
matching a `member.start` (every entry of a `member` list is tried) that are direct children of any
of those spans, counted together across spans and across entries: none is not found, more than one
is ambiguous (a line two entries both match counts twice, so is ambiguous), one is the item, ended
by the end kind of the entry that matched it. A `member` list is how one resolver addresses items
that end differently, such as a Rust method (`brace_balanced`) and a struct field (`indent`, up to
the next field or the closing brace: the closing `>,` of a field whose type spans lines is level
with the field and is not part of the span). The container may match a `struct` as well as an
`impl`, so `Type::field` is found in the struct's span and `Type::method` in the impls'; a name used
by both is ambiguous. A direct child of a `brace_balanced`
container is a line that starts at brace depth one; of an `indent` container, a line at the
indentation of the container's first non-blank body line; of a `heading` container, a heading one
level deeper. A match deeper inside another member (a nested `fn`) is not a candidate. A member's
end is searched only within its container span. `Type::<Trait>::member` is the same over the
`trait_container` spans; `Type::<Trait>` alone is the whole impl, which must be unique. A member of
a member (`A::b::c`) and a form the resolver declares nothing for are reported unsupported. Types
of the same name in different inline modules are one namespace in the first build.

**Reasons.** `embodiment.locator-exists` reports, once per locator, with the locator and the cause:
the path is missing or untracked; the locator does not parse (naming the grammar); the file is not
valid UTF-8; no resolver covers the file, or several do; the symbol, container or member is not
found; it is ambiguous (with the candidate lines); the form is unsupported for that resolver; the
end is not found; the range lies beyond the end of the file; a commit anchor is not yet supported;
and a scope given without a pin (drift is then judged on the whole file). A locator reported this
way is not also judged for drift.

**Drift.** A scoped pin drifts when the hash of its resolved span differs from the pin. The cited
file's last commit says nothing about the scope, so there is no file-based grace window for it: a
mismatch is drift at once, unless the record declares an audit date (`drift_audit_field`), in which
case the window counts from that date as for a file-level pin. A scope without a pin has no effect
on drift, which is judged on the whole file, and is reported as above. When such a scope also does
not resolve, the two points of `ADR-74` hold together: `embodiment.locator-exists` reports the
unresolved scope (point 6) and drift is still judged on the whole file (point 8), because with no
pin there is no span hash to compare. A commit anchor is reported as unsupported and is not
judged for drift. `embodiment.locator-promotion-candidate`
and `urzua explain` key on the path. `urzua fix --rebaseline` re-pins a scoped pin and refuses one
that does not resolve (`SPEC-8`), and `urzua explain <path>#<scope>` prints the span and the pin to
write (`SPEC-13`).

**Test vectors (normative).** Each row is a resolver, a text and a scope, with the span (1-based,
inclusive) or the failure. Rows 1 to 12 use the `rust` resolver above with the `leading` pattern;
`T` is the text shown, lines numbered from 1.

| # | Text `T` (lines) | Scope | Result |
|---|---|---|---|
| 1 | `/// d` / `#[derive(X)]` / `struct A {` / `  a: u8,` / `}` | `A` | 1-5 (leading doc and attribute included) |
| 2 | `fn f() {` / `}` / `fn f() {` / `}` | `f` | ambiguous: lines 1, 3 |
| 3 | `fn f() {}` | `g` | not found |
| 4 | `fn o() {` / `  if x {` / `}` | `o` | the end is not found |
| 5 | `fn s() {` / `  let a = "}";` / `}` / `  more();` / `}` | `s` | 1-2 (the early end is not detected) |
| 6 | `impl A {` / `  fn f(&self) {}` / `}` / `impl<T> A {` / `  fn g(&self) {}` / `}` | `A::g` | 5-5 (found in the second block) |
| 7 | `impl A {` / `  fn f(&self) {}` / `}` / `impl A {` / `  fn f(&self) {}` / `}` | `A::f` | ambiguous: lines 2, 5 |
| 8 | `impl A {` / `  fn f(&self) {` / `    fn n() {}` / `  }` / `}` | `A::n` | not found (not a direct child) |
| 9 | `impl fmt::Display for A {` / `  fn fmt(&self) {}` / `}` | `A::<Display>::fmt` | 2-2 |
| 10 | `impl fmt::Display for A {` / `  fn fmt(&self) {}` / `}` | `A::<Display>` | 1-3 |
| 11 | `impl From<u8> for A {` / `  fn from(a: u8) {}` / `}` / `impl From<u16> for A {` / `  fn from(a: u16) {}` / `}` | `A::<From>::from` | ambiguous: lines 2, 5 |
| 12 | `impl A {` / `  fn f(&self) {}` / `}` | `A::f::g` | unsupported (member of a member) |
| 13 | `a` / `b` / `c` | `L2-L3` (no resolver) | 2-3 |
| 14 | `a` / `b` / `c` | `L2-L4` (no resolver) | the range lies beyond the end (3 lines) |
| 15 | `a:` / `  x: 1` / `` / `  y:` / `    z: 2` / `` / `b:` (`indent`, start `^\s*{symbol}:`) | `a` | 1-5 (trailing blank excluded) |
| 16 | `# T` / `## One` / `text` / `### Sub` / `more` / `## Two` (`heading`, start `^#+\s+{symbol}$`) | `One` | 2-5 |
| 17 | `k1:` / `  v` / `` / `k2:` (`blank_line`) | `k1` | 1-2 |
| 18 | `impl A {` / `  const X: Foo = Foo {` / `    a: 1;` / `  };` / `  const Y: u8 = 3;` / `}` (member `end: semicolon`, start `^\s*const\s+{symbol}\b`) | `A::X` | 2-4 |
| 21 | `struct A {` / `    /// d` / `    a: u8,` / `    b: u8,` / `}` (the `rust` resolver, its container also matching `struct`, and a second `member` entry `^\s*{symbol}\s*:([^:]|$)` with `end: indent`) | `A::a` | 2-3 (doc line included, `b` excluded) |
| 22 | as 21, with `impl A {` / `    fn a(&self) {}` / `}` appended | `A::a` | ambiguous: lines 3, 7 (a field and a method) |
| 23 | as 21 | `A::z` | not found |
| 24 | `struct A {` / `    b: Vec<` / `        u8,` / `    >,` / `    c: u8,` / `}` (resolver of 21) | `A::b` | 2-3 (the closing `>,` is level with the field and is not included) |
| 19 | the bytes `a\r\nb\r\r\nc\n`, lines 1-3 | | pinned bytes `a\nb\r\nc` |
| 20 | the bytes `\uFEFFfn a() {\n}\n`, lines 1-2 | | pinned bytes `fn a() {\n}` |

### Locator kinds: files, directories and globs (`BUG-176`)

Which kinds of path a `Realized-by` locator may name, and which of those may carry a `@sha256:`
pin, are options of `embodiment.locator-exists`. Maintainer decision, 2026-11-05: this is
repository configuration, not engine policy.

```yaml
rules:
  embodiment.locator-exists:
    level: error
    locator_kinds: [file, directory, glob]   # default [file]
    pinnable: [file, directory]              # default [file]; each kind must also be in locator_kinds
```

- `locator_kinds` (optional, non-empty when given): `file`, `directory` and `glob`. A **file** is a
  git-tracked path present on disk, as always. A **glob** is a path holding `*`, `?` or `[`, matched
  with the diff rules' matcher (`Diff rule path options`: `*` within one component, `**` across
  components, `?` one character, `[abc]` a class); it covers the tracked files it matches, and a
  glob that names a directory covers the files below it. A **directory** is any other path with a
  tracked file below it, compared by path component. A path that is itself a tracked file is a file
  whatever characters its name holds. The default `[file]` is the behaviour before this option.
- A directory or glob locator is legal when it covers at least one tracked file that is present on
  disk: an untracked directory, a path with no tracked file below it, and a glob matching nothing
  are reported as missing, and a staged deletion of every covered file leaves nothing covered.
- A locator of a kind that `locator_kinds` does not declare is reported (`locator-kind-undeclared`),
  naming the option. A pin (`@sha256:`) on a directory or glob whose kind `pinnable` does not
  declare is reported (`locator-kind-unpinnable`), naming that option. A scope (`#...`) applies to
  one file and is reported unresolved on a directory; on a glob it is a malformed locator.
- A `pinnable` kind not in `locator_kinds`, an empty list, an unknown kind, and either option on any
  other rule are load errors: each would be a silent no-op or a locator that is always reported.

**Drift.** An unpinned directory or glob drifts on the newest commit among the files it covers now:
the record is behind when that commit is strictly after the commit that last touched the
`Realized-by` line, exactly as for one file, including `drift_grace_days`. A pinned one drifts when
its pin differs from the current one, with no file-based grace window (as for a scoped pin; only a
declared audit date holds it back).

**The pin.** The pin of a directory or glob is the lowercase hex SHA-256 of one line per covered
file, sorted by path, each `<path>` then a tab then the lowercase hex SHA-256 of the file's raw
bytes then `\n`. It moves when a covered file's content changes, a file is added, removed or moved
into or out of the set, and not when any other file changes. `urzua explain <directory-or-glob>`
prints `coverage`: the kind, the covered files, the hash and, when `pinnable` allows the kind, the
`pin` to paste; `urzua fix --rebaseline` lists the covered files of a stale pin and re-pins it
(`SPEC-8`, `SPEC-13`). A kind that is not declared in `locator_kinds` is not judged here: `explain`
says so in a notice, and drift reads the path's git history as it did before the option.

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

`test_paths` belongs to `embodiment.back-pointer-resolution` and to `embodiment.locator-on-back-pointer-path`
(below); on any other rule it is the usual option-on-a-rule-that-does-not-take-it load error.

### Preferring the lighter mechanism: `embodiment.locator-on-back-pointer-path` (`ADR-78` item 4, `MILE-189`)

An opt-in warning for a `Realized-by` locator on a file the back-pointer scan already reads, where an
`Implements:` or `Verifies:` comment would say the same thing. It declares its own paths, as
`embodiment.back-pointer-decision-accepted` does, so it does not depend on the other rule being on.

```yaml
rules:
  embodiment.locator-on-back-pointer-path:
    level: warn
    back_pointer_paths: ["src", "tests"]
    test_paths: ["tests"]      # optional, each entry inside a back_pointer_paths entry
```

- It flags a `code:` locator on a file under `back_pointer_paths` (an `Implements:` comment is code tier
  anywhere) and a `test:` locator on a file under `test_paths` (a `Verifies:` comment counts only there).
- It does not flag what a comment cannot say: a `spec:` locator, a `test:` locator outside `test_paths`,
  a locator with a `#scope`, or a locator with an `@sha256` pin. `Realized-by` is the only option for
  those, and for any path the comment scan cannot reach, which is why this is a nudge and not an error.
- A record whose type declares `statuses.delivery.back_pointer_evidence: false` is not flagged: a source
  comment is not evidence for that type (`ADR-84`), so the locator is the only evidence it has, and advice to
  drop it would leave the record unevidenced.
- When the file already carries the comment (an `Implements:` or `Verifies:` line citing this record), the
  message says to drop the locator, not to add the comment.
- It offers no edit action. Removing the locator before the comment exists would delete the evidence
  and leave `Embodiment` disagreeing with what is computed, and the other half of the repair is a comment in a
  source file, which no action kind addresses (`ADR-73`: an exemption with a reason, as for
  `embodiment.back-pointer-*`). The author adds the `Implements:` or `Verifies:` comment first, then
  removes the locator from `Realized-by`.
- `back_pointer_paths` is required (a load error without it); `test_paths` is optional.
- `scope: changed` is optional (`ADR-90`, `MILE-203`; see "Judging only the changed records" below).

### Judging only the changed records: `scope: changed` (`ADR-90`, `MILE-203`)

A nudge says "while you are here, do this", so it is useful only at the moment someone touches the
record. A rule that takes the `scope` option can be told to judge only the records a diff adds or changes.

```yaml
rules:
  embodiment.locator-on-back-pointer-path:
    level: info
    back_pointer_paths: ["src"]
    scope: changed
```

- `scope` is a rule option with one value, `changed`. There is no `scope: all`; omitting the option is
  that. Any other value (including a list, a number or a boolean) is a load error that names `changed`.
  A key with no value is refused like every other option.
- With the option, a rule judges (reaches a verdict on, and may report a finding for) only the records in
  the diff's changed set: added, modified or the new name of a rename, working-tree edits included. Its
  candidates are all still eligible. The others are counted in the population's `deferred_by_scope`
  (`SPEC-2`), and the report carries one `info` notice for the rule saying how many slots were not judged
  and why (`no --base`, or `unchanged since <base>`).
- On a plain `check` there is no diff, so such a rule judges none of its candidates: it is `ran`, with
  `examined` 0. `check --stdin-path` treats the proposed record as the one changed record, so the editor
  hook is where the nudge fires without `--base`. A path argument narrows further, as for every rule.
- A dependent is not changed. The diff scope's dependents (a record that cites a changed file, or shares
  an id with a changed record) are still examined by the rules that do not declare the option; for a rule
  that does, only the records the diff itself touches are judged. A change to the configuration file does
  not make a record changed.
- Which rules take it is a column of the rule table (`urzua rules` prints it as `judges`). A rule whose
  verdict on a record depends on that record alone (`one-record`) lists `scope`; today that is
  `embodiment.locator-on-back-pointer-path`. So does an `anchor-scoped` rule (`ADR-90` amendment,
  2026-11-07): it reads other records as context but reports only advisory findings anchored on a changed
  record, one finding per changed anchor; today that is `embodiment.locator-promotion-candidate`, which
  emits a finding on each changed citer of a shared locator. A cross-record rule (id collision, reference resolution,
  reciprocity, sequencing, which can leave a defect in an unchanged record that a change caused), a rule
  that already judges only the changed set (`status.transition-allowed`,
  `revision-log.version-bump-required`) and a rule that is not record-level (configuration, files, diff,
  run, directory-anchored) do not take it: declaring it is the usual "rule 'x' does not take the option
  'scope'" load error.
- `config.scope-matches-nothing` does not read `deferred_by_scope`: a rule told to wait for a diff is not a
  configuration that reaches nothing, so a plain `check` raises no warning for it, while a real
  `out_of_scope` still does. `waiver.matches-nothing` leaves a waiver on such a rule eligible and
  unexamined, because a whole-corpus run of the rule looked at almost nothing. The actions of `ADR-73` are
  unchanged.
- A rule with the option is silent on a plain `check` by design, so it cannot be a gate over the whole
  corpus. Remove the option to see the full backlog.

### Done-means items: `statuses.delivery.items` (`ADR-96`, `MILE-214`)

`statuses.delivery.items` (optional) makes a type's Done means a list of items that each name their own
evidence, and computes the `Status` from the items instead of from the record-level evidence field:

```yaml
statuses:
  delivery:
    delivered_tiers: ["Implemented", "Verified"]
    unevidenced_tiers: ["Not started"]
    not_done: ["Planned", "InProgress"]
    set_status: "Done"
    items:
      section: "Done means"            # must be a section the type requires
      clause_separator: " — "
      evidence_marker: "evidence:"
      waiver_marker: "waived:"
      partial_status: "InProgress"     # in values and in not_done
      resolving_tiers: ["Specified", "Implemented", "Verified"]   # default: delivered_tiers
      kind:                            # optional: a record declares its kind
        field: "Kind"                  # a field the type declares
        kinds:
          - {name: build}              # names nothing: the type's delivered_tiers
          - name: decision
            delivered_tiers: ["Specified", "Implemented", "Verified"]   # each also in resolving_tiers
            cited_records: success     # a cited record counts only at a `success` status of its type
      waiver_categories:
        - {name: not-applicable}
        - {name: superseded-by-decision, cite: required}
        - {name: deferred-to, cite: required, expires: required}
        - {name: process-outcome}
```

Absent, the type behaves exactly as before. Declared, the grammar of the item lines is `SPEC-6`'s, and
the engine reads only the three declared words (the separator and the two markers), never a position or a
guess. The keys are load-time errors when they would be inert or ambiguous: a `section` the type does
not declare under `sections.required`; a blank separator, marker or category name; the two markers equal
or one a prefix of the other; a `partial_status` outside `values`, outside `not_done`, or one the
type's `transitions` do not allow from a `not_done` status; a `resolving_tiers` entry that is not a
declared embodiment tier; a `delivered_tiers` entry that is not in `resolving_tiers` (the error names both lists), because no item could resolve at it; a duplicate category or one whose name holds whitespace, a parenthesis or a
comma; an `expires: required` category without `cite: required` (a deferral needs a target); and
`items` on a type whose `status.delivery-agreement` is off or not declared in `rules`, because that
rule's `DoneWithUnresolvedItems` finding is what holds a status to the items.

A record declares its kind (`ADR-96`, amendment of 2026-11-07): `kind.field` names the header field and
`kind.kinds` is the vocabulary. A kind's `delivered_tiers` replaces the type's for the rule that a record
needs at least one item resolved at a delivered tier, and for the delivered and partial standings, so a
`decision` kind that counts `Specified` is done when every item resolves at `Specified` or higher. A kind
that omits `delivered_tiers`, a record with no `Kind` and a type with no `kind` key all use the type's
`delivered_tiers`. `cited_records: success` reads the status of a record an item cites: a locator that
resolves to a record counts only while the record's status is one its own type declares `success`, so a
Proposed decision or a Draft spec evidences nothing; a locator that is not a record is not judged by status.
The keys are load-time errors when they would be inert: a `field` that is blank or that the type does not
declare; no `kinds`, or a name listed twice; a `delivered_tiers` entry that is not a declared embodiment tier,
that is not in `resolving_tiers` (no item could resolve at it) or that is in `unevidenced_tiers`. A `Kind`
value outside the vocabulary is reported by `status.done-items-shape` and judged as no kind.

`items.pending` (optional) lists the texts that mean an item is not yet written, in the shape and with the
matching of `statuses.pending`: a whole value (trimmed, case-insensitive) or `{pattern: "..."}`. An item whose
text matches is an unfilled placeholder: it never resolves, its evidence and waiver clauses are ignored, and
`status.done-items-shape` reports it on its line (`ADR-96`, amendment of 2026-11-08). Omitted keeps today's
behaviour. An empty list, a blank or duplicate entry and an invalid pattern are load errors.
Whole-value entries match case-insensitively, but `{pattern: ...}` entries are case-sensitive regexes, and a real
item that begins with a declared pattern is flagged too, so a pattern should be specific to the template's wording.

A dependency caps the standing (`ADR-96`, amendment of 2026-11-07): where `items` is declared and
`roadmap.pairs` names a sequencing pair, a record whose items all resolve but which has an unfinished
dependency computes `partial_status`, not `set_status`. Nothing is added to the configuration for this; a
repository that omits `items` or `roadmap.pairs` is unchanged. `sequence.status-agreement` skips a blocked
record whose `partial_status` comes from the cap (every item resolves) and emits an info notice naming the
dependency, so `partial_status` may stay listed in `statuses.progress`; a record with an unresolved item in
that status is still reported.

The standing is computed per item and per record (`SPEC-2`): an item is resolved when its evidence
clause names at least one locator, every locator exists (the `embodiment.locator-exists` test) and the
tier computed over only that item's locators is in `resolving_tiers`; it is waived when it carries a
valid waiver clause and is not resolved. `Specified` counts in `resolving_tiers`, but a record needs at
least one item resolved at a `delivered_tiers` tier to compute delivered or partial. Delivery is judged
from the items only: a record-level `Realized-by` locator never makes an opted-in record delivered,
though item locators are evidence for `Embodiment`, drift, `embodiment.locator-exists` and
`fix --rebaseline` as `Realized-by` locators are.

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
  is the option on any other rule, a blank entry, and an entry that no record type declares as a
  field (it could never be examined).

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
> | 2026-11-04 | Bumped to `0.47`. A type's `revision_log` takes `required: true\|false` (default `true`), which `urzua fix --apply` reads; it may stand alone. **Why:** `MILE-159` -- records without a revision log section could never be written by `fix --apply`. | **substantive** |
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
> | 2026-11-04 | Bumped to `0.62`. Documented scoped evidence pins: the locator grammar and parse order, the hashed bytes, the top-level `locator_resolvers` declaration (`start`, `end`, `leading`, `container`, `member`, `trait_container`), the five end kinds, resolution and its failure reasons, with normative test vectors. **Why:** `MILE-185` / `ADR-74` -- a pin on a volatile file went stale on edits unrelated to its decision; resolvers are repository configuration, not engine knowledge. | **substantive** |
> | 2026-11-05 | Bumped to `0.63`. Documented the drift rule for a scoped pin: no file-based grace window, an audit date still applies, and a scope without a pin is judged on the whole file. **Why:** `MILE-185` / `ADR-74` point 8. | **substantive** |
> | 2026-11-05 | Bumped to `0.64`. Clarified that an unpinned scope that does not resolve is reported by `embodiment.locator-exists` while drift is still judged on the whole file, and that a commit anchor is not judged for drift. **Why:** review of `MILE-185` part 2, reconciling `ADR-74` points 6 and 8. | **substantive** |
> | 2026-11-05 | Bumped to `0.65`. Realized-by names the implementing code and tests, ready for `Shipped` (`MILE-7`). **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-core/src/config.rs, test:rust/crates/urzua-cli/tests/own_config_integration.rs, test:rust/crates/urzua-cli/tests/field_levels_integration.rs` -- stated `Drift detected` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Bumped to `0.66`. Status changed from Accepted to Shipped. **Why:** Maintainer decision, 2026-11-05: its Realized-by evidence computes Verified and an independent review judged it sound. | **substantive** |
> | 2026-11-05 | Bumped to `0.67`. Documented that `status.transition-allowed` matches a renamed record to its merge-base copy by `Stable-Id`. **Why:** `BUG-191` -- a rename read as a deletion plus an addition, so the record was judged as new. | **substantive** |
> | 2026-11-05 | Bumped to `0.68`. Documented the per-type `role: suppression` declaration: the engine reads which type suppresses findings from it instead of the type name `waiver`; at most one type declares it; the type named `waiver` is the suppression type only when no type declares the role. **Why:** `BUG-74`, maintainer decision of 2026-11-05 -- the literal made suppression silently absent for a corpus that named its exception record anything else. | **substantive** |
> | 2026-11-05 | Bumped to `0.69`. Documented `effective_requires` on the suppression type: the fields a suppression must carry before it takes effect. A waiver missing one, or holding a blank, placeholder or pending value (the type's `statuses.pending` vocabulary when declared), suppresses nothing; its target finding is reported again and `waiver.matches-nothing` reports the waiver as `waiver-ineffective`. Omitted, behaviour is unchanged. Refused at load on a type without `role: suppression`, when empty, blank or repeated, or naming a field the type does not declare. **Why:** `BUG-177`, maintainer decision of 2026-11-05. | **substantive** |
> | 2026-11-05 | Bumped to `0.70`. Documented that a type's `revision_log.required` also decides whether `revision-log.change-class-required` reports a record with no revision-log section. **Why:** `BUG-50` -- a record could leave the rule's scope by losing one line, and absence was indistinguishable from compliance; the rule reports it only for a type that declares `required: true`, so a configuration that never declared `revision_log` is unchanged. | **substantive** |
> | 2026-11-05 | Bumped to `0.71`. A type's declared `identity` also decides how its H1 id is read: the `prefix` capture and a number, with a title that does not match reported as carrying no id. **Why:** `BUG-32`, maintainer decision 2026-11-05. | **substantive** |
> | 2026-11-05 | Bumped to `0.72`. A `dir` or `extra_dirs` entry not in canonical form (`./docs/adr`, `docs/adr/`, `docs//adr`) is a config load error (exit 2) that names the canonical spelling, where it used to parse and match nothing. **Why:** `BUG-168`, maintainer decision 2026-11-05. | **substantive** |
> | 2026-11-05 | Bumped to `0.73`. Documented declared boundaries for `boundary.published-path-reference`: the `boundaries` option (`paths`, `kind`, `reason`, `severity`, `exclude`) and `ignore_marker`, the prohibitions each kind carries, the boundary's severity as the level of its findings, `published_paths` read as a published boundary (not deprecated, and now a prefix-or-glob list), the published skip in the evidence collector, and the three reported states (an unused `exclude`, an unused marker, an uncovered path). **Why:** `MILE-125`, `ADR-80`. | **substantive** |
> | 2026-11-05 | Bumped to `0.74`. Documents the top-level `override_log` declaration and `override.unreviewed`: the log's table format, what "reviewed" means (a named person, not a blank, placeholder or declared pending value), that a declared log which does not exist is reported, and the load errors. **Why:** `MILE-188`, `ADR-78` item 1. | **substantive** |
> | 2026-11-05 | Bumped to `0.75`. Documents `embodiment.locator-on-back-pointer-path` and its `back_pointer_paths` and `test_paths` options: what it flags, what a comment cannot say and it therefore leaves, and that it offers no edit action (an exemption, as for the back-pointer rules). **Why:** `MILE-189`, `ADR-78` item 4. | **substantive** |
> | 2026-11-05 | Bumped to `0.76`. `fail_on` names `urzua audit` as an alias of `check`. **Why:** `MILE-190`, `ADR-79`. | **structural** |
> | 2026-11-05 | Bumped to `0.77`. Added "Supported schema versions": the window of `N` and `N-1`, the table of releases, the in-memory upgrade and `config.schema-version-outdated`, the structured out-of-window error, `urzua migrate config`, what version 1 was and what its upgrade does (a type that omits `header_shape` is refused, not silently moved from `blockquote` to `yaml-frontmatter`), and the rule for when a bump is required. The open question about a version field is answered. **Why:** `ADR-77` / `MILE-34` -- a config on an older schema version was rejected with a bare equality error and no path forward. | **substantive** |
> | 2026-11-05 | Bumped to `0.78`. A type's `statuses` declaration gains `progress` (statuses that claim work advanced) and `blocked` (written waiting statuses, opt-in), and a top-level `roadmap.pairs` names the sequencing pairs the derived `blocked` value reads. The opt-in rule `sequence.status-agreement` reports a blocked record that states a `progress` status, and a written blocked status while nothing blocks the record; a progress-status record with an unresolved or unreadable dependency is counted unreadable even when the type writes no blocked status. **Why:** `MILE-144`, decided in `ADR-83` (maintainer decision, 2026-11-05: `Blocked` is derived only, per `RFC-48`). | **substantive** |
> | 2026-11-05 | Bumped to `0.79`. A top-level `roadmap: {pairs, group_by}` names the `relation_pairs` entries that are dependencies and the header fields `urzua roadmap` groups by, validated at load. **Why:** `MILE-182`, decided in `ADR-82`: one graph over the declared sequencing pairs, with the pairs declared because an `acyclic` containment pair is not a dependency. | **substantive** |
> | 2026-11-05 | Bumped to `0.80`. Documented the `locator_kinds` and `pinnable` options of `embodiment.locator-exists`: a `Realized-by` locator may name a directory or glob when the repository declares that kind, a directory or glob is legal when it covers a tracked file, its drift uses the newest commit among the files it covers, its pin hashes the sorted (path, file content hash) list, `explain` shows the files, and an undeclared kind is a finding naming the option. Both options default to `[file]`. **Why:** `BUG-176` -- maintainer decision, 2026-11-05: which locator kinds are legal, and which can be pinned, is configuration. | **substantive** |
> | 2026-11-05 | Bumped to `0.81`. Documented the decision gate options of `diff.source-requires-record` (`decision_types`, `fix_types`, `link_fields`) and their load-time validation. **Why:** `MILE-133`, maintainer decision 2026-11-05. | **substantive** |
> | 2026-11-05 | Bumped to `0.82`. A resolver's `member` may be a list of entries, each with its own `start` and `end`, tried together (a Rust method ends `brace_balanced`, a struct field `indent`); a `container` may match a `struct` as well as an `impl`, so `Type::field` is addressable. A single mapping is read as before. Vectors 21-24 added. **Why:** `ADR-74` made the symbol the scope, but its `member` could name only an `fn`, so a decision about one struct field could pin only a line range, which shifts when anything is inserted above it; the declared resolver now reaches the field, with no language knowledge in the engine (`ADR-74` point 5: a member declares its own start pattern and end kind from the closed set). | **substantive** |
> | 2026-11-05 | Bumped to `0.83`. Documents the `closes` and `strict` options of `claim.status-agreement`: a changeset fragment lists the records it closes in a one-line frontmatter key, each id must resolve (`claim.id-unresolved`) and be at a terminal status its own type declares, a block list or malformed value is `claim.field-malformed`, `strict` makes a fragment without the key a finding, and a file with no frontmatter is out of scope and disclosed, except that under `strict` an unterminated `---` fence or a blank line before the opening fence is `claim.field-malformed`. **Why:** `BUG-66`, decided by `ADR-85`; the fence case because the pinned release tool rejects both. | **substantive** |
> | 2026-11-05 | Bumped to `0.84`. Documented `permanent` on the suppression type: per rule, a suppression without `Expires` takes effect only under a declared category and, where the category requires it, a resolving citation; declaring it closes permanence for unlisted rules; the load-time validation. **Why:** `MILE-193` / `ADR-86`. | **substantive** |
> | 2026-11-05 | Bumped to `0.84`. Documents the top-level `block_on: introduced \| all` key for the editor hook: `introduced` is the default and what an absent key means, `all` blocks on any error in the edited record, any other value is a load error. **Why:** `MILE-195`, `ADR-78` amendment of 2026-11-05. | **substantive** |
> | 2026-11-05 | Bumped to `0.85`. `info` is a rule level and a `field_levels` level and a boundary `severity` (`ADR-87`, `MILE-194`): findings are reported and counted, never `blocking`, never fail `fail_on`, and a rule set to `info` caps every declared severity at `info`. `fail_on: info` and every other unrecognised level are refused at load, the level message listing `off`, `info`, `warn` and `error`. **Why:** `MILE-194` builds `ADR-87`. | **substantive** |
> | 2026-11-05 | Bumped to `0.86`. Documented `permanent` on the suppression type: per rule, a suppression without `Expires` takes effect only under a declared category and, where the category requires it, a resolving citation; declaring it closes permanence for unlisted rules; the load-time validation. **Why:** `MILE-193` / `ADR-86`. | **substantive** |
> | 2026-11-05 | Bumped to `0.87`. `statuses.delivery` documented as applying to any record type, with the optional `back_pointer_evidence` key (default `true`, today's reading; `false` holds the type to its declared evidence field alone), an `unevidenced_tiers` that may be empty, the waiver for a deliberately open record with evidence, and the load error for a type without its evidence field. **Why:** `ADR-84`, accepted by the maintainer on 2026-11-05. | **substantive** |
> | 2026-11-05 | Bumped to `0.88`. `roadmap.pairs` also sets what `sequence.cycle` reads as one graph: a loop that uses an edge of each of two declared pairs is reported, and an `acyclic` pair not named, or an absent `roadmap.pairs`, keeps the per-pair reading. The load-time check that each named pair declares `acyclic: true` is what guarantees every pair in that graph is cycle-checked. **Why:** `MILE-192`, following `ADR-82` (design re-check of 2026-11-05): the roadmap saw a cross-pair loop that `check` did not. | **substantive** |
> | 2026-11-05 | Bumped to `0.89`. Records that `condition`, the record type for a fact nobody builds (`SPEC-23`), is declared in configuration alone from the primitives documented here, with no engine code that reads the name, and that a `Depends-on` resolves to it through the cross-type sequencing pair. **Why:** `MILE-187`, `RFC-48`. | **substantive** |
> | 2026-11-05 | Bumped to `0.90`. `urzua index --apply` replaces an artifact through a temporary file and a rename. **Why:** review of the CLI found the write truncated the file first. | **substantive** |
> | 2026-11-05 | Bumped to `0.91`. `closed_statuses` and `terminal_statuses` are optional once no record type needs them (every type declares `statuses`, or declares no status field); `embodiment.locator-on-back-pointer-path` no longer flags a record of a type declaring `statuses.delivery.back_pointer_evidence: false` and tells a locator whose file already carries the comment to drop the locator. **Why:** hardening review: the lists were required yet never read in a configuration where every type declares `statuses` (`BUG-178`), so they could only drift from the declared vocabulary; and the nudge told a `Fixed` bug to drop its only counted evidence, which `status.delivery-agreement` then reports. | **substantive** |
> | 2026-11-05 | Bumped to `0.92`. The compiled-in `field.pending` match reads `pending` as a whole word and ignores a path-shaped token, so `spending` and `docs/pending-settlement/x.md` are no longer pending; a type declaring `statuses.pending` is unchanged. **Why:** Phase 0 hardening from an adopter switchover -- a waiver `Scope` path containing `spending` was reported as unfinished work. | **substantive** |
> | 2026-11-05 | Bumped to `0.93`. Documents `roadmap.work_types` (`MILE-196`): the optional list of record types that are work, its effect on `urzua roadmap`, and its `roadmap: ...` load errors (an unknown, repeated or blank name, and an empty list). **Why:** `ADR-82`'s Amendment of 2026-11-05, accepted by the maintainer. | **substantive** |
> | 2026-11-05 | Bumped to `0.94`. Documents the `verb_claims` option of `claim.status-agreement` (`MILE-197`): `false` turns the closing-verb reading off so only `closes` is a claim; default `true`; a load error without `closes` (either value) or on any other rule; a notice when set without `strict`. **Why:** `ADR-88`, accepted by the maintainer. | **substantive** |
> | 2026-11-05 | Bumped to `0.95`. Documents the `scope` rule option (`scope: changed`): its one value, which rules take it (the table's `judges` column), how it composes with `--base` and `--stdin-path`, that dependents are not changed, and that `config.scope-matches-nothing` and `waiver.matches-nothing` treat a deferred slot as neither unreachable nor missing a finding. `embodiment.locator-on-back-pointer-path` takes it. **Why:** `MILE-203`, decided in `ADR-90`. | **substantive** |
> | 2026-11-06 | Bumped to `0.96`. Documents the top-level `harness` key (`adapters`, `instructions_file`, `guardrails` with `enabled` and `on_unjudged`) and `authority`: an absent key installs and enables nothing, an unknown key or guardrail and a guardrail no chosen adapter provides are reported and never skipped, `acceptance` or `protected-paths` enabled with no `authority` is a load error, and `on_unjudged: deny` refuses an agent and allows a person. **Why:** `MILE-198`, decided in `ADR-89` section 3. | **substantive** |
> | 2026-11-06 | Bumped to `0.97`. Documents the top-level `attestation` key (`identity: gh-verified \| asserted`, `require_verified`): the identity source of `fix --apply` and `fix --rebaseline --apply`, its recording in the revision row beside the actor kind, and the load errors for an unknown value, an unknown key and `require_verified` with `asserted`. **Why:** `MILE-201`, `ADR-89` section 5. | **substantive** |
> | 2026-11-06 | Bumped to `0.98`. Documents the top-level `authority` table (`deciders` groups of `logins`, `evidence` defaulting to `pr-author`, `gates` by type and status and by path) with its load errors, the local refusal of an agent's edit-tool move to a gated status or write to a gated path (final, not liftable by the override), the rule option `require_evidence`, the actors file and the accepted risk; an enabled `acceptance` or `protected-paths` guardrail with no gate of its kind is a load error. **Why:** `MILE-200`, decided in `ADR-89` section 4, accepted by the maintainer. | **substantive** |
> | 2026-11-06 | Bumped to `0.99`. The `authority` example's protected paths no longer list `scripts/hooks`, which this repository deleted. **Why:** `MILE-204`. | **substantive** |
> | 2026-11-06 | Bumped to `0.100`. States that claim ages and every date comparison use UTC. **Why:** maintainer decision of 2026-11-05: UTC everywhere. | **substantive** |
> | 2026-11-07 | Bumped to `0.101`. Documents the top-level `discovery` key (`include_untracked`, a boolean, default `false`): untracked, non-ignored record files directly under a declared directory join discovery, and an unknown key or a non-boolean value is a load error. **Why:** `MILE-206` / `ADR-92`. | **substantive** |
> | 2026-11-07 | Bumped to `0.102`. Documents the top-level `new` key (`check_open_prs`, `pull_ref_patterns`, `remote`, `fetch_timeout_seconds`, `lock_timeout_seconds`) and its load errors. **Why:** `MILE-89` / `ADR-93`. | **substantive** |
> | 2026-11-07 | Bumped to `0.103`. The rule table's `judges` column gains `anchor-scoped`, which with `one-record` is the only value that lists `scope` (`ADR-90` amendment); `embodiment.locator-promotion-candidate` is labelled with it and, under `scope: changed`, emits one finding per changed citer. **Why:** the accepted 2026-11-07 amendment to `ADR-90`, points 4 and 5. | **substantive** |
> | 2026-11-07 | Bumped to `0.104`. `date.field-valid`'s `date_fields` entries are validated at load: a blank entry or one that no record type declares is an error, as `date_order` already was. **Why:** a typo ran with no eligible slot and no notice. | **substantive** |
> | 2026-11-07 | Bumped to `0.105`. An `authority.gates` `paths` entry is normalised like `published_paths`, and one that could match nothing (`.`, a backslash) is a load error. **Why:** hardening review: `./.agent` loaded and gated nothing, for `diff.decider-required` and the `protected-paths` guardrail alike. | **substantive** |
> | 2026-11-07 | Bumped to `0.106`. A `harness.guardrails` name the build does not ship is a config load error for every command, suggesting the nearest valid name; unknown keys elsewhere in `harness` are still kept and reported. **Why:** `ADR-89` amendment of 2026-11-07, maintainer decision from the core-model review. | **substantive** |
> | 2026-11-07 | Bumped to `0.107`. The `sequence.cycle` bullet "One graph per pair" no longer says a loop across two acyclic pairs is never detected: it is reported when both pairs are named in `roadmap.pairs`. **Why:** the bullet contradicted `ADR-82`, `MILE-192` and the `roadmap.pairs` bullet below it; `ADR-76`'s matching Decision text carries a proposed amendment. | **substantive** |
> | 2026-11-07 | Bumped to `0.108`. Documents `statuses.delivery.items` (`ADR-96`, `MILE-214`): the opt-in per-item evidence for a type's Done means, its keys, its load errors and how the standing is computed from the items. **Why:** `MILE-214`, the engine of `ADR-96`; an undeclared config is unchanged. | **substantive** |
> | 2026-11-07 | Bumped to `0.109`. Documents the optional `revision_log` keys `classes`, `class_column`, `date_column` and `tool_class` (`ADR-94`, `MILE-213`): the class vocabulary, the header names of the class and date columns, and the class `urzua fix` writes, their engine defaults, and their load errors. A `\|` inside a log cell stays in its cell, and a row wider than its header is a finding when a column is declared. No `schema_version` bump. **Why:** `MILE-213`, the build of `ADR-94`; an undeclared config behaves as before. | **substantive** |
> | 2026-11-07 | Bumped to `0.110`. Documents that a dependency caps the standing computed from `statuses.delivery.items` (a record whose items all resolve beside an unfinished dependency computes `partial_status`) and that `sequence.status-agreement` skips such a record with an info notice, so `partial_status` may stay a `progress` status. No new key; no `schema_version` bump. **Why:** the `ADR-96` amendment of 2026-11-07, accepted by the maintainer; a configuration without `items` or a sequencing pair is unchanged. | **substantive** |
> | 2026-11-07 | Bumped to `0.111`. Documents `extension_prefixes` on `header.field-set-consistency` (`MILE-141`): the per-type extension prefix, its fallback to `allow_extension_prefix` for a type it does not name, and its load errors (empty or whitespace prefix, a collision with a declared field, an undeclared type or one without `known_fields`). **Why:** `MILE-141` Done means item 3, built on the maintainer's decision of 2026-11-07. | **substantive** |
> | 2026-11-07 | Bumped to `0.112`. `relation_fields.title` names the field `urzua new` fills with the record's title, and `revision_log.version_bump.initial` the version it writes into the `version_bump` field; both are optional and leave the field blank when undeclared. A status listed in `sections.exempt_statuses` that no record of the type holds is reported by `type.dir-matches-nothing`. **Why:** `MILE-74` -- a scaffolded spec failed `check` on a blank `Version` and `Subject` that `new` could have filled; `MILE-7` -- an exemption that sets nothing aside was silent. | **substantive** |
> | 2026-11-07 | Bumped to `0.113`. Documents `statuses.delivery.items.kind`: a field and a vocabulary of kinds, each with optional `delivered_tiers` and `cited_records: success`, so a `decision` milestone is done when every item resolves at `Specified` or higher and cites only decided records; the load errors; opt-in. **Why:** the `ADR-96` amendment of 2026-11-07, accepted by the maintainer (`MILE-73`). | **substantive** |
> | 2026-11-07 | Bumped to `0.114`. Documents `embodiment.claim-expired` and `embodiment.claim-renewal-unattested` (`ADR-97`, `MILE-129`): the `periods`, `anchor_field`, `require_pin` and `max_batch` options, the most-specific-entry rule, the computed tier, the `spread_days` first-cycle offset, the load errors, and that declaring `claim_max_age_days` beside the rule is refused. Off until declared. **Why:** `MILE-129`, the build of `ADR-97`. | **substantive** |
> | 2026-11-07 | Bumped to `0.113`. Documents `embodiment.claim-expired` and `embodiment.claim-renewal-unattested` (`ADR-97`, `MILE-129`): the `periods`, `anchor_field`, `require_pin` and `max_batch` options, the most-specific-entry rule, the computed tier, the `spread_days` first-cycle offset, the load errors, and that declaring `claim_max_age_days` beside the rule is refused. Off until declared. **Why:** `MILE-129`, the build of `ADR-97`. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/config.rs@sha256:18818ee3b226, rust/crates/urzua-cli/tests/own_config_integration.rs@sha256:cba8b084a518, rust/crates/urzua-cli/tests/field_levels_integration.rs@sha256:3c8aa6715827 Reason: Pinned the files as they are now after checking each against the record it evidences: config.yaml and the own-config test still declare the adr, rfc, spec and condition types that SPEC-16, SPEC-17, SPEC-18 and SPEC-23 describe, with spec_shipped_integration.rs, rules/roadmap.rs, condition_type_integration.rs and work_types_integration.rs behind them, check.rs, rules.rs, check_integration.rs and fail_on_integration.rs still implement urzua check (SPEC-2), the workflows, the install-knope action and the release and gate test scripts still make up the pipeline SPEC-20 describes, the Makefile and checks.yml still dispatch the repository layout of SPEC-21, the harness modules, the bundled hook script and the harness integration tests still implement the request, verdict and adapter layout of SPEC-24, config.rs and the own-config and field-levels tests still implement SPEC-3, and acceptance.rs and acceptance_property.rs still implement SPEC-4. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Bumped to `0.115`. The pins on this spec's `Realized-by` locators were re-baselined against the files as they are after the sharded CI and the milestone `Kind` amendment landed on main (the `Re-baselined` row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97`: every stale pin is re-pinned with its real delta before expiry is enabled. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/own_config_integration.rs` cba8b084a518 -> 06fa48e432cb. Reason: Read the diff of each pinned file since the old pin and the claim still holds. own_config_integration.rs: one test added, own_config_declares_a_level_for_every_shipped_rule, which fails when a shipped rule has no declared level in the repository config; the existing tests are unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Bumped to `0.116`. The pins this spec's `Realized-by` holds on files changed in this change were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. **Why:** a revision-log entry that adds a structural change needs a `Version` bump (`revision-log.version-bump-required`). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/own_config_integration.rs` 06fa48e432cb -> 92a2b9ceca89. Reason: Read the diff of each pinned file since the old pin and the claim still holds. own_config_integration.rs: own_config_declares_a_level_for_every_shipped_rule now checks the parsed config's rule levels instead of text-matching the file; no other test changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Bumped to `0.117`. Config load refuses a type whose `delivered_tiers` names a tier outside `items.resolving_tiers`, naming both. **Why:** round-4 review of the delivery items. | **substantive** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/config.rs` 18818ee3b226 -> df8b21e9c8a5. Reason: split parse_with_document_models, validate_relation_pairs, claim_expiry_problems and revision_log_problem in config.rs into named validation helpers (SPEC-22 100-line bound); behaviour unchanged, outputs byte-identical, validation order and messages pinned by the new validation_order_tests **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/config.rs` df8b21e9c8a5 -> 20203e601224. Reason: Read the diff of each pinned file or range since the old pin against what this record evidences: config.rs gained the validate_ignore_markers helper, called first in validate_boundaries, which accepts ignore_marker on embodiment.back-pointer-resolution and embodiment.back-pointer-decision-accepted, validates the marker (non-blank, one line, no padding) ahead of the boundary-only checks so it covers every rule that takes the option (this replaces the boundary rule's inline check, with the same errors), refuses a marker inside the Implements: or Verifies: keyword on the two back-pointer rules and refuses two different markers on them; no other option, parse or default changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Bumped to `0.119`. Documents `ignore_marker` on `embodiment.back-pointer-resolution` and `embodiment.back-pointer-decision-accepted`: the boundary rule's option reused as one marker for both rules and the evidence collector (differing values are a load error), excusing the whole line (not reported, not judged, not evidence), a load-time guard against a marker inside a back-pointer keyword, disclosure as `excused_lines` and an `info` notice in `check`, `fix` and `reverify`, and an unused marker reported once per line; opt-in. **Why:** `ADR-80` amendment of 2026-11-08, accepted by the maintainer. | **substantive** |
> | 2026-11-08 | Bumped to `0.120`. States that `urzua check` reports an unknown `harness` key, or an unknown key inside a guardrail, as a warning notice. **Why:** the `ADR-89` amendment of 2026-11-07 keeps these "reported, not an error", but only `harness install`, `doctor` and the session hook read them; a typo such as `enabeld` left the guardrail off while `check` was silent. | **substantive** |
> | 2026-11-08 | Bumped to `0.121`. Documents `statuses.delivery.items.pending`, an optional placeholder vocabulary for Done-means items in the shape of `statuses.pending`. **Why:** `ADR-96` amendment of 2026-11-08 (placeholder items). | **substantive** |
> | 2026-11-09 | Bumped to `0.122`. A back-pointer rule's `ignore_marker` is also refused at load when it has fewer than 3 non-whitespace characters, has no letter or digit, or is part of a declared record id head such as `ADR-`. **Why:** tightens the guard of the 2026-11-08 amendment, which refused only text inside a back-pointer keyword; `//`, `#` or `ADR-` loaded and excused every back-pointer line. | **substantive** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/config.rs` 20203e601224 -> 921027c81b7e. Reason: config.rs: read the diff since the previous pin; it adds generic_marker_problem and its call in validate_ignore_markers (a back-pointer ignore_marker shorter than 3 characters, with no letter or digit, or inside a record id head is refused at load) plus its test; this adds the guard this record now documents (0.122 row above), and no other claim changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.123`. A back-pointer rule's `ignore_marker` is also refused at load when, after removing the back-pointer keywords, the declared record id heads that start no record id, comment leaders and punctuation, it leaves fewer than 3 letters or digits (`// Implements:`, `# Verifies:`, `-- Implements: ADR`). **Why:** round-5 review: the guard refused only text inside a keyword or id head, so a padded marker such as `// Implements:` loaded and excused every back-pointer line. | **substantive** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/config.rs` 921027c81b7e -> 3cfc01bcd73e. Reason: Read the diff of config.rs since the old pin against what this record evidences: the only change is generic_marker_problem and its new helper strip_generic_head (a back-pointer ignore_marker that is only keywords, id heads, comment leaders and punctuation around fewer than 3 letters or digits is refused at load) plus its test cases; the behaviour this record evidences is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.124`. Note on the log: The version numbers of earlier `Bumped to` rows are not strictly increasing, and history is not rewritten: the order of the rows is the order the changes landed, and the current `Version` header is authoritative. Rows affected: `0.6` after `0.7`; `0.10` after `0.11`; `0.8`, `0.21` and `0.84` each twice; `0.113` twice, the second after `0.114` (the claim-expiry documentation row). **Why:** a review found the out-of-order and repeated numbers and a reader could be misled about which row introduced which version. | **structural** |
> | 2026-11-09 | Bumped to `0.125`. Documents `harness.outside_writes` (`allow`, `shared`, `deny`) and `harness.allow_paths`: an unknown value or a relative path is a load error, `allow_paths` beside a mode other than `deny` is reported by `doctor`. **Why:** `ADR-89` amendment of 2026-11-09 (outside writes), accepted by the maintainer: a linked worktree at a commit without gates was an unjudged place to write. | **substantive** |
> | 2026-11-09 | Bumped to `0.126`. Documents `drift` on `embodiment.claim-expired`: a per-evidence-category policy (`block`, `acknowledge`, `ignore`; undeclared is `block`) for what drift of a locator does to a renewal by `urzua reverify`, with its load errors. **Why:** `ADR-97` amendment of 2026-11-09 (drift policy); a drifted `spec:` locator is unpinned by design, so `reverify --pin-only` could not clear it and 66 of 355 claims could not be renewed. | **substantive** |
> | 2026-11-09 | Bumped to `0.127`. Documents `authority.decider_table` (`base`, `head`, `both`; undeclared is `head`): which revision's table `diff.decider-required` reads, the stricter of the head's and the base's declaration, a base with no config or no `authority` holding no deciders and no gates, an unreadable base falling back to the head table with a warning, an `info` notice with no `--base`; `urzua init` proposes `base` and `urzua doctor` warns when gates are declared without it. **Why:** `ADR-89` amendment of 2026-11-09 (decider table), accepted by the maintainer. | **substantive** |
