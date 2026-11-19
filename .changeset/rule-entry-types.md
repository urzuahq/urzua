---
default: minor
closes: [MILE-351]
---

A rule entry under `rules:` can now list the record types it judges with `types:`, for example `status.children-agreement: {level: error, types: [milestone]}`. A record of another type is not judged by that rule: its findings are not reported, and the rule's population counts it as `out_of_scope` and in a new `out_of_types` counter. Without `types:` a rule judges every type, exactly as before. A type name that is not declared, or an empty list, stops the configuration from loading, and so does `types:` on a rule that judges the configuration, files, a diff or the run rather than records. A listed type the rule can never judge is named in one `rule-types-never-judged` notice. `types:` applies the same way to `check --base`, the editor hook, `fix` and `urzua explain --rule`, and works alongside `scope: changed`.
