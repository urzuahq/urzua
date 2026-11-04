---
Stable-Id: 01M6M7237A9CBQAHZJKYEK5ET7
Status: Proposed
Date: 2026-11-04
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: RFC-8, ADR-64
---
# 73 — A finding carries a structured suggested_action with a closed set of kinds

## Context

`MILE-44` asks for a `suggested_action` on every finding. `RFC-8` and `ADR-15` name the field on `urzua fix` detect entries and on Tier 3 findings that cannot be repaired unambiguously, and `MILE-44`'s Why says `SPEC-2` already documents `suggestedAction`. It does not: `BUG-21` rewrote `SPEC-2`'s output contract to the shipped shape, and `Finding` (`report.rs`) has `rule`, `severity`, `file`, `line`, `message`, `waived` and nothing else. No accepted record defines the field's shape, so the first implementation would define it by accident.

The shape is an output contract that agents parse, with 53 rules behind it. `ADR-64` settles one half: output a consumer is expected to act on is structured, not prose. Unsettled: which kinds of action exist, what each carries, how a consumer treats a kind added later, and what the engine may claim to know.

Walking the 53 rules of `RULE_TABLE` at 2026-11-04 shows what the shape must express:

- A `fix` repair exists and `urzua fix --apply` writes it: `embodiment.consistency`, `status.delivery-agreement` (`SPEC-8`).
- Another urzua command regenerates it: `index.freshness` (`urzua index --apply`), `citation.issue-exists` (`urzua refs refresh`). `fix` does not.
- A header field needs a value only a human can supply (`field.quality` blank or placeholder, `field.pending`, `status.required-field-missing`), or has a computable one (`field.untrimmed-value`, the trimmed value; `header.field-case-mismatch`, the declared spelling).
- A header field must be removed (`status.forbidden-field-present`) or a half-stated relationship completed on the other record (`relation.*-reciprocity`).
- A configuration edit: the `config.*` rules, `relation.target-status-undeclared`, `type.dir-matches-nothing`.
- A record to create (`diff.source-requires-record`).
- Things no record-field edit expresses: a non-record file (`claim.status-agreement`, `embodiment.back-pointer-*`, `boundary.published-path-reference`), a rename or move (`filename.title-consistency`, `identity.collision`, `type.record-outside-declared-dir`, `filename.identity-pattern`), deleting a record (`waiver.matches-nothing`), and prose located by line (`citation.resolution`).
- Advisory findings with no edit: `embodiment.locator-promotion-candidate`, `type.no-declared-spec`, `diff.source-and-records-mixed`.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. A prose `suggested_action` string | Trivial | Not machine-actionable; contradicts `ADR-64` |
| B. A structured object: closed-per-release `kind`, flat `target`, required `op` where an edit has several forms, declared per finding variant with reasoned exemptions (chosen) | Agents dispatch on `kind`; testable coverage; one place for each computed value | The vocabulary is a public contract |
| C. An open `kind` string with free-form params | No up-front design | No dispatch; a coverage test means nothing |
| D. Populate only the mechanically obvious rules and omit the rest | Smallest change | A consumer cannot tell "no action by design" from "not written" |

## Decision

In the context of findings that say what is wrong but never what to do, facing an output contract that agents parse and that `ADR-64` requires to be structured, we decided that **every finding variant declares a `suggested_action` object or a reasoned exemption, with `kind` from a vocabulary closed per release, a flat `target`, and a `value` only where one is derivable**, to achieve findings an agent can act on without reading prose, accepting that the vocabulary is a public contract.

Keys are snake_case and kinds kebab-case, like the rest of the report. `MILE-44`'s `suggestedAction` spelling is corrected to `suggested_action`.

```json
{ "kind": "edit-field", "op": "set", "target": { "file": "docs/bugs/BUG-9-x.md", "field": "Regression-test" } }
```

### The vocabulary (normative; `SPEC-2` reproduces this table verbatim)

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

### Declaration

The declaration is per finding variant, in `RULE_TABLE` (`MILE-171`): for every variant a rule can emit, a function from the finding's context returns an action or `Action::Exempt(&'static str)` with a reason, modelled on `HeaderNone::Exempt`. A variant that returns nothing without an exemption fails a coverage test. `Finding` gets the field with a default of none and a constructor, so the test, not the compiler, catches a missing declaration. Exemption reasons are shown by `urzua rules`, so "no action by design" is discoverable although it is an omitted field on the wire. A waived finding carries its action unchanged.

### Compatibility

The set of kinds, `op` values and `target` keys is closed per release and extensible across releases. Adding a kind, an `op` value or an optional `target` key is non-breaking. Renaming or removing one, changing its meaning, or changing which keys are required is breaking. Consumers MUST treat an unrecognised `kind`, or an unrecognised `op` on a known kind, as non-actionable and fall back to `message`, and MUST ignore unrecognised `target` keys. `SPEC-2` states this in its output contract. The report has no output version field today; if one is added it is a separate record and kind additions stay minor.

### Per-rule declaration (appendix; exemptions carry the reason)

| Rule | Declared action |
|---|---|
| `header.required-fields` | missing field: `edit-field` set; duplicate key: `edit-field` remove; missing header region: exempt, no kind addresses a region |
| `pointer.resolution` | `edit-field` set (repoint), no value |
| `pointer.target-status` | `edit-field` set, no value |
| `field.quality`, `field.pending` | `edit-field` set, no value |
| `claim.status-agreement` | exempt: the rule catches a claim that may be false, so the right fix is often the claim file (a non-record file, no kind yet) and editing `Status` would be the mechanical answer and possibly wrong |
| `filename.title-consistency` | exempt: a rename or an H1 edit, no kind yet |
| `relation.supersession-reciprocity`, `relation.pair-reciprocity` | `edit-field` append on the counterpart, value computed |
| `relation.cross-type-disallowed` | `edit-field` remove on the claim |
| `relation.target-status-undeclared` | `edit-config` set |
| `revision-log.change-class-required` | `edit-section` set on the revision log |
| `embodiment.consistency` | `run-command` `urzua fix --apply --ids <record>` |
| `embodiment.requires-accepted-status` | `edit-field` set on `Embodiment`, no value (`Embodiment` is a declared header field, so removing it can trip `header.required-fields`; `ADR-15`: never auto-lower) |
| `embodiment.back-pointer-resolution`, `embodiment.back-pointer-decision-accepted` | exempt: edits a source file, no kind for a non-record file yet |
| `embodiment.locator-exists` | `edit-field` set on `Realized-by`, no value |
| `embodiment.locator-promotion-candidate` | exempt: advisory, never auto-promoted |
| `header.field-set-consistency` | `edit-field` remove |
| `field.untrimmed-value` | `edit-field` set, value the trimmed text |
| `field.leading-reserved-indicator` | `edit-field` set, no value (quoting is a choice) |
| `header.field-case-mismatch` | `edit-field` rename, value the declared spelling |
| `header.case-variant-key` | `edit-field` remove, no value (the rule does not say which spelling is right) |
| `narrative-field.stale` | `edit-field` set, no value |
| `type.dir-matches-nothing` | `edit-config` set |
| `identity.collision` | exempt: a renumber, no kind yet (`urzua migrate ids` exists) |
| `type.record-outside-declared-dir` | exempt: a move, no kind yet |
| `type.no-declared-spec` | exempt: a signal to review, never a mandate |
| `header.pointer-field-clean` | `edit-field` set |
| `config.pointer-declaration-missing` | `edit-config` set, value `[]` |
| `config.pointer-field-not-known` | `edit-config` append to `known_fields` |
| `config.pointer-narrative-overlap` | `edit-config` remove from one list |
| `config.header-none-has-no-required-fields`, `config.relation-field-not-known`, `config.known-fields-declaration-missing` | `edit-config` |
| `filename.identity-pattern` | exempt: a rename, no kind yet |
| `status.undeclared-value` | `edit-field` set on `Status`, no value |
| `status.required-field-missing` | `edit-field` set, no value |
| `status.forbidden-field-present` | `edit-field` remove |
| `status.delivery-agreement` | `run-command` `urzua fix --apply --ids <record>` |
| `date.field-valid`, `date.field-order` | `edit-field` set, no value |
| `citation.resolution` | exempt: prose located by line, no kind yet |
| `citation.path-exists` | `edit-section` set on the named reference section |
| `citation.issue-exists` | `edit-section` set on the named reference section, no value (`urzua refs refresh` is not offered: it needs the network and may not apply to a typo) |
| `boundary.published-path-reference` | exempt: edits a published non-record file |
| `section.required` | `edit-section` add |
| `section.shape` | `edit-section` set |
| `index.freshness` | `run-command` `urzua index --apply` |
| `diff.source-requires-record` | `add-record`, `type` when determinable |
| `diff.source-and-records-mixed` | exempt: process advice, no file edit |
| `config.scope-matches-nothing` | `edit-config` set, no value |
| `waiver.matches-nothing` | expired: `edit-field` set on `Expires`, no value; unknown rule: `edit-field` set on `Rule`, no value; no finding matched: `edit-field` set on `Scope`, no value; deleting the waiver is exempt for each, no kind yet |

The exemptions that say "no kind yet" (non-record file edit, rename or move, delete a record, prose by line) are the four capabilities deferred from the first release. Adding `edit-file`, `rename-file`, `remove-record` and a line-located prose edit later is non-breaking under the compatibility rule.

### Relation to other records

- `urzua fix` repairs keep their own entry shape (`record`, `field`, `computed_value`, `tier`, `evidence`). Whether a Tier 3 ambiguous repair entry carries a `suggested_action` (`RFC-8`) is left to the Tier 3 work; this ADR neither builds nor supersedes that line. The `ADR-69` hook reads only `repairs` and is unaffected; it must not be wired to findings.
- `MILE-72`'s `next_action` is defined there as a command string or `null`. It stays a string; it does not reuse these kinds. `MILE-72` is unchanged.

## Reversibility

Adding kinds, `op` values and optional keys is additive for the engine and, under the compatibility rule, for conforming consumers. Renaming or removing any of them, or changing required keys, is breaking for every parser of `check` output, and there is no output version to negotiate it. Consumers that model `kind` as an exhaustive enum would fail on the first new kind, which is why the compatibility rule is part of the decision. The vocabulary should be reviewed before acceptance; the first release uses the five kinds above.

## Consequences

- `Finding` gains `suggested_action`; `SPEC-2` documents the table and the compatibility rule with a revision row, and the acceptance expectations (`MILE-170`) change for every finding that now carries one.
- `RULE_TABLE` gains a per-variant action declaration; a variant without an action or an exemption fails the coverage test.
- `MILE-44` needs a revision row when this is accepted. It widens scope: its What says "at least the rules where a fix is mechanically obvious", this ADR requires every variant to declare an action or a reason. Its Done means changes the `suggestedAction` spelling and replaces "a rule without one omits it" with the declared exemption.
- Cost: about 74 `Finding` construction sites and 53 rules to declare; some actions carry no `value`, so the field points to the right edit and is not always a patch.

## References

- `ADR-64` -- structured in, structured out.
- `ADR-15`, `RFC-8` -- name `suggested_action` on `fix` entries; `SPEC-8` -- the shipped repairs.
- `RFC-3`, `ADR-7` -- agent-native JSON output.
- `ADR-11` -- the waiver record (`waive` is a candidate future kind).
- `MILE-44`, `MILE-171`, `MILE-72`, `BUG-21`, `SPEC-2`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Drafted as Proposed. **Why:** `MILE-44` needs a structured shape that no accepted record defines and that is hard to reverse once agents parse it. | **substantive** |
> | 2026-11-04 | Reworked after independent review: normative per-kind key table, `run-command` replaces `fix-apply`, required `op`, per-variant declaration with reasoned exemptions, compatibility rule, per-rule appendix, `note` dropped, `MILE-72` relation settled. **Why:** the first draft left target keys unspecified, mixed what changes with who performs it, and gave consumers no rule for later kinds. | **substantive** |
> | 2026-11-04 | Re-review amendments: `waive` dropped from the first release, `value` types and `remove` semantics stated, unknown `op` covered, `citation.issue-exists`, `claim.status-agreement`, `embodiment.requires-accepted-status` and `waiver.matches-nothing` declarations corrected. **Why:** the re-review found a kind with no producer and an unstated `value` type. | **substantive** |
