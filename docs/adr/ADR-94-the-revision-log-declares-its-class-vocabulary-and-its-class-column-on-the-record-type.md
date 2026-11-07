---
Stable-Id: 01M6SVM84PY8AW2N8VS8XQV0TV
Status: Proposed
Date: 2026-11-07
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
---
# 94 — The revision log declares its class vocabulary and its class column on the record type

## Context

Line references are to `origin/main` at `5f6c062f`. Paths without a directory are under
`rust/crates/urzua-core/src` (`rules/revision_log.rs`, `config.rs`, `fix.rs`) or, for `check.rs` and `init.rs`,
`rust/crates/urzua-cli/src/commands`.

`ADR-64` makes config-driven the founding claim, and `AGENTS.md` names a hardcoded type-specific assumption in
Rust source a regression against it. A record type already declares how its revision log is found
(`marker` or `heading`, `MILE-167`), whether it is required (`MILE-159`, `BUG-50`) and which classes require a
version bump (`BUG-51`). Three things about the log are still literals in the engine. The maintainer decided on
2026-11-07 that the rule must not hardcode its vocabulary.

What exists today:

- **The class vocabulary is two literals.** `revision-log.change-class-required` accepts a row only when its
  class, with emphasis and whitespace stripped, is `substantive` or `structural`
  (`rules/revision_log.rs:61-62`). The finding says `expected "substantive" or "structural"`
  (`rules/revision_log.rs:73`), and the rule's description repeats it (`rules/table.rs:537`, `:559`).
- **The class column is the last cell.** The parser reads the date from `cells.first()` and the class from
  `cells.last()` (`rules/revision_log.rs:293`). The header row is skipped unread (`rules/revision_log.rs:283`),
  so a table titled `Date | Change | Class` and one titled `Class | Date | Change` are both read by position.
  Both rules take the class from this one parser (`rules/revision_log.rs:61`, `:173`).
- **The version-bump rule is already declared, but its classes are free text.** `revision_log.version_bump`
  is `{field, classes}` (`config.rs:1081-1088`); the rule keeps an added row whose class is in `classes`
  (`rules/revision_log.rs:173-174`). Load refuses a blank `field`, an empty `classes` and a blank class
  (`config.rs:2191-2200`), but never checks a class against any vocabulary, because there is none to check against.
  This repository declares `classes: ["substantive", "structural"]` on `spec` only (`.urzua/config.yaml:242`).
- **The tool writes a literal class.** `urzua fix --apply` appends `**structural**` to its row
  (`fix.rs:385`) and `fix --rebaseline` does the same (`fix.rs:632`). `ADR-15` states the rule
  ("every applied write appends a `structural` revision-log entry", `ADR-15:43`). A repository whose vocabulary has
  no `structural` would have the tool write a row its own `change-class-required` rejects.
- **Nothing else reads the class.** `status.transition-allowed` (`rules/status.rs:337`) reads the header's status
  and the merge base's copy, not the log. The `skip_revision_log` option of `citation.*` reads the log's identity
  only (`rules/citation.rs:139`). `check.rs:131` asks only whether a type declares a bump. No rule enforces
  `ADR-14`'s "a substantive row states a real Why" (`ADR-14:78`); `fix.rs:632` merely writes one.
- **The defaults live in the engine, not in a generated config.** The marker default is the constant
  `DEFAULT_REVISION_LOG_MARKER` (`config.rs:1091`) returned by `RevisionLogId::default()` (`config.rs:1102`).
  `urzua init` generates a config, but it writes only `dir`, `required_fields`, `header_shape`, an optional
  `prefix` and the rule levels (`init.rs:475-503`, then the rule levels to `:550`) and infers nothing from the corpus (`ADR-33`); it declares no
  `revision_log` at all.
- **A new optional key is not a schema break.** `ADR-77` (`ADR-77:68-70`) and `SPEC-3` (`SPEC-3:1458`) bump
  `schema_version` only for a removal, a rename or a changed default; an additive optional key whose default
  preserves behaviour is not one.
- **This repository declares `revision_log` on all seven record types** (`.urzua/config.yaml:126`, `:180`,
  `:239`, `:255`, `:320`, `:379`, `:429`).

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Keep the literals (today) | Nothing to build | `ADR-64` regression; a repository with another vocabulary or column order cannot use the rule, and the fix writer breaks it |
| B. Declare the vocabulary only; keep the last-cell column | Smaller change | Half the hardcoding remains; a log with a trailing notes column is misread; the bump rule is still positional |
| C. Declare the vocabulary, the column by header name and the tool's class, per type; undeclared keeps today's behaviour from in-engine defaults (chosen) | Same place as `marker`, `required` and `version_bump`; no config changes for anyone who declares nothing; no schema bump | The two default names stay in Rust as one named constant pair |
| D. One top-level vocabulary for every type | Declared once | Types differ in what they log (`revision_log` is per type, `config.rs:976`); a new inheritance rule no other key has |
| E. Per-class flags that carry the bump (`classes: {substantive: {bump: true}}`) | One list | Breaks the shipped `version_bump.classes` shape; two ways to say the same thing |
| F. Defaults written by `urzua init` into the config, none in the engine | Engine names no class | Reaches only new configs; an existing undeclared config still needs an engine default, so the literals stay; rewriting existing configs needs a migration, a break `ADR-77` reserves for removed keys |
| G. No vocabulary check unless declared (accept any non-blank class) | The engine names no class | Changes behaviour for every undeclared config: a class such as `minor` is rejected today (`rules/revision_log.rs:62`) and would pass |
| H. Bump `schema_version` and migrate | Explicit upgrade | The key is additive and optional; `ADR-77` says no bump |

## Decision

*In the context of a revision-log rule that names two classes and one column in Rust source, facing a maintainer
decision of 2026-11-07 that it must not, we decided that a record type declares the log's class vocabulary, the
header names of its class column and, optionally, its date column and the class the tool writes, as optional
keys of its existing `revision_log` declaration, and that a type declaring none keeps today's behaviour from defaults held in the engine, to make the
rule serve any repository's vocabulary with no change for one that declares nothing, accepting that the two default
class names remain in Rust as one documented constant pair.*

1. **Four optional keys on `revision_log`**, beside `marker`, `heading`, `required` and `version_bump`
   (`config.rs:1063-1074`):
   ```yaml
   revision_log:
     marker: "**Revision log**"
     required: true
     classes: [substantive, structural]   # the vocabulary
     class_column: Class                  # a header cell's name
   date_column: Date                    # a header cell's name
     tool_class: structural               # what fix writes
     version_bump: {field: Version, classes: [substantive, structural]}
   ```
   Each may stand alone, like `required` (`config.rs:2179` allows a declaration with neither `marker` nor
   `heading` only when `required` or `version_bump` is present; it will list the new keys too).
2. **Undeclared keeps today's behaviour, from the engine.** No `classes` means the vocabulary
   `[substantive, structural]`; no `class_column` means the last cell, whatever the header says; no `date_column`
   means the first cell; no `tool_class` means `structural`. These are constants next to `DEFAULT_REVISION_LOG_MARKER` (`config.rs:1091`), reached the
   same way `RevisionLogId::default()` is (`config.rs:1102`). The default is in the engine, not in a generated
   default config, for two reasons. A generated config reaches only repositories that run `init` after the change,
   and the stated population, configs that declare nothing today, would still need an engine default (option F).
   And `init` infers nothing from the corpus (`init.rs`, `ADR-33`), so writing today's values into every adopted
   config would assert a vocabulary the adopter never chose. The engine keeps the names as a compatibility
   default and no rule reads a name outside that one pair.
3. **Vocabulary.** `classes` is a non-empty list of non-blank, distinct names; load refuses otherwise, naming the
   type, as `InvalidRevisionLog` does today (`config.rs:1507`, `:2203`). A row's class is matched as now: emphasis
   (`*`) and whitespace stripped, then compared exactly, case-sensitively (`rules/revision_log.rs:61`). Declared
   names get the same stripping before they are compared or checked for blankness, so "distinct" means distinct
   after stripping and case-sensitive: `structural` and `**structural**` are one name and refused as a duplicate,
   `Structural` and `structural` are two. The finding
   lists the declared names; with the default it reads exactly as today.
4. **Columns by header name.** With `class_column` declared, the class is the cell under the header cell of that
   name, in every log of the type. A header cell matches when, with emphasis and whitespace stripped, it equals the
   declared name exactly: case-sensitive, like class values, with no case folding or fuzzy match. A miss is already
   a loud finding that lists the cells found. A row shorter than the column has
   a blank class, which is the existing blank-class finding. A log whose header has no such cell, or has more than
   one (duplicate header cells are refused, not resolved by taking the first), is one finding on that record at the
   header line, naming the column, the cells found and, for a duplicate, their positions; its rows are not judged
   (a declared input that matches nothing is reported, not skipped). The finding is a new variant and so carries a
   `suggested_action` (`ADR-73`): `edit-section` on the revision log, like `change-class-required` (`ADR-73:89`),
   with an `action_source` and a `SPEC-2` row. `revision-log.version-bump-required` reads the
   same parser; for such a log it reports the record `Unreadable` in its population (the outcome it already
   uses, `rules/revision_log.rs:148`) and adds no second finding. The same holds for the base side: the rule reads the
   merge base's copy of the log with the same parser, and a base log it cannot read (the declared column missing or
   duplicated there) also makes the record `Unreadable`; it is not treated as an empty log, which would make every
   current row look added and raise false bump findings (today `find_revision_log_entries(then_text).unwrap_or_default()`
   does exactly that).
   With `date_column` declared, the date is the cell under that header name, found and reported by the same rules
   as `class_column` (a miss or duplicate is the same finding, naming the column); `date_column` and `class_column`
   may not name the same header, a load error. Undeclared, the date stays the first cell
   (`rules/revision_log.rs:293`). The date feeds a message and the row identity, never a rule outcome.
5. **Which classes require a bump.** Exactly the classes in `version_bump.classes`, unchanged in shape
   (`config.rs:1081-1088`); a declared class absent from it (a `cosmetic`, say) requires none, and a row whose class
   is outside the vocabulary is the change-class rule's finding and requires nothing from the bump rule
   (`rules/revision_log.rs:173-174`). When `classes` is declared, a `version_bump.classes` entry outside it is a load
   error naming the entry. When `classes` is undeclared, `version_bump.classes` is not validated against the
   default pair, so a config that loads today still loads.
6. **The tool's class.** `tool_class` must be in `classes` when both are declared. With `classes` declared and
   `tool_class` omitted, `structural` is used if the vocabulary has it; otherwise load refuses, naming the
   missing key, since `fix` would write a row the rule rejects. `fix --apply` and `fix --rebaseline` write
   `tool_class` where they write `structural` now (`fix.rs:385`, `:632`). The row is laid out by the log's own
   header: the date in the date column (the first when `date_column` is undeclared), the class in the class column,
   the narrative in the first remaining column, any other cell empty. A log with no class or date column, or no
   column left for the narrative, is refused as a log with no rows to append after already is (`fix.rs:365-374`,
   `:598-605`). This changes what an undeclared type's `fix` writes only for a log whose header does not have exactly
   three columns, and today's write there is a malformed row.
7. **This repository declares today's values** on all seven types: `classes: [substantive, structural]`,
   `class_column: Class`, `date_column: Date`, `tool_class: structural`. Nothing changes here. Because that is also the default, the
   non-default path is exercised by an acceptance corpus with another vocabulary and a reordered table, and by
   unit tests, not only by this repository's config.
8. **No `schema_version` bump** (`ADR-77:68-70`, `SPEC-3:1458`): three additive optional keys whose absence
   preserves behaviour.
9. **Not decided here.** A rule enforcing `ADR-14`'s Why for the classes that need one ("require a Why on
   substantive rows"). It adds a rule and a finding, and would change undeclared configs unless opted in (for
   example through a `classes_requiring_why` key), so it is a separate opt-in decision. Decided here, from the open
   questions of the review: the date column is declared by name (`date_column`, optional), because a reordered
   table otherwise reads the class as the date; header matching is exact, not case-insensitive.

## Reversibility

Easy to walk back before release: the keys are optional, and removing them returns every config to the defaults.
After a release an adopter's config may name them, so the key names become sticky, as `marker` and `heading` did.
The one irreversible-looking choice is the load error for a declared vocabulary with no usable `tool_class`; it is
loud and its fix is one line in the config.

## Consequences

- A repository can name its own classes and put the class column anywhere. The two default names remain in Rust as
  one pair; the maintainer should expect that sentence in review.
- A repository that declares nothing sees no change: same accepted classes, same message, same last-cell reading,
  same `**structural**` rows.
- A header typo in `class_column` becomes a visible finding on every record of the type, not a silent pass.
- The tool can no longer write a class its own rule rejects, and a vocabulary without `structural` forces a
  decision at load time instead of a failing `fix`.
- `ADR-15`'s "every applied write appends a `structural` entry" becomes "the type's `tool_class`", which defaults
  to `structural`, and `ADR-14`'s closed `substantive | structural` vocabulary and columns become declared
  defaults; both are amended on acceptance (2026-11-07) by dated amendment sections.
- The population of the change-class rule is unchanged (a record per log); a log with a missing class column is a
  finding, and the bump rule discloses it as unreadable, as it does for an unreadable base log.
- `SPEC-3` (the keys, their defaults, the load errors), `SPEC-2` and the rule table (`table.rs:537`, `:559`),
  `SPEC-8` (the written class and row layout) and `SPEC-4`'s corpus list change in the milestone that builds this.

## References

- `ADR-14` -- the revision log and its `substantive | structural` tag, including the Why amendment; amended
  2026-11-07 to make the vocabulary and columns declared.
- `ADR-15` -- tool-authored writes append a `structural` row; amended 2026-11-07 to the type's `tool_class`.
- `ADR-73` -- every finding variant declares a `suggested_action`.
- `ADR-20` -- `fix --apply` and its revision-log write path.
- `ADR-33` -- `init` infers nothing from the corpus.
- `ADR-64` -- structured config in, structured output out; config-driven.
- `ADR-77` -- when `schema_version` changes.
- `BUG-51` -- `revision-log.version-bump-required` and `version_bump`.
- `BUG-50`, `MILE-159`, `MILE-167` -- `required` and the log's identity.
- `MILE-213` -- the milestone that builds this decision.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Drafted as Proposed. **Why:** Maintainer decision, 2026-11-07: the revision-log rule must not hardcode its vocabulary; the class names and the class column (by header name) are declared on the record type, this repository declares today's values, and an undeclared config keeps today's behaviour. The maintainer accepts it in a later change. | **substantive** |
> | 2026-11-07 | Folded in the independent review of `#391`, accepted by the maintainer on 2026-11-07: an optional `date_column` declared by header name; exact header matching; a duplicate header cell is a finding, not resolved; "distinct" in `classes` is after stripping, case-sensitive; an unreadable base log makes the record `Unreadable`; the missing-column finding carries an `edit-section` `suggested_action`; `fix`'s row layout noted; `ADR-14` is amended alongside `ADR-15`; the Why rule stays a separate opt-in decision. **Why:** the review found the base-side `unwrap_or_default` would raise false bump findings, the new finding variant needs an `ADR-73` action, and a reordered table makes the first-cell date the class; maintainer decision, 2026-11-07. `Status` is unchanged here; acceptance is recorded separately. | **substantive** |
