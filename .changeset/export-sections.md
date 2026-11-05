---
default: minor
closes: [MILE-115]
---

`urzua export --format json` now emits each record's sections. For a record type that declares
`sections`, every record carries `sections` (each with `name`, `heading`, `line`, `end_line` and
`content`, an alias resolving to the declared name) and `preamble`, the text before the first
section. A type that declares no `sections` gets `null` for both, so "no declared document model"
is distinct from "declared, none present" (`[]`).
