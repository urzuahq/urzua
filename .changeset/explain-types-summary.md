---
default: minor
closes: []
---

`urzua explain --types` carries, for each type that declares them, the statuses a gate by type name reserves for a group (`gated`, no logins), the values a field declares (`field_values`), the condition sentences in place of their count (`conditions`), the revision-log classes the type declares (`revision_log_classes`) and a `checklist` marker with its section, the evidence categories whose tier the configuration declares and the resolving tiers; it prints each array of scalars on one line. `urzua explain --type <name>` shows a type's `checklist`: the engine's evidence categories, the tier each is declared to compute to with its line, whether that tier is listed in `resolving_tiers`, and one sentence on how a locator is written. `template` says whether its file exists when the command runs and is printed whether or not it does. The summary's `conditions` is now a list of sentences, not a count.
