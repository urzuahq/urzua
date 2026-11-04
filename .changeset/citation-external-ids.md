---
default: minor
---

`citation.resolution` accepts an optional `external_ids` list of prefixed ids that are not records of
the repository, so a public standard number that shares a record prefix (`RFC-4122` against an `RFC`
prefix) is no longer reported as a dangling citation. Each entry is an exact id or an inclusive
numeric range (`RFC-4000..4999`) under a declared prefix; a malformed entry or one under no declared
prefix is a config error. A matching citation is counted as out of scope in the rule's population, and
any other dangling id is still reported.
