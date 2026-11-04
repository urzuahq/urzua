---
default: minor
---

`citation.resolution` accepts the optional `skip_revision_log` flag that `citation.path-exists`
already has: when `true`, each record's revision log (the table under its type's declared
`revision_log` identity) is not read, so append-only history that cites an id as it was is not
reported as a dangling citation. It is `false` by default, so behaviour is unchanged unless it is
declared.
