---
Stable-Id: 01M6M8A27T8HXBKK1WRB3ZF7YM
Status: Draft
Date: 2026-11-04
Author: beauwilliams
Supersedes / Superseded-by: null
---
# 49 — Config schema versions have a declared support window and a migration path

## Summary

`urzua` reads exactly one config schema version and rejects every other one with a bare
`unrecognized schema_version` error. This RFC proposes that each release declares which schema
versions it reads, that a config outside that window is reported as a structured finding which names
the supported versions and the migration to reach one, and that a schema break which leaves existing
configs behind ships a mechanical upgrade (`urzua migrate config`) instead of no aid at all. It would
amend `ADR-54`'s default ("a break ships no migration aid"), because the reason that default was safe
(a measured migration population of zero) stops being true once a real adopter's config exists.

## Motivation

- **The current behaviour.** `config.rs` holds `CURRENT_SCHEMA_VERSION = 2`, and loading a config
  whose `schema_version` is anything else fails with `UnrecognizedSchemaVersion`: "this build of urzua
  understands version 2". The message names the version urzua reads, but not what changed or how to
  get there, and there is no way to read a version-1 config at all.
- **`ADR-12` and `ADR-52`** made `schema_version` mandatory precisely so a break is loud. A loud break
  is the floor, not the contract: it tells an adopter that their config no longer loads, not what to do.
- **`ADR-54` set the default that a break ships no migration aid**, and argued it from evidence: the
  release that broke the format had one download and the only TOML config was this repository's own,
  converted by hand. Its Consequences say outright that "a break with real adopters behind it is a
  different question" and must be decided again "with numbers rather than instinct".
- **That question now has a date.** Phase 0 ends with urzua governing its own repository at parity
  with a real adopter environment. Phase 1 (`MILE-102`) is that adopter switching over. From then on
  a config urzua does not own exists outside this repository, and the next schema break is the first
  one with a non-zero migration population.
- **Breaks are coming.** Phase 0 alone added `fail_on`, per-type `revision_log`, `embodiment`,
  `extra_dirs`, `statuses.delivery`, `issue_refs` and the rule-option rework in `MILE-174`. Most of that
  was additive, but `MILE-85` removed the blockquote and bold-list header shapes outright and turned
  configs that named them into load errors. A rename of any declared key is a break.
- **`MILE-34`'s Done means** asks that a config on an older schema version be reported with a
  migration path rather than a bare equality reject. Nothing decides what that path is, so it cannot be
  built.

## Proposal

1. **A declared support window.** Each release reads a contiguous set of schema versions, recorded in
   a "Supported schema versions" table in `SPEC-3` (release, versions read, version written). The
   proposed window is the current version plus the one before it (N and N-1). A version older than N-1,
   or newer than N, is outside the window.
2. **Reading N-1.** A config at N-1 loads. It is upgraded in memory to N before any rule runs, so
   rules only ever see the current shape. `check` adds one warning finding,
   `config.schema-version-outdated`, on the config file: it names the version found, the current
   version, and the command that rewrites the file. Under `fail_on: warning` that fails a strict repo
   while leaving an adopter able to run every other check.
3. **Outside the window.** Loading fails as today (exit 2), but the error is structured rather than a
   sentence: `{found, supported: [..], current, migration}`, where `migration` names the oldest
   release that still reads `found` and the command to run with it.
4. **`urzua migrate config`.** It reads an N-1 config and writes the N form in place, preserving
   comments where the YAML library allows and reporting every key it renamed, moved or dropped. A
   second run reports nothing to do. It never reads a version outside the window, so each release
   carries exactly one upgrade step (N-1 to N), not a chain.
5. **When a bump is required.** `schema_version` changes only for a break: a removed or renamed key,
   a changed meaning for an existing key, or a default that changes behaviour for an unchanged config.
   An additive optional key with a default that preserves behaviour is not a break and needs no bump.
   The rule is written into `SPEC-3` next to the table, so a reviewer can apply it to a config change.
6. **`ADR-54`.** The decision that comes out of this RFC amends `ADR-54`: its "no migration aid" rule
   stays for any format with a measured migration population of zero, and this policy applies once that
   population is non-zero. It does not supersede `ADR-54`, whose reasoning this RFC relies on.

## Open questions

- **Is N-1 the right window?** One version keeps the code to a single upgrade step. A wider window
  helps an adopter who skips releases, at the cost of a chain of upgrade steps that each need tests.
- **Is "outdated" a warning or an error?** A warning lets an adopter's CI stay green through an
  upgrade under `fail_on: error`, but a strict repository on `fail_on: warning` fails the moment it
  upgrades the binary. That may be the wanted behaviour, or it may need its own declared level.
- **Should `migrate config` be its own command** or a mode of the existing `migrate` family
  (`migrate ids`, `migrate schema --report`), which `ADR-22` keeps report-only for record schemas?
- **How is the in-memory upgrade tested?** One proposal is a fixture config at every supported version
  in the acceptance corpora, with a case proving that each loads to the same effective config.
- **What counts as the adopter population** that reopens `ADR-54`? Asset downloads, as `ADR-54`
  measured, or the known adopters?

## Non-goals

- Versioning of record files or their headers. That is `RFC-6` and the per-type `Version` field.
- Reading configs from before `schema_version` existed, or the TOML format `ADR-52` removed.
- A general plugin or extension compatibility contract.
- Changing what `schema_version` 2 means today.

## References

- `MILE-34`: the milestone this would let be built.
- `ADR-12`, amended by `ADR-52`: `schema_version` is required and a mismatch is loud.
- `ADR-54`: a break ships no migration aid, the default this would amend.
- `ADR-22`: `migrate schema` stays report-only for record schemas.
- `MILE-85`: the most recent break that turned existing configs into load errors.
- `MILE-102`: the Phase 1 switchover that gives the next break a real migration population.
- `SPEC-3`: where the supported-versions table and the bump rule would live.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Revision log added, 2026-11-05. | **structural** |
