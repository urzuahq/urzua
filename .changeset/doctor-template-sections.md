---
default: minor
closes: []
---

`urzua doctor` gains the check `template-sections`: for each record type whose configuration requires sections, it reports each required section the type's template (`.urzua/templates/<type>.md`) has no heading for, as a warning that names them, and reports clean when the template has them all. A type with no template, or none requiring sections, has nothing to compare.
