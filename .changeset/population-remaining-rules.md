---
default: major
---

**Breaking: `rules_executed` entries no longer carry `records_examined` or `scope`.** Every rule now
reports a `population` instead — what it was eligible to examine and what it judged, in a named unit
(`record`, `field`, `record-type`, `path`, `claim`). The old number held three different denominators
under one name: `field.quality` reported 1041 over a 315-record corpus, correct all along and
labelled as something it was not. A consumer reading `records_examined` should read
`population.examined` and check `population.unit` before comparing it to a record count.

The report also gains a top-level `records_read_by_any_rule`: how many distinct records some rule
actually reached a verdict about, as a union rather than a sum across units. `files_examined` says
what was read off disk; this says what was judged.

`check` and `audit` no longer report `not-run` on the grounds that no record-scoped rule ran. What
gets checked is what the repository declares, so a thin configuration earns a clean exit — and says
`records_read_by_any_rule: 0` where a reader sees it, rather than reading as a checked corpus.
