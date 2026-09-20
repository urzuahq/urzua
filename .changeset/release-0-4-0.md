---
default: minor
---

Governance is configuration. Every rule is declared and opt-in, and the configuration is YAML.

**Breaking: the config is now `.urzua/config.yaml` at `schema_version: 2`.** A `0.3.0` config does
not load. Every rule a repository wants must be named in the `rules` table -- an absent rule is off,
not on -- and `urzua init` writes a starter table for an adopted corpus. A rule taking options
(`pointer.target-status`, `claim.status-agreement`, `narrative-field.stale`) fails to load without
them rather than running inert. Unknown rule names and misplaced options are load-time errors.

**A rule's severity is declared, not compiled in.** `level: warn` or `level: error` per rule, so a
repository decides what blocks its own CI.

**`dir` means that directory, not that subtree** (`RFC-35`). A type's records sit directly in its
`dir`; nested directories belong to no type, and the new `type.record-outside-declared-dir` reports a
record-shaped file that falls between types rather than dropping it silently.

New rules, all opt-in: `type.dir-matches-nothing`, `type.record-outside-declared-dir`,
`identity.collision`, `embodiment.locator-exists`, `field.pending`, `claim.status-agreement`, and
`pointer.target-status` split out of `pointer.resolution` so a dangling reference and a reference to
a superseded record can carry different severities.

**The engine no longer knows this repository's vocabulary.** `narrative-field.stale` reads a declared
`terminal_statuses` instead of a table keyed on record-type names, so the rule applies to a corpus
whose types are named anything at all.

**`urzua new` finds a record's number rather than assuming its position**, so a hyphenated type
prefix parses and the command stops reissuing the same number.

**`urzua init` proposes one type per directory holding records**, writes the type prefix that
directory's filenames actually use, and declines to propose rules the corpus gives it no way to
satisfy.

**A run that establishes nothing is reported as `not-run`, not `ok`.** A configuration with no rules
declared, or only rules that read the configuration or the path inventory, no longer exits 0 as
though the corpus had been checked.

**A record the tool cannot read stops the run.** An unreadable or missing tracked record previously
dropped out of the corpus, leaving a smaller `files_examined` and a clean verdict. A deletion staged
in git is still a legitimate absence.

**A path argument narrows what is reported on, not what a reference resolves against.** `check
docs/adr/` no longer reports every reference that points outside the requested path as dangling, and
no longer discards findings about files git does not track.

`claim_paths` is read to any depth, follows symlinks that stay inside the repository, bounds the walk
against cycles, and fails the run rather than skipping a claim it could not read.

Fixes BUG-38, BUG-39, BUG-42, BUG-43, BUG-44, BUG-45, BUG-46, BUG-48, BUG-49, BUG-53, BUG-54, BUG-55, BUG-56, BUG-58, BUG-59, BUG-60, BUG-61, BUG-62, BUG-63, BUG-64, BUG-67, BUG-68, BUG-69, BUG-70, BUG-71, BUG-72, BUG-75, BUG-76, BUG-77, BUG-78, BUG-79, BUG-80, BUG-81, BUG-82, BUG-83, BUG-84, BUG-85 and BUG-86.
