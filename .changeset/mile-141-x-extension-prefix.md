---
default: minor
closes: []
---

Adds an opt-in `allow_extension_prefix` option to `header.field-set-consistency`. A repository that
declares `allow_extension_prefix: true` on that rule can annotate a record with a local field whose
header key begins `x-` (in any case) without declaring the field in `known_fields`. Undeclared keys
without the prefix are still findings, so a typo in a real key is still caught. The option is off by
default, so `check` reports exactly what it reported before for any repository that does not set it.

```yaml
rules:
  header.field-set-consistency:
    level: warn
    allow_extension_prefix: true
```

The option is accepted only on `header.field-set-consistency`; a config that declares it on any other
rule fails to load.
