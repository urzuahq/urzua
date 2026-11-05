---
default: minor
---

Add the `revision-log.version-bump-required` rule and the `revision_log.version_bump` declaration. A record type declares
a version field and the revision-log change classes that require it to change; on a `--base` run, a changed record that
adds a row of one of those classes while its version equals the merge base's copy is reported. The merge base's copy of
each changed record is read by the CLI. No type has it by default, and enabling the rule with no type declaring it is a
configuration error.
