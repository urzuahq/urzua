---
default: minor
---

Adds an optional `fields` option to `pointer.target-status`: a list of declared pointer/narrative field names the rule checks. A dependency field such as `Implements` can now be judged separately from a lineage field such as `Derives-from`, whose Superseded target is permanent history rather than a stale reference. Omitting `fields` checks every declared field, as before. Fields left out are not counted in the rule's population, and a `fields` entry that names no declared pointer or narrative field is a config error rather than a silently ignored setting.
