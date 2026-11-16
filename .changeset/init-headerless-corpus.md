---
default: minor
closes: [MILE-265]
---

`urzua init` adopts a corpus whose records have no header block, such as the common `0001-title.md`
ADR style with `## Status`/`## Context`/`## Decision` sections: it proposes `header_shape: none` with the
`##` sections at least 80% of the files carry as `sections.required`, so `urzua check` reports a record
missing one, and its `notes` name what the config cannot check (duplicate numbers without a type prefix,
the `## Status` text, prose references). It withholds rules that would examine nothing on such a type,
excludes `index.md` and numberless files beside numbered records as well as `README.md`, and in a
repository with no records names `urzua init --types adr` in its message and `next_action`. `urzua doctor`
with no config names a step that works instead of always "run `urzua init`". `urzua migrate ids` reports
`written` and says why a headerless type got nothing. The rename finding for a numberless file names
`exclude_files` as the alternative.
