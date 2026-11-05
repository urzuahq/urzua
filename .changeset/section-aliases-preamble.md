---
default: minor
closes: []
---

A required section can declare `aliases`, other headings that satisfy it, and `preamble: true`, which
reads a marked Y-statement from the text before the first `##` heading when its section is absent.
`urzua check` reports each alias in use as an info notice, and the shape check judges an aliased
section or the preamble the same way it judges the named section.
