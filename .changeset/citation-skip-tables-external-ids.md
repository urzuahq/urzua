---
default: patch
---

`citation.path-exists` accepts an optional `skip_tables` list of markers (`**Revision log**`): a line
containing one, and the pipe table after it, are not read, so append-only history that names a path as
it was is not reported as a missing file. The list is empty by default, so behaviour is unchanged unless
it is declared. `config.scope-matches-nothing` no longer warns about the citations of an `external_ids`
entry of `citation.resolution`: they are counted as eligible and unexamined instead of out of scope, so a
declared input is not reported as a misconfiguration.
