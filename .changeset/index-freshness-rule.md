---
default: minor
---

New opt-in rule `index.freshness`: a committed index of one record type, either a whole file or a list
between `begin`/`end` marker lines in a README, must list exactly the records. Declare each artifact under
the rule's `indexes` option with its `path`, record `type` and entry `format` (`{id}`, `{file}` and
`{title}` placeholders). `check` reports a record with no entry, an entry naming no record, a duplicated
entry, a stale file name or title, a missing file and a removed marker pair, and the rule's census says how
many artifacts it compared.
