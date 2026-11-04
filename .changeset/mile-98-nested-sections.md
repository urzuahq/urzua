---
default: minor
---

A record type that declares `sections.depth: 2` has its `###` headings read as sections nested under
their `##`, and a required section can declare `level` and `under` to ask for one, for example
`{name: Pros and cons, level: 3, under: Options}`. A level the declared `depth` does not reach fails
the config load. Types that declare no `level` are checked exactly as before.
