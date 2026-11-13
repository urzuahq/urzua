---
default: patch
closes: []
---

The gate's renderer handles reports over 128 KB (it passed them to `jq` as an argument and failed
with `Argument list too long`, so a corpus with many waived findings lost its pull-request report).
The record-edit hook now judges every finding that needs no git history in a shallow clone and
names the drift rule it could not judge, where it used to allow every edit unjudged; it no longer
suggests a `urzua fix` command for an error the edit itself introduced. `check <path>` reads git
history only for the records in scope, which cuts a one-record check on a few hundred records from
about five seconds to about one. `field.pending`'s built-in match reads `pending` as a whole word,
so a value such as `docs/spending/report.md` is no longer unfinished work. `init` proposes a flat
directory of header-bearing files and flags a proposed directory in which no file has a header.
`new` creates a type's directory for its first record. `doctor` and the hook name urzua commands,
not make targets, and the `filename.identity-pattern` finding names five renames and counts the
rest.
