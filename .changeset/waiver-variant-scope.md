---
default: minor
closes: [MILE-168]
---

A waiver may name the finding it excuses. `Variant` is the finding's action variant and `Key-path` the configuration key its `suggested_action.target.key_path` names; a waiver that names one covers only the findings it matches, so a waiver written for one finding in `.urzua/config.yaml` no longer hides every other finding of its rule there. A waiver scoped to the configuration file that names neither is reported by `waiver.matches-nothing` (variant `waiver-config-scope-too-broad`) while it stays in effect; every other waiver, and any waiver that names neither field, works as before. `urzua waive --from-report` groups drafts by rule, file, variant and key path, writes `Variant` and `Key-path` where the waiver type lists them in `known_fields`, reports `variant` and `key_path` on each draft, and skips only the finding an existing narrowed waiver names. A waiver type that lists neither field drafts as before.
