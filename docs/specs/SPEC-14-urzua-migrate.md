---
Stable-Id: 01M6R3NK5EEDKBJSNGJHTYE0QZ
Version: '0.20'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/migrate.rs@sha256:f3fe484ef1e800cafe0af678597102d62432a421e15f176981aacf4e8d20622f, code:rust/crates/urzua-core/src/migrate.rs@sha256:a8b30583f9658b1ffdcd5920fb2bd08adf9e41bd55d406ac94aaaaf5f59fe768, test:rust/crates/urzua-core/src/migrate.rs@sha256:a8b30583f9658b1ffdcd5920fb2bd08adf9e41bd55d406ac94aaaaf5f59fe768, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:3441046fd3446240de4164e0cc9dc8d73b8e9c05893d6c43b227b109c4216b5f, code:rust/crates/urzua-cli/src/commands/migrate_filenames.rs@sha256:f5b90c25b3a3f58307b7befa3de6fb093a60167082d89086c77ba7203bdf2fbd, code:rust/crates/urzua-core/src/migrate_filenames.rs@sha256:a7126c7f473ca6f9b85a4fe8402e189c9883f9b9ba2d86dc8437ddec1714b7d0, test:rust/crates/urzua-core/src/migrate_filenames.rs@sha256:a7126c7f473ca6f9b85a4fe8402e189c9883f9b9ba2d86dc8437ddec1714b7d0, test:rust/crates/urzua-cli/tests/migrate_filenames_integration.rs@sha256:91cda512bee82530a47e780cf3138f07b46af243f91e4e63130f8b710cbf4ddb
Author: beauwilliams
Subject: '`urzua migrate ids`/`urzua migrate filenames`/`urzua migrate schema`/`urzua migrate config` -- backfilling stable IDs, renaming record files to the declared filename convention, previewing schema changes and rewriting a config to the current schema version.'
Implements: ADR-21, ADR-22, ADR-77
Parent: SPEC-1
---
# SPEC-14 — `urzua migrate`

## Purpose

Migrates the corpus itself — four genuinely different operations under one verb, not one command
with mode flags: backfilling stable identifiers onto existing records, renaming record files to
the filename convention their type declares, previewing what a
newly-required field would break before it's ever added to config, and rewriting the config from the
previous schema version to the current one.

## `urzua migrate ids`

Backfills a `Stable-Id` header field (ULID, ADR-21) into every record lacking one, retaining the
filename's current number as the display number unchanged — cross-references keep resolving by
filename-derived number exactly as they do today; nothing about `pointer.resolution` changes.
Dry-run by default; `--apply` writes.

**Encoding**: ULID — 128-bit, time-ordered, lexicographically sortable, collision-free without
coordination (the property concurrent agent branches need). Backfilled IDs are stamped with the
migration run's own time, not each record's original git history — ADR-3 §6's proposal to derive
backfill timestamps from git history is a named, tracked simplification, not silently dropped,
since nothing in the schema currently reads or sorts by a stable ID's embedded timestamp.

`urzua-id::StableId::generate()` wraps `ulid::Ulid::generate()`.

**Output (shipped, ADR-46)**:

```json
{
  "status": "ok",
  "apply": true,
  "missing": ["docs/adr/0001-x.md"],
  "results": [
    { "record": "docs/adr/0001-x.md", "outcome": "applied" }
  ]
}
```

`results` is present only on `--apply` (a dry run has nothing to report per-file beyond `missing`
itself). Each result's `outcome` is `applied`, `skipped` (no header-shaped region found -- carries an
`error` message), or `failed` (the write itself failed -- also carries an `error` message). Exit code
is 1 if any result is `failed`, 0 otherwise -- `skipped` does not affect it, since it isn't a write
failure. `status` is `partial-failure` (not `ok`) when any result is `failed`, so the report and the exit
code agree. `--apply` takes the lock `fix --apply` takes (`.urzua/cache/fix.lock`), re-reads each record
right before writing and fails that record, writing nothing, if it changed since it was read, and
replaces each file through a temporary file and a rename, so it cannot overwrite a concurrent edit or
leave a truncated record. `migrate config` replaces the config the same way.

## `urzua migrate filenames`

Renames every record file whose name does not match its type's filename convention, and rewrites the
citations of the renamed paths (`MILE-134`; `RFC-46` was rejected, so a corpus in another shape is
renamed rather than tolerated, `ADR-36`). Dry-run by default; `--apply` writes, as `migrate ids` does.

- **Which files.** The records `filename.identity-pattern` reports: a file directly in a type's `dir`
  whose name yields no record id. A file already on the convention is never touched.
- **The new name** is read from the type's declared `identity.pattern`, never a hardcoded shape. The
  leading digits of the old name are the number, the rest (after any `-`, `_` or space) is the slug, and
  the name is `PREFIX-NUMBER-slug.md`, the number keeping its written zero-padding when the pattern
  accepts it, otherwise widened or narrowed to the nearest width it accepts, never changing the number's
  value (`5-x.md` becomes `ADR-0005-x.md` and `00012-z.md` becomes `ADR-0012-z.md` under `\d{4}`). A name
  that already carries the prefix but is mispadded for the pattern (`ADR-5-x.md`) keeps its number. A type declaring no pattern gets `TYPE-NNNN-slug.md` as written. The directory, number and
  slug never change, so the record id is the same and the file is untouched apart from its citations.
  `Stable-Id` is not read or written, so a rename keeps the pairing the decider gate uses (`ADR-89`); a
  record without one is named in a `stable-id-missing` notice, because a diff can pair the rename
  with the old file only through it, and `migrate ids --apply` should run first.
- **Citations.** Every path token ending `.md` in a record or any other tracked markdown file is read
  as the file's own relative link (`./x.md`, `../adr/x.md`, `x.md`), a repository path (`docs/adr/x.md`)
  or a root path (`/docs/adr/x.md`); one that leads to a renamed file has its file name replaced and
  nothing else (a `#anchor`, the directory part and the surrounding text stay). A same-named file in
  another directory is not a citation. A citation in a tracked file that is not markdown (code,
  configuration, text) is not edited: it is reported in `unrewritten`.
- **Skipped, never overwritten.** A file with no leading number, a number too large to read, a name the
  declared pattern rejects, a new name that is already a file or another rename's new name, and a
  number that is already another record's of the type or that two renames would both take (`5-x.md` and
  `0005-y.md`: one record id on two files) are listed in `skipped` with a `reason` (`no-number`,
  `number-too-large`, `pattern-rejects`, `target-exists`, `number-collision`) and left alone. A
  collision skips every file of the clash, since renaming one would hide it. Nothing is overwritten.
- **Evidence pins.** A pin (`path@sha256:...`) on a file whose text the run rewrites goes stale. The
  command never re-pins, because a re-pin carries a reason a person attests: each such pin is listed in
  `stale_pins` (`file`, `line`, `locator`, `changed_file`) with a `stale-pins` notice, and `urzua fix
  --rebaseline` re-pins them.
- **Writes.** `--apply` takes the `fix` lock, rewrites citations first (each file re-read and replaced
  atomically, failing without writing if it changed since it was read), then renames; a failed rewrite
  stops the renames, so a renamed file never keeps citations of its old name. It uses the filesystem, not
  `git mv`: git sees a delete and an add that its rename detection pairs, and nothing is staged. A second
  applied run finds nothing to rename, reports empty lists and writes nothing.
- **A failed rename.** If a rename fails after the rewrites, the rewritten citations name the new path
  of a file still at its old one: the rename is reported `failed` (exit 1, `partial-failure`) and the
  rewrites stay. Running the command again heals it: the file is still off the convention, so it is
  renamed, and its citations already read the new name.

```json
{
  "status": "ok",
  "apply": true,
  "renames": [
    { "record_type": "adr", "from": "docs/adr/0001-first.md", "to": "docs/adr/ADR-0001-first.md" }
  ],
  "rewrites": [
    { "file": "docs/adr/ADR-0002-second.md", "line": 9, "from": "./0001-first.md", "to": "./ADR-0001-first.md" }
  ],
  "stale_pins": [],
  "unrewritten": [
    { "file": "notes.txt", "line": 1, "reference": "docs/adr/0001-first.md", "renamed_to": "docs/adr/ADR-0001-first.md" }
  ],
  "skipped": [
    { "from": "docs/adr/README.md", "reason": "no-number", "message": "..." }
  ],
  "results": [
    { "path": "docs/adr/ADR-0002-second.md", "action": "rewrite", "outcome": "applied" },
    { "path": "docs/adr/0001-first.md", "action": "rename", "outcome": "applied" }
  ]
}
```

`results` is present only when something was written; each `outcome` is `applied`, `failed` (carries an
`error`) or `skipped` (a rename stopped by a failed rewrite). Exit code is 1 if any result is `failed`,
and `status` is then `partial-failure`.

## `urzua migrate schema --report --field <Name>`

The only implemented tier of a three-tier design (report, waiver-assist, apply). Lists every record
lacking a real value for `Name` — using the same blank/placeholder/pending/present classification
`field.quality` already applies to declared required fields, against a field that isn't declared
yet — without touching config or any record. Read-only, same as `check`. Errors with exactly what's
missing if invoked without both `--report` and `--field <Name>`, rather than doing nothing silently.

## `urzua migrate config`

Rewrites `.urzua/config.yaml` (or the file `--config` names) from the previous schema version `N-1` to
the current `N`, in place (`ADR-77`; the window, the version-1 upgrade and the bump rule are in
`SPEC-3`'s "Supported schema versions"). It edits lines rather than re-serializing, so comments and
layout survive, then checks that the edited text loads exactly as the in-memory upgrade `check` performs
would have, and writes nothing otherwise: a file the upgrade cannot make valid (a removed
`header_shape`, say) is left exactly as it was and the command exits 2 with the reason.

- **No flags.** There is no dry run: `check` already shows what an `N-1` config would become, as one
  warning, and the rewrite is a plain file edit under version control.
- **A config at `N`** is left alone: `outcome: "nothing-to-do"`, `changes: []`, exit 0. A second run
  after a rewrite is exactly this.
- **A config outside `N-1` and `N`** is never read further. It is a could-not-run (exit 2) carrying the
  same `schema_version_error` object `check` gives, whose `migration` names the release to install and
  the command to run with it.

```json
{
  "status": "ok",
  "file": ".urzua/config.yaml",
  "outcome": "migrated",
  "from": 1,
  "to": 2,
  "changes": [
    { "action": "dropped", "key": "record_types.adr.header_layout", "reason": "no rule reads it any more; ..." },
    { "action": "added", "key": "rules.field.quality", "value": "error", "reason": "a version-1 build ran this rule at this severity; ..." }
  ]
}
```

`outcome` is `migrated` or `nothing-to-do`. Each change is one key: `action` is `added`, `changed`,
`renamed`, `moved` or `dropped`; `key` is a dotted path from the top of the file; `to` names the new
path of a rename or a move; `value` is what an added or changed key now holds; `reason` says why.

## What's deliberately not built

- **`migrate schema --assist-waivers`** — generating a draft waiver (placeholder `Reason`, human
  edits before commit) for every record a newly-required field would break. Blocked on deciding
  whether a generated draft preserves the review property RFC-11's waiver model depends on, or
  whether only a human-authored waiver (SPEC-10) does.
- **`migrate schema --apply`** — delegating to `urzua fix` (SPEC-8) for whichever fields happen to
  be tool-writable. Structurally, almost none are: most newly-required fields (e.g. `Reviewers`)
  need human authorship, the same eligibility question SPEC-8 already answers for `Embodiment`.
  `fix`'s own apply mode existing (SPEC-8) makes this delegation technically possible now, but the
  eligibility question — which fields, if any, would ever qualify — is still open.

## References

- ADR-21 — the ULID encoding decision and its named git-history-timestamp simplification.
- ADR-22 — report-mode-only, and why the other two tiers aren't built yet.
- ADR-77, RFC-49 — the support window and the migration path `migrate config` is.
- ADR-3 — the stable-identity/display-number separation `migrate ids` backfills onto existing
  records.
- SPEC-8 — `fix`'s eligibility test, which `migrate schema --apply` would need to delegate to.
- SPEC-10 — the waiver record shape `--assist-waivers` would need to respect.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec, bundling `migrate ids` and `migrate schema` since SPEC-1 already treats them as one feature area under one verb. **Why:** MILE-77 found `migrate` documented only as two separate ADRs while comparable-complexity command areas (`check`, `init`) had specs. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-09-11 | Added `migrate ids`'s output contract (`missing`/`results`, BUG-20, ADR-46) -- previously undocumented, since the command printed plain-text prose rather than JSON. `--apply` now also exits 1 on a real write failure, matching `fix --apply`'s own signal for the same shape of outcome. | **substantive** |
> | 2026-11-05 | Bumped to `0.5`. Added `Realized-by` naming the implementing code and the tests that exercise this spec: the `migrate` unit tests in `urzua-core/src/migrate.rs` and the `migrate_ids_*` tests in `check_integration.rs`. **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`; the tests run this repository's own `.urzua/config.yaml`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/commands/migrate.rs, code:rust/crates/urzua-core/src/migrate.rs, test:rust/crates/urzua-core/src/migrate.rs, test:rust/crates/urzua-cli/tests/check_integration.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Status changed from Accepted to Shipped and bumped to `0.6`. **Why:** maintainer decision of 2026-11-05 to ship the specs whose implementation is verified; `Embodiment: Verified` with `Realized-by` naming the implementing code and tests. | **structural** |
> | 2026-11-05 | Bumped to `0.7`. Added `urzua migrate config` and its output contract. **Why:** `ADR-77` / `MILE-34` -- a config on the previous schema version needs a command that rewrites it. | **substantive** |
> | 2026-11-05 | Bumped to `0.8`. `migrate schema --report` classifies the candidate field with the same classifier `field.quality` uses, including a type's `template_placeholders` vocabulary and declared `pending` values, and reads `.urzua/templates/` to do so. **Why:** Phase 0 hardening of the core model: the preview disagreed with the rule for template-vocabulary types. | **substantive** |
> | 2026-11-05 | Bumped to `0.9`. `migrate ids --apply` reports `status: partial-failure` when a write failed, takes the `fix` lock, re-reads before writing, and writes atomically; `migrate config` writes atomically. **Why:** review of the CLI found `status: ok` printed beside exit 1, and a write from the text read at session start with no lock overwriting a record edited in between. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 4 locators pinned, 4 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/migrate.rs@sha256:772ecad047bc, rust/crates/urzua-core/src/migrate.rs@sha256:58ae3634352f, rust/crates/urzua-core/src/migrate.rs@sha256:58ae3634352f, rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b833 Reason: Pinned the files as they are now after checking each against the record it evidences: declaration.rs and config.yaml still declare pointer_fields and narrative_fields per type (MILE-90), config.yaml still requires Subject on specs with checks.yml gating it (MILE-91), document_model.rs, both section.rs files, schema_version.rs and config.yaml still read the declared identity, fields and sections (MILE-98), .git-hooks/pre-push, the Makefile and checks.yml still run urzua check (MILE-99), waiver.rs, rules/waiver.rs and waiver_stale_integration.rs still implement the waiver type SPEC-10 describes, audit.rs and the own-config and check integration tests still implement SPEC-11, new.rs, new_record.rs and new_integration.rs still implement SPEC-12, explain.rs, both graph.rs files and the own-config test still implement SPEC-13, both migrate.rs files and check_integration.rs still implement SPEC-14, and doctor.rs and its tests still implement SPEC-15. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Bumped to `0.10`. The `code:` and `test:` locators of `Realized-by` are now hash-pinned by `urzua reverify --pin-only` (the `Pinned` row above) and, where the record stated `Drift detected`, its `Embodiment` was recomputed (the tool-authored row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97` section 10: a claim is renewed only over pinned code and test evidence, so the corpus is pinned once before expiry is enabled. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` f893cb99b833 -> 8538aedb7492. Reason: the tests in check_integration.rs now name real tracked files; the behaviour this record relies on is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.11`. The `Realized-by` pin on `check_integration.rs` was re-baselined (the `Re-baselined` row above) after its tests were changed to name real tracked files; no behaviour this spec describes changes. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` 8538aedb7492 -> 5ab61b95fd58. Reason: check_integration.rs: the only change is in pre_commit_hook_stops_a_planted_bad_header_with_the_real_binary, which now symlinks the urzua binary into the fixture instead of copying it (a copy raced a forking test thread and failed with ETXTBSY under load); the test's setup, planted defect and assertions are otherwise unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/migrate.rs` 772ecad047bc -> f3fe484ef1e8. Reason: split run_ids_inner, moving per-record work into backfill_stable_id (migrate.rs); behaviour unchanged, outputs byte-identical **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/migrate.rs` 58ae3634352f -> a8b30583f965. Reason: Read the diff of each re-pinned file against main. core/migrate.rs: the Notice literal gains detail: None; no behaviour change. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.14`. The pins this spec's `Realized-by` holds on files this change touched were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. | **structural** |
> | 2026-11-09 | Bumped to `0.15`. Re-pin rows in this revision log were reworded to name who re-verified, matching the tool's wording since 2026-11-09; hashes, reasons and dates are unchanged. **Why:** Maintainer decision, 2026-11-09. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` 5ab61b95fd58 -> a60c9c61b94c. Reason: Read the diff of each pinned file since the previous pin. rust/crates/urzua-cli/tests/check_integration.rs: adds one regression test that a claim file with an upper-case .MD extension under claim_paths is read; nothing else changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.16`. Evidence pins re-baselined for this change; the specification text is unchanged. **Why:** a revision-log entry that adds a structural change bumps `Version`. | **structural** |
> | 2026-11-13 | Bumped to `0.17`. Added `urzua migrate filenames` and its output contract. **Why:** `MILE-134`: a corpus whose record files are named in another shape is renamed to the type's declared convention, with its citations rewritten, rather than by a one-off script. | **substantive** |
> | 2026-11-13 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 4 locators pinned, 4 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/migrate_filenames.rs@sha256:39efa2e76028, rust/crates/urzua-core/src/migrate_filenames.rs@sha256:fc13122703fe, rust/crates/urzua-core/src/migrate_filenames.rs@sha256:fc13122703fe, rust/crates/urzua-cli/tests/migrate_filenames_integration.rs@sha256:fc77b1cc5faf Reason: Pinned the new migrate filenames code and tests this record evidences, as written in this change **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-13 | Bumped to `0.18`. `migrate filenames` skips a record-id clash (`number-collision`), narrows zero-padding as well as widening it, fixes a prefixed but mispadded name, lists the evidence pins its rewrites make stale (`stale_pins`) instead of re-pinning them, and states what a failed rename leaves and that a rerun heals it. **Why:** review of the first build of `MILE-134`. | **substantive** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/migrate_filenames.rs` 39efa2e76028 -> f5b90c25b3a3, `rust/crates/urzua-core/src/migrate_filenames.rs` fc13122703fe -> a7126c7f473c, `rust/crates/urzua-cli/tests/migrate_filenames_integration.rs` fc77b1cc5faf -> 91cda512bee8. Reason: Read the diff of each pinned file. core migrate_filenames.rs: number-collision skip reason and check, narrowing zero-padding, prefixed mispadded names, stale_pins detection. cli migrate_filenames.rs: stale_pins in the report with a stale-pins notice, a doc comment on write_plan and a unit test of the failed-rename path. The integration test file gained four tests for these. The documented behaviour is unchanged where not stated in the new SPEC-14 text. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` a60c9c61b94c -> bc2e870defc6. Reason: Read the diff of each re-pinned file against main and the claim still holds. check_integration.rs: the ADR-0001 waiver fixture's H1 gains the citation-fixture marker text and every_header_reading_rule_reports_a_header_none_type_as_out_of_scope declares the new rule's required options; no assertion changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Bumped to `0.19`. Evidence pins re-baselined for this change (the `Re-baselined` row above); the specification text is unchanged. **Why:** a revision-log entry that adds a structural change bumps `Version`. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` bc2e870defc6 -> 3441046fd344. Reason: check_integration.rs changed by test hardening only: the three tests that need chmod to deny access now start with common::permission_test_can_run, so they skip loudly as root and fail in CI instead of passing silently, and 3 not-exit-0 assertions now pin the exact exit code; no assertion of the recorded behaviour was weakened **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Bumped to `0.20`. The test files this spec's evidence pins were hardened (exact exit codes, isolated git, visible root skips); the pins are re-baselined in the row above and the specified behaviour is unchanged. **Why:** the review's test findings; a re-baseline row is structural, so the version moves with it. | **structural** |
