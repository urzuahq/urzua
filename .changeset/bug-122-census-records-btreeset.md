---
default: patch
---

`census_records` collected every examined candidate's path into a `Vec`, then sorted and deduped the
whole thing once at the end -- for a `Field`-unit rule, a record with many declared fields could
contribute one clone per examined slot before the same set collapsed out of a single bulk sort. Now
uses a `BTreeSet`, deduped incrementally as candidates are examined instead of in one final pass over
the full candidate list. No behavior change.
