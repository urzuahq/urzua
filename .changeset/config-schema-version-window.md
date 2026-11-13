---
default: minor
closes: []
---

Introduce a support window for the config `schema_version`, and `urzua migrate config` to bring a config from the previous version up to date.

Each release reads the current `schema_version` and the one before it. A config at the previous
version is upgraded in memory before any rule runs, so `check` keeps working through an upgrade; the
only difference is one new warning, `config.schema-version-outdated`, on the config file, naming the
version found and offering `urzua migrate config` as its suggested action. Under `fail_on: warning`
that warning fails a strict repository until the file is migrated.

`urzua migrate config` rewrites a config at the previous version to the current one, in place, keeping
its comments. It lists every key it added, changed, renamed, moved or dropped, makes no change on a
second run, and leaves a file it cannot make valid exactly as it was. A version-1 record type that
omits `header_shape` is refused with an error naming the type: version 1 read it as `blockquote`,
which is no longer read, and the upgrade will not quietly change it to `yaml-frontmatter`.

A config outside that window is no longer a bare "unrecognized `schema_version`" error. The run exits
2 with a `schema_version_error` object (`found`, `supported`, `current`, `migration`) beside the usual
`error` sentence, where `migration` names the oldest release that still reads the version and the
command to run with it. `SPEC-3` gains a "Supported schema versions" table and the rule for when
`schema_version` must change.
