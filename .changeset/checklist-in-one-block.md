---
default: minor
closes: []
---

A checklist (`statuses.delivery.items`) is declared in one block (`MILE-335`). A type that declares `items` no longer has to list the evidence field (`Realized-by` by default) in `required_fields` or `known_fields`, and an incomplete declaration is refused naming the missing key and printing the smallest block that loads. `docs/reference/checklist.md` shows a complete configuration, a record with a met, an unmet and a waived item, the evidence forms accepted and rejected, and what `urzua fix` does and does not change; a test loads its block and record. An evidence clause error lists the categories, a locator that is a record identifier is told a record is cited by its path, and a cited record whose tier does not count names `resolving_tiers` and where it is declared (a category no tier declares is told to declare one under `embodiment.tiers`). A new evidence category `record` cites a record by its path and counts where a tier declares `evidence: record`.

Two defaults change (`MILE-326`, `MILE-335`). The rules that judge a checklist are on by default wherever a type declares `statuses.delivery.items`, and the two status-requirement rules wherever a type declares a `statuses.require` or `statuses.forbid` entry:

- A repository that declares `items` and named only `status.delivery-agreement` now also gets `status.done-items-shape` at `error` and `status.waived-item-stale` at `warn`.
- A repository that declared `require` or `forbid` entries without naming `status.required-field-missing` or `status.forbidden-field-present` now reports at `error`.
- A configuration that names a rule keeps the level it gives, and one that declares none of these entries reports exactly as before.
- `urzua init` proposes none of the five rules in either mode, as before: `init --types` writes `statuses:` blocks but none with `require`, `forbid` or `items`, and adopt mode writes no `statuses`. An entry added later reports without a `rules:` entry.
- A clean run whose only rules apply by declaration is `ok`, and such a rule counts toward `ok` only once it has examined a record; a run in which none did stays `not-run`. A rule named explicitly under `rules:` still counts toward `ok` when it read no record, as on 0.4.7.

`urzua doctor` warns of a `require` or `forbid` entry whose rule the configuration turns off (MILE-326).

A finding about a required field with no value says whether the field is `missing from the header` or `empty`, names the values the type declares for it in the form `field.value-not-declared` uses, and carries them as the suggested action's value (`MILE-327`).
