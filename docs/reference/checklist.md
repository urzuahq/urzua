# A checklist whose state is computed

A record type can declare a checklist: a section of the record whose list items each end with the
evidence that meets them, or with a waiver. `urzua` then computes whether the record is finished from
the items, and `check` reports a `Status` that disagrees. The mechanism answers "is this record done"
without a person ticking a box. It is a mechanism, not a workflow: you choose the type, the section
and every status word.

This page is the whole recipe. The configuration block and the record below are loaded by a test, so
they cannot drift from the code.

## The configuration

Declare the checklist under the type's `statuses.delivery.items`, and require the section it reads.
This is a complete type: copy it under `record_types:` in `.urzua/config.yaml` and rename what you like.

<!-- test:config -->
```yaml
  task:
    dir: docs/task
    prefix: TASK
    required_fields: [Status, Kind, Date, Author, Stable-Id]
    header_shape: yaml-frontmatter
    statuses:
      values: [Open, InProgress, Done, WontDo]
      success: [Done]
      failure: [WontDo]
      initial: [Open]
      transitions: {Open: [InProgress, Done, WontDo], InProgress: [Open, Done, WontDo], Done: [InProgress]}
      delivery:
        delivered_tiers: [Implemented, Verified]
        unevidenced_tiers: [Not started]
        not_done: [Open, InProgress]
        set_status: Done
        items:
          section: Acceptance
          clause_separator: " — "
          evidence_marker: "evidence:"
          waiver_marker: "waived:"
          partial_status: InProgress
          waiver_categories: [{name: not-applicable}]
    sections: {from: headings, depth: 2, required: [{name: Acceptance}]}
```

Nothing else is needed:

- The three rules that judge a checklist, `status.delivery-agreement`, `status.done-items-shape` and
  `status.waived-item-stale`, are on as soon as a type declares `items`. A configuration that names one
  under `rules:` keeps the level it gives; `status.delivery-agreement` set to `off` is refused at load,
  because it is the rule that holds a status to the items.
- The type does not list an evidence field such as `Realized-by` in `known_fields`. Each item carries its
  own evidence; a header field is read only by a `delivery` that declares no `items`.

What each key means:

| Key | What it does |
|---|---|
| `delivered_tiers` | Evidence tiers that mean the work exists. At least one item must resolve at one of them for the record to count as delivered or partly delivered. |
| `unevidenced_tiers` | Tiers that mean nothing was delivered. May be empty (`[]`). |
| `not_done` | The statuses that say the work is not finished. |
| `set_status` | The `success` status `urzua fix` writes when every item is met or waived. |
| `items.section` | The heading whose list items are the checklist. The type must require it under `sections.required`. |
| `items.clause_separator` | The text before a clause marker. |
| `items.evidence_marker`, `items.waiver_marker` | The words that start an evidence clause and a waiver clause. Neither may be a prefix of the other. |
| `items.partial_status` | The status a partly met record is held at and repaired to. It must be in `values` and in `not_done`. |
| `items.waiver_categories` | The reasons an item may be waived. A category may require a cited record (`cite: required`) or an expiry date (`expires: required`). |
| `items.resolving_tiers` | Tiers at which an item's own evidence counts. Omitted means `delivered_tiers`. |

The smallest declaration that loads is shorter still (no `transitions`, no `initial`, no waiver
categories). A declaration that leaves out a key is refused at load, naming the key and printing that
smallest block:

<!-- test:load-error -->
```text
record type 'task' declares an invalid `statuses`: `statuses.delivery.items.partial_status` is missing. The smallest checklist declaration that loads, as the record type's keys (replace the names with its own):
statuses:
  values: [Open, InProgress, Done]
  success: [Done]
  delivery:
    delivered_tiers: [Implemented]
    unevidenced_tiers: []
    not_done: [Open, InProgress]
    set_status: Done
    items:
      section: Acceptance
      clause_separator: " — "
      evidence_marker: "evidence:"
      waiver_marker: "waived:"
      partial_status: InProgress
sections: {from: headings, depth: 2, required: [{name: Acceptance}]}
```

## A record

A record under `docs/task/` with one item met, one unmet and one waived:

<!-- test:record -->
```markdown
---
Status: InProgress
Kind: feature
Date: 2026-11-17
Author: kai
Stable-Id: 01JTASK000000000000000001
---
# 1 - Export the report

## Acceptance

- The report is written as JSON. — evidence: code:src/lib.rs, test:tests/export.rs
- The report is signed.
- Windows is supported. — waived: not-applicable -- the service runs on Linux only
```

- **Met.** The first item ends with an evidence clause: the separator, the marker, then locators. Every
  locator must name a tracked file that exists, and the tier they compute (`code:` is `Implemented`,
  `test:` is `Verified`) must be one the type counts.
- **Unmet.** The second item has no clause, so it is not met.
- **Waived.** The third item is excused with a reason from `waiver_categories`: `waived: <category> --
  <reason>`. A waived item is listed in every `check` report, never hidden, and `status.waived-item-stale`
  warns once its reason has gone (the item gains evidence, a cited record is missing or finished, or an
  expiry date has come).

With one item met, one unmet and one waived, the record is partly delivered, so `InProgress` is the
status that agrees. `Open` would be reported (`urzua fix --apply` moves it to `InProgress`), and `Done`
would be an error naming the unmet item.

## The evidence an item accepts

An evidence clause is a comma-separated list of `category:path` locators and nothing else.

| Written | Accepted |
|---|---|
| `code:src/lib.rs` | Yes. A file; the category `code` computes `Implemented`. |
| `test:tests/export.rs` | Yes. `test` computes `Verified`. |
| `code:src/lib.rs#parse_report` | Yes. A symbol scope inside the file; `#L10-L20` is a line range. A scope needs a pin to be judged: add `@sha256:<64 hex>` (`urzua fix --pin --apply` writes it). |
| `code:src/lib.rs, test:tests/export.rs` | Yes. The tier is the highest category present. |
| `spec:docs/spec/export.md` | A file, yes, but `spec` computes `Specified`, which counts only when `Specified` is in `items.resolving_tiers`. |
| `record:docs/adr/ADR-1-use-x.md` | A record cited by its path, under the category `record`; see below. |
| `4f2a9c1` (a commit hash) | **Rejected.** A commit is not a locator. Cite the file the commit changed. |
| `#512` (a pull request number) | **Rejected.** Cite the file that carries the change. |
| `see the pull request` (free text) | **Rejected.** Write the explanation outside the clause. |
| `spec:ADR-1` (a record identifier) | **Rejected.** The text after the category is read as a path, and a record is cited by its path. |

A rejected clause is a `status.done-items-shape` finding that lists the categories, and the item counts
as unmet. The categories are `spec`, `code`, `test` and `record`; the engine fixes the names, and
`embodiment.tiers` in `.urzua/config.yaml` maps each to the tier your repository calls it.

### Citing a record

Evidence is often a decision record rather than a file of code. Cite it by its path under the category
`record`, and declare the tier that category computes to, which no default tier does:

<!-- test:embodiment -->
```yaml
embodiment:
  drift: Drift detected
  tiers:
    - {state: Verified, evidence: test, claims_built: true}
    - {state: Implemented, evidence: code, claims_built: true}
    - {state: Decided, evidence: record}
    - {state: Specified, evidence: spec}
  none: Not started
```

and add the tier to the type's `items.resolving_tiers` (`[Implemented, Verified, Decided]`). Without
that line the finding says the locator resolves only to a tier that is not in `resolving_tiers`, lists the
tiers that count, and names the key to change: `record_types.task.statuses.delivery.items.resolving_tiers`
in the configuration. A `record:` locator that is not a record of
a declared type does not resolve. To count a cited record only once it is accepted, declare a `kind` with
`cited_records: success` (see `SPEC-3`, "Done-means items").

## What `urzua fix` does and does not change

`urzua fix` (a dry run) lists what it would change; `urzua fix --apply` writes it with a revision-log
row. It moves a status **up**, never down:

- `Open` with every item met or waived becomes `set_status` (`Done`).
- `Open` with some item met and another unmet becomes `partial_status` (`InProgress`).
- A record already at a `success` status is never reopened. One with an unmet item is a
  `status.delivery-agreement` error naming each item and why, and `fix` prints a notice: add the
  evidence, waive the item, or reopen the record by hand.
- It never writes evidence, never ticks an item, and never edits the item text.

```sh
urzua check
urzua fix
```

## Converting records one at a time

A record whose items carry no clause is not judged until it reaches a finished status. While it is
`Open` or `InProgress`, items with no evidence and no waiver report nothing; only a `Done` record with
an unmet item is an error. A repository can declare the checklist today, leave its existing records
alone, and add clauses to each record as it is worked.
