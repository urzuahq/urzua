# The record model

Urzua has no built-in idea of an ADR, an RFC or an incident review. A repository declares its record
types in [`.urzua/config.yaml`](config.md), and urzua validates, orders and repairs against that
declaration. This page describes the parts a declaration is made of.

## A record

A record is one markdown file in the directory its type declares (`dir`). It has:

- **A header**: YAML frontmatter at the top of the file. Its keys are the record's fields.
- **A title**: the first `#` heading.
- **Sections**: `##` headings and the text under them.
- **Optionally a revision log**: a table the tool appends to when it rewrites a field. See below.

A record's directory must hold it directly. Records in nested directories belong to no type;
`type.record-outside-declared-dir` reports them rather than ignoring them.

## Record type

A type is a name (`adr`, `milestone`, `waiver`, ...) with a directory and a set of declarations about
its records. The ones you will use most:

| Declaration | What it says |
|---|---|
| `dir`, `prefix` | Where records live; the id prefix (`ADR`) used in file names and when referring to a record (`ADR-12`). |
| `header_shape` | `yaml-frontmatter` (the default) or `none` for records with no header block. |
| `required_fields` | Fields every record must carry with a real value. |
| `known_fields` | The complete set of fields a record may carry. A field outside it is reported. |
| `statuses` | The status vocabulary and its lifecycle. |
| `pointer_fields` | Fields whose value names another record; each is resolved. |
| `sections` | Headings every record must have, optionally with a required shape. |
| `revision_log` | How the revision log is found and what its columns are called. |

The full list, with every key, is in [the config file](config.md).

## Identity

A record is identified three ways, each for a different job:

- **The id** is `PREFIX-N`, read from the file name. `ADR-12-use-postgres.md` is `ADR-12`. It is what
  humans write in prose and in pointer fields. The default file name convention is
  `TYPE-NNNN-slug.md`; a type can declare its own pattern with `identity`.
- **The display number** is the number in the id.
- **The stable id** is a ULID stored in the `Stable-Id` header field. It never changes, even if a
  record is renamed or renumbered. `urzua new` writes one, and `urzua migrate ids` backfills them.

## Status and lifecycle

A type may declare a status vocabulary:

```yaml
statuses:
  values: [Proposed, Accepted, Rejected, Superseded]
  success: [Accepted]            # statuses that count as a successful outcome
  failure: [Rejected, Superseded]
  initial: [Proposed]            # statuses a record may be filed in
  transitions:
    Proposed: [Accepted, Rejected]
    Accepted: [Superseded]
```

With that declared, `status.undeclared-value` reports a status outside `values`,
`status.transition-allowed` reports a move the table does not allow (judged against the base of a
`check --base` run), and `status.unreachable-state` reports a declared value nothing can reach.
`urzua new` starts a record at the first value of `values`.

Statuses can also drive required fields: `require` and `forbid` name fields that must (or must not)
be present once a record is in given statuses, such as a `Deciders` field required once a decision is
`Accepted`.

## Relations between records

A field named in `pointer_fields` holds record ids, comma separated. `pointer.resolution` reports an
id that matches no record. `Supersedes` and `Superseded-by` are checked for reciprocity by
`relation.supersession-reciprocity` where the type declares them under `relation_fields`.

The relationships form a graph. `urzua graph` prints every edge, each tagged `kind` (`pointer` or
`narrative`) and `dangling`. Which fields count as edges is declared, so a different organization can
point an incident review at a runbook as easily as an ADR at an RFC.

A second kind of relation, `relation_pairs`, declares two fields as inverses of each other (for
example `Depends-on` and `Blocks`), and can mark a pair as a sequencing relation, which `urzua
roadmap` uses to order work.

## Evidence: `Embodiment` and `Realized-by`

A record can claim that something was built. Two fields carry the claim:

- **`Realized-by`** lists what realizes the record, as `category:path` locators separated by commas:
  `code:src/lib.rs, test:tests/db.rs`.
- **`Embodiment`** states how far the claim goes (for example `Specified`, `Implemented`,
  `Verified`, `Not started`).

`check` computes the state the locators support, by the strongest category present, and reports a
record whose stated `Embodiment` disagrees. `urzua fix` rewrites the field to the computed value.
A locator can be pinned to a content hash so that a change to the cited file is reported as drift.
The vocabulary of states and which category lifts a record to which state is yours to declare under
`embodiment`. See [waivers and drift](waivers-and-drift.md).

## Sections

`sections` declares which headings a record needs (`required`), with optional aliases, nesting and a
shape such as a table with a minimum number of rows. `section.required` and `section.shape` judge
them. A section list in a milestone-style record can also carry per-item evidence; the config keys
for that are under `statuses.delivery.items` in [the config file](config.md).

## Revision log

When urzua rewrites a field (`fix --apply`, `fix --rebaseline --apply`, `reverify --apply`) it
appends a row to the record's revision log saying who, what and why, and classifies it. The log is a
table found by a marker line (default `**Revision log**`) or a heading. A type whose log is
`required` refuses a write on a record that has none, rather than writing somewhere it cannot be
audited. A hand-written row needs a `Class` that is one of the declared classes (default
`substantive` or `structural`); `revision-log.change-class-required` reports one without.

## Suppression: the waiver type

One record type may declare `role: suppression`. Its records are waivers: reviewed exceptions to a
rule for a path. See [waivers and drift](waivers-and-drift.md).
