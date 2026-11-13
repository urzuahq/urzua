---
default: patch
closes: []
---

`urzua init` no longer proposes `section.required`, `section.shape`, `section.duplicate`, `section.level-skipped` and `section.order`. Each reads a `sections` key that `init` never writes, so on a new or adopted corpus they ran over no record and reported a clean pass. The notice listing the rules `init` withholds now names them and the key each needs (`sections.required`, `sections.unique_headings`, `sections.no_skipped_levels`, `sections.order`); declare the key on a record type, then enable the rule.
