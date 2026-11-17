---
Stable-Id: 01M7MQTY327WDMATPQ3X4XV7ZK
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 340 — urzua explain --types carries what a type declares to gate, constrain and log, and which evidence a checklist accepts

## What

`urzua explain --types` carries, for each record type, the declarations an agent needs at session start and today has to ask `urzua explain --type <name>` for: the statuses a gate by type name reserves for a group, the values a field declares, the sentence of each declared condition, the revision-log classes a type declares, and a checklist marker naming its section and the evidence categories its items may cite. `urzua explain --type <name>` shows the same checklist with each category's declared tier. Every added value is a declaration copied from the configuration or produced by an existing engine function, worded as a declaration.

## Why

An adopting repository reported, after setting its configuration up by hand and then running the command: "`--type <name>` had everything it needed and nothing wrong. The summary (`--types` with no `--full`), which is what an agent reads at session start, omitted exactly the facts it had got wrong." It moved decision records straight to a status that a gate reserves for a group (the summary shows `path_gates` but not gates declared by type); it used a revision-log class that is not declared; it did not know a field takes declared values; it did not know a condition existed (the summary showed only `"conditions": 1`, where `--type` has the sentence, which the reporter called the best part of the output); and it did not know a type has a checklist section with an evidence clause. It also reported that the summary is pretty-printed with one array element per line, which is most of its bytes (4,013 bytes for six types there); that nothing says which evidence categories a checklist item accepts after its evidence marker (it tripped on a category that does not exist and on one needing a path); and that `template` shows a default path without saying whether the file exists. The reporter's list also asked which required fields `urzua new` leaves for the agent to fill; that is not part of this milestone, because another open change alters what `urzua new` writes and adds a `to_fill` list.

## Done means

- `urzua explain --types` prints, for a type that declares them and only then, `gated` (each status an `authority.gates` entry naming the type lists under `to`, with the group that entry names under `by`, and no logins), `field_values`, `conditions` as the `declared: when ...` sentences that `--type` prints in place of a count, `revision_log_classes` from `revision_log.classes`, and `checklist` (the section, each evidence category whose declared tier is named, the categories no tier declares, and the resolving tiers); a type that declares none of them has none of those keys.
- `urzua explain --type <name>` prints `checklist` for a type that declares `statuses.delivery.items`: the section and evidence marker with their lines, the engine's evidence categories, for each category the tier an `embodiment.tiers` entry declares with its line and whether that tier is listed in `resolving_tiers` (the built-in tiers and the type's `delivered_tiers` marked as defaults where the configuration declares none), and one sentence from the engine's locator grammar; no resolution rule is described.
- `urzua explain --types` prints each array of scalars on one line; `--full`, `--type` and every other command keep their formatting; the sizes of the summary on this repository's configuration, on the three-type fixture and on a fixture declaring every added key are stated in the pull request and bounded in `explain_types_integration`.
- `urzua explain --type <name>` prints `template` with `exists_when_run`, whether the conventional template file was there when the command ran.
- A test on a configuration that declares each addition and one that does not shows the key present and absent, and a test shows every added summary value equal to the value `--type` prints for the same configuration.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Planned`. **Why:** an adopting repository reported on 2026-11-17 that the summary omitted gates by type, declared field values, condition sentences, revision-log classes and the checklist, which `--type` shows; decided by the builder on 2026-11-17, pending maintainer review. | **structural** |
