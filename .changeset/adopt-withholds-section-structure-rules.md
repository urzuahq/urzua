---
default: patch
closes: []
---

`urzua init` (adopt) no longer proposes `section.duplicate`, `section.level-skipped` and `section.order`. Each reads a `sections` key that adopt never writes, so on an adopted corpus they ran over no record and reported a clean pass. The notice listing the rules adopt withholds now names them and the key each needs (`sections.unique_headings`, `sections.no_skipped_levels`, `sections.order`).
