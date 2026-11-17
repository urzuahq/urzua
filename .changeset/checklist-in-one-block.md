---
default: minor
closes: []
---

A checklist (`statuses.delivery.items`) is declared in one block. A type that declares `items` no longer has to list the evidence field (`Realized-by` by default) in `required_fields` or `known_fields`, and an incomplete declaration is refused naming the missing key and printing the smallest block that loads. `docs/reference/checklist.md` shows a complete configuration, a record with a met, an unmet and a waived item, the evidence forms accepted and rejected, and what `urzua fix` does and does not change; a test loads its block and record. An evidence clause error lists the categories, a locator that is a record identifier is told a record is cited by its path, and a cited record whose tier does not count names `resolving_tiers` and where it is declared. A new evidence category `record` cites a record by its path and counts where a tier declares `evidence: record`.

Two defaults change. `status.delivery-agreement`, `status.done-items-shape` and `status.waived-item-stale` are on by default wherever a type declares `statuses.delivery.items`, and `status.required-field-missing` and `status.forbidden-field-present` are on by default (at `error`) wherever a type declares a `statuses.require` or `statuses.forbid` entry. A configuration that names a rule keeps the level it gives; a configuration that declared `require` or `forbid` entries without naming the rules will start to report. A configuration that declares none of these entries reports exactly as before. `urzua doctor` warns of a `require` or `forbid` entry whose rule the configuration turns off.

A finding about a required field with no value says whether the field is `missing from the header` or `empty`, names the values the type declares for it in the form `field.value-not-declared` uses, and carries them as the suggested action's value.
