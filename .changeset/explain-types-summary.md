---
default: minor
closes: []
---

`urzua explain --types` carries, for each type that declares them, each gate that names the type (`gates`, one entry per gate, no logins), the values a field declares (`field_values`), the condition sentences in place of their count (`conditions`), the revision-log classes the type declares (`revision_log_classes`) and a `checklist` marker with its section, the engine's evidence categories, the tier the configuration declares for each and the resolving tiers; it prints each array of scalars and each flat object on one line. `urzua explain --type <name>` shows a type's `checklist`: the engine's evidence categories, the tier each is declared to compute to with its line, beside the `resolving_tiers`. `template` says whether `urzua new` can read its file when the command runs (`readable_when_run`) and is printed whether or not it does. The summary carries one note that these are declarations. The summary's `conditions` is now a list of sentences, not a count.
