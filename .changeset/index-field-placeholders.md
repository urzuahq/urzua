---
default: minor
closes: []
---

An index `format` for `index.freshness` and `urzua index` can use placeholders for declared header
fields, such as `{Status}` and `{Date}`, beside `{id}`, `{file}` and `{title}`. A field placeholder is
filled from the record and checked against the committed index; a placeholder that names no declared
field of the type fails the configuration load.
