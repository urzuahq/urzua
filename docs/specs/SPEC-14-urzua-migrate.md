---
Stable-Id: 01M6R3NK5EEDKBJSNGJHTYE0QZ
Version: '0.15'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/migrate.rs@sha256:f3fe484ef1e800cafe0af678597102d62432a421e15f176981aacf4e8d20622f, code:rust/crates/urzua-core/src/migrate.rs@sha256:a8b30583f9658b1ffdcd5920fb2bd08adf9e41bd55d406ac94aaaaf5f59fe768, test:rust/crates/urzua-core/src/migrate.rs@sha256:a8b30583f9658b1ffdcd5920fb2bd08adf9e41bd55d406ac94aaaaf5f59fe768, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:5ab61b95fd5864a41566dee0d902fac000dca3552f90316331ecf90617c3f9b0
Author: beauwilliams
Subject: '`urzua migrate ids`/`urzua migrate schema`/`urzua migrate config` -- backfilling stable IDs, previewing schema changes and rewriting a config to the current schema version.'
Implements: ADR-21, ADR-22, ADR-77
Parent: SPEC-1
---
# SPEC-14 — `urzua migrate`

## Purpose

Migrates the corpus itself — three genuinely different operations under one verb, not one command
with mode flags: backfilling stable identifiers onto existing records, previewing what a
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
