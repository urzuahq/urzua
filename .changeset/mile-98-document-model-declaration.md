---
default: minor
---

A record type in `.urzua/config.yaml` can now declare its document model with three optional keys,
`identity` (`from: filename` plus a `pattern`), `fields` (`from: prefix-lines` or
`yaml-frontmatter`) and `sections` (`from: headings`, with optional `depth` and `items`). The
declaration is validated when the config loads: an unknown source, an unknown key, an out-of-range
`depth`, a blank `pattern` or a key with no value is an error naming the record type and key. No rule
reads the declaration yet, so declaring it changes no finding; a config without it loads exactly as
before.
