---
default: minor
closes: []
---

`urzua explain --types` carries, for each type that declares them, the statuses a gate by type name reserves for a group (`gated`, no logins), the values a field declares (`field_values`), the condition sentences in place of their count (`conditions`), the revision-log classes the type declares (`revision_log_classes`) and a `checklist` marker with its section, the engine's evidence categories, the tier the configuration declares for each and the resolving tiers; it prints each array of scalars and each flat object on one line. `urzua explain --type <name>` shows a type's `checklist`: the engine's evidence categories, the tier each is declared to compute to with its line, and whether that tier is listed in `resolving_tiers`. `template` says whether its file exists when the command runs and is printed whether or not it does. The summary's `conditions` is now a list of sentences, not a count.
