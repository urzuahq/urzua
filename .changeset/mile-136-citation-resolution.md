---
default: minor
---

New opt-in rule `citation.resolution`: a decision ID cited anywhere in a record's prose (for example
`ADR-12` in a Context paragraph) must resolve to a record, or the rule reports an error naming the file
and line. It reads the identifiers carrying a record type's configured prefix, compares numbers by value
so `ADR-0012` and `ADR-12` are one id, and skips the header (which `pointer.resolution` governs) and
fenced code blocks. Declare it under `rules:` at `warn` or `error` to enable it; the run reports how many
citations it examined, so a corpus that cites nothing is distinguishable from a clean one.
