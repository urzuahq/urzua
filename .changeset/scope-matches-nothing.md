---
default: major
---

**Breaking: each `population` in `rules_executed` gains `out_of_scope`.** It counts candidates the
*configuration* cannot reach, as distinct from ones the corpus has not written yet — two states that
previously arrived as one number despite having opposite fixes. Always present, including as `0`.

Adds `config.scope-matches-nothing` (MILE-106), an opt-in rule that reports a declared rule whose
candidates the configuration does not reach. On a corpus whose filenames carry no type prefix,
`identity.collision` is reported and `revision-log.change-class-required` is not, though both examined
zero of two candidates: the first is a configuration that cannot match, the second is a corpus that
has not written a revision log yet.
