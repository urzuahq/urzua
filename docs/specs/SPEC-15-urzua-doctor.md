---
Stable-Id: 01M6R3NK5E1D23JRXBPTP54NSG
Version: '0.21'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/doctor.rs@sha256:64faee47e0b165641ae9c88489c962bf0ec50673fb822afc63be36601f6edf50, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs@sha256:44fea1626a547a6d9226f09c5e9cd116aad2dfe384a082aaeb8eec2c74f22af5, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:5ab61b95fd5864a41566dee0d902fac000dca3552f90316331ecf90617c3f9b0
Author: beauwilliams
Subject: '`urzua doctor` -- diagnosing whether checks are actually wired to CI/hooks, not just present.'
Implements: —
Parent: SPEC-1
---
# SPEC-15 — `urzua doctor`

## Purpose

Reports on the tool's own configuration and invocation health — is there a config, does it parse, is
it wired into a real gate — as a separate command, deliberately. `check` validates records; whether
`check` *itself* is correctly invoked is a different question with a different failure mode, and
conflating them risks the same shape of bug as collapsing blank/placeholder/pending into one state
(SPEC-2). One source implementation's checker ran clean for weeks — not because the corpus was
clean, but because nothing ever invoked it.

Pulled out of SPEC-3 (Configuration) into its own spec: `doctor` is a real, standalone feature area
with its own output shape and its own bug history (BUG-4), not merely a detail of how config is
structured.

## Output contract (shipped)

Stdout is JSON, unconditionally (ADR-23) — this is a live fix (BUG-4): `doctor` originally printed
`[OK]`/`[WARN]`/`[ERROR]` plain-text lines, the one command that hadn't caught up with every other
command's output contract.

```json
{
  "status": "ok | warn | error",
  "checks": [
    { "check": "config-exists", "status": "ok", "message": "..." }
  ]
}
```

(`notices` -- see below -- is omitted here since it's empty in this example; present, non-empty, only
when there's a real observation to report.)

`status` is the worst severity across `checks` (`error` beats `warn` beats `ok`). Exit codes: `2` if
`.urzua/config.yaml` doesn't exist at all (run `urzua init` first); `1` if the config fails to parse,
or any check reports `error`; `0` otherwise — a `warn`-only report still exits `0`, since only an
error blocks. That 2/1/0 split is carried on an explicit, non-serialized field (ADR-46) precisely
*because* the first two cases both report the same `status: error` -- `status` alone can't tell them
apart, so the exit code doesn't try to derive it from `status`. `notices` (ADR-46) is omitted when
empty, same as every other command's report.

## Checks, by id (shipped)

| `check` id | Meaning |
|---|---|
| `config-exists` | `.urzua/config.yaml` is present. `error` if missing (short-circuits, exit 2). |
| `config-parses` | The config parses with no unrecognized keys (`deny_unknown_fields` — a typo'd key is a hard parse error, not a silently-ignored one, and so is a `harness.guardrails` name the build does not ship). `error` if not (short-circuits, exit 1). Unrecognized entries inside the `harness` table still load; the message then counts them and `harness-config` names them. |
| `harness-outside-writes` | `warn` when `harness` enables a guardrail and `outside_writes` is undeclared: a write into a linked worktree of the repository at a commit with no gates is then not judged by this repository's gates; recommends `shared` (`ADR-89` amendment of 2026-11-09, `SPEC-3`). |
| `harness-installed` | `warn` when `harness` declares an adapter, a guardrail or an instructions file and `.urzua/harness.lock` is missing or the installed bundle has drifted from it. A `harness` block that declares none of the three (only `outside_writes` and `allow_paths`, as `urzua init` writes it) installs nothing, so a missing lock is not reported. |
| `record-types-declared` | At least one `[record_types.*]` entry exists. `error` if the map is empty — `check` would never examine anything. |
| `rules-enabled` | At least one rule is enabled (level other than `off`). `error` if none is: every rule is opt-in (`ADR-53`), so `check` would report not-run and examine nothing (`BUG-174`). |
| `record-type-dir` | One check per declared type: does `dir` actually exist on disk. `error` if not — this fires today for `waiver` in this very repo, correctly, since no waiver record has been created yet (MILE-59) and its directory was deliberately never pre-created. |
| `record-type-required-fields` | One check per declared type with an empty `required_fields`. `warn`, not `error` — a type with no required fields is unusual, not necessarily wrong. |
| `ci-wired` | Some workflow under `.github/workflows/` runs `urzua check` (a call in a command position: a `#` comment and an `echo`/`printf` line are not a call) or uses urzua's reusable `actions/urzua-gate` action (`MILE-180`); a repository's own wrapper script or make target is not recognised, which runs the checker for the caller. `warn` if not — a check that exists but is never run reports nothing to anyone. |
| `authority-decider-table` | When `authority` declares gates: `ok` naming the value of `decider_table`, or `warn` when it is undeclared, since the table is then read from the pull request's own head and a pull request can add its author to a decider group in the change it asks to have approved; the message recommends `base` (`SPEC-3`, `ADR-89` amendment of 2026-11-09). |

| `hook-wired` | One check per local hook layer (`pre-commit`, `pre-push`), carrying `layer` and a machine-readable `state` (`MILE-179`, `ADR-71`). The hooks directory is `core.hooksPath` (relative to the working tree root) when set, else the repository's own `hooks`. `state` is `not-installed`, `installed-running-urzua` (the file runs `urzua <subcommand>`, by a path ending in `urzua` or by a variable assigned one; a mention in a comment or an `echo` does not count), `installed-not-running-urzua`, or `installed-not-executable` (git ignores a hook without the execute bit; not checked on non-Unix hosts). `ok` only for `installed-running-urzua`; the other states are `warn`, since a local layer is a convenience and only CI is the guarantee. `error` when the hooks directory or hook file cannot be read: not knowing is not the same as absent. Hook-manager configuration (lefthook, pre-commit framework) is not read; a manager that installs real hook files, or sets `core.hooksPath`, is seen. |

## What's aspirational, not yet built

SPEC-3's original doctor language named more than what exists today; the gap is named here rather
than compounded:

- **Which config resolved and from where** — `doctor` reads the global `--config` when given (and
  names that file in `config-exists`), else the default `.urzua/config.yaml`; it doesn't yet report
  the resolution path as a field of its own.
- **Which rules are off** — per-rule levels exist (`ADR-53`, every rule opt-in) and `doctor` errors
  when none is enabled (`rules-enabled`), but it does not list which individual rules are off or what
  level each has.
- **Whether the binary is invoked from CI, a hook, or ad hoc** — `doctor` checks whether the CI
  *workflow file* runs `urzua check` as text; it doesn't detect the actual
  invocation context of the specific run currently executing `doctor` itself.

## References

- BUG-4 — the plain-text-output defect this spec's shipped shape fixes.
- ADR-23 — the stdout-JSON-always contract `doctor` now follows.
- SPEC-2 — `check`, the command whose own correct invocation `doctor` verifies without duplicating.
- SPEC-3 — configuration, `doctor`'s original home before this spec split out.
- MILE-80 — configurable rule severity, which "which rules are off" depends on.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec, split out of SPEC-3. **Why:** doctor is a real, standalone feature area (its own output shape, its own bug history) that had been folded into configuration's spec by default rather than by a deliberate call; MILE-77's review named it as deserving its own spec. Written the same day BUG-4 (plain-text output) was found and fixed, so the shipped shape reflects the fix, not the pre-fix behavior. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-09-11 | Added `notices` to the documented shape and explained why the 2/1/0 exit-code split lives on a separate field rather than being derived from `status` (ADR-46). | **substantive** |
> | 2026-09-17 | Config moves from `.urzua/config.toml` to `.urzua/config.yaml` (`ADR-52`, shipped in the same change). The described mechanism changes, not just its rendering. | **substantive** |
> | 2026-11-02 | Bumped to `0.5`. Added the `rules-enabled` check and corrected the "which rules are off" paragraph, which still said there is no per-rule config (stale since `ADR-53`). **Why:** `doctor` exited 0 for a config where `check` exits 2 not-run (`BUG-174`). Also noted that config load now rejects an invalid record-type `prefix` or `dir` (`BUG-173`), surfaced through `config-parses`. | **substantive** |
> | 2026-11-04 | Bumped to `0.6`. `ci-wired` also accepts a workflow that uses urzua's reusable `urzua-gate` action. **Why:** a workflow wired in through the action never spells `urzua check`, so doctor would have reported a wired repository as unwired (`MILE-180`). | **substantive** |
> | 2026-11-04 | Bumped to `0.7`. Added the `hook-wired` check and its `layer` and `state` fields. **Why:** `ADR-71` makes wiring verifiable and `MILE-179` builds it; `doctor` examined the config and workflows but not hooks, so a repository could not verify its own hook. | **substantive** |
> | 2026-11-05 | Bumped to `0.8`. Added `Realized-by` naming the implementing code and the tests that exercise this spec: `doctor_reads_the_repositorys_own_config_and_reports_the_unwired_hook_as_a_warning`. **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`; the tests run this repository's own `.urzua/config.yaml`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/commands/doctor.rs, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs, test:rust/crates/urzua-cli/tests/check_integration.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Status changed from Accepted to Shipped and bumped to `0.9`. **Why:** maintainer decision of 2026-11-05 to ship the specs whose implementation is verified; `Embodiment: Verified` with `Realized-by` naming the implementing code and tests. | **structural** |
> | 2026-11-05 | Bumped to `0.10`. `doctor` reads the global `--config`, as every other command does, instead of always reporting on the default file. **Why:** review of the CLI found `urzua doctor --config x.yaml` reported on the default config, a silent no-op. | **substantive** |
> | 2026-11-06 | Bumped to `0.11`. `ci-wired` and `hook-wired` read a call, not a word: a `#` comment and an `echo`/`printf` line are dropped, the call is `urzua <subcommand>` (a path ending in `urzua`, or a variable assigned one), and `make records` is no longer recognised; the harness checks gain `harness-legacy-hook`. **Why:** Phase 0 hardening from a second adopter switchover -- a hook whose only mention of urzua was a comment read as wired, and the engine carried one repository's make target. | **substantive** |
> | 2026-11-07 | Bumped to `0.12`. `config-parses` no longer says "no unrecognized keys" while the harness table holds unrecognized entries (it counts them and points to `harness-config`); an unknown `harness.guardrails` name is now a config load error, so this check reports `error` for it. **Why:** `ADR-89` amendment of 2026-11-07. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/doctor.rs@sha256:46c027154ca9, rust/crates/urzua-cli/tests/record_types_own_config_integration.rs@sha256:7caa23975490, rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b833 Reason: Pinned the files as they are now after checking each against the record it evidences: declaration.rs and config.yaml still declare pointer_fields and narrative_fields per type (MILE-90), config.yaml still requires Subject on specs with checks.yml gating it (MILE-91), document_model.rs, both section.rs files, schema_version.rs and config.yaml still read the declared identity, fields and sections (MILE-98), .git-hooks/pre-push, the Makefile and checks.yml still run urzua check (MILE-99), waiver.rs, rules/waiver.rs and waiver_stale_integration.rs still implement the waiver type SPEC-10 describes, audit.rs and the own-config and check integration tests still implement SPEC-11, new.rs, new_record.rs and new_integration.rs still implement SPEC-12, explain.rs, both graph.rs files and the own-config test still implement SPEC-13, both migrate.rs files and check_integration.rs still implement SPEC-14, and doctor.rs and its tests still implement SPEC-15. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Bumped to `0.13`. The `code:` and `test:` locators of `Realized-by` are now hash-pinned by `urzua reverify --pin-only` (the `Pinned` row above) and, where the record stated `Drift detected`, its `Embodiment` was recomputed (the tool-authored row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97` section 10: a claim is renewed only over pinned code and test evidence, so the corpus is pinned once before expiry is enabled. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` 7caa23975490 -> 44fea1626a54. Reason: Read the diff of each pinned file since the old pin and the claim still holds. record_types_own_config_integration.rs: the milestone fixture's Done means is now a list item instead of a prose sentence (the repository config checks Done means as items), and the helper that swaps in items replaces that item; no assertion changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Bumped to `0.14`. The pin this spec's `Realized-by` holds on `record_types_own_config_integration.rs` was re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. **Why:** a revision-log entry that adds a structural change needs a `Version` bump (`revision-log.version-bump-required`). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` f893cb99b833 -> 8538aedb7492. Reason: the tests in check_integration.rs now name real tracked files; the behaviour this record relies on is unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Bumped to `0.15`. The `Realized-by` pin on `check_integration.rs` was re-baselined (the `Re-baselined` row above) after its tests were changed to name real tracked files; no behaviour this spec describes changes. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` 8538aedb7492 -> 5ab61b95fd58. Reason: check_integration.rs: the only change is in pre_commit_hook_stops_a_planted_bad_header_with_the_real_binary, which now symlinks the urzua binary into the fixture instead of copying it (a copy raced a forking test thread and failed with ETXTBSY under load); the test's setup, planted defect and assertions are otherwise unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/doctor.rs` 46c027154ca9 -> 4d8d0d22c116. Reason: split doctor::run into config_shape_checks, workflows_invoke_checker, ci_wired_check and report_from (doctor.rs); behaviour unchanged, outputs byte-identical **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/doctor.rs` 4d8d0d22c116 -> 64faee47e0b1. Reason: Read the diff of each pinned file since the old pin and the claim still holds. doctor.rs: hooks_dir's git closure uses urzua_io::git::GitRun with the same arguments, working directory and 'could not run git' wording; nothing else changed; behaviour unchanged, outputs byte-identical. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Bumped to `0.18`. The pins this spec's `Realized-by` holds on files this change touched were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. | **structural** |
> | 2026-11-09 | Bumped to `0.19`. Adds the `harness-outside-writes` check. **Why:** `ADR-89` amendment of 2026-11-09 (outside writes), accepted by the maintainer: a linked worktree at a commit without gates was an unjudged place to write. | **substantive** |
> | 2026-11-09 | Bumped to `0.20`. `harness-installed` is not reported for a `harness` block that declares no adapter, guardrail or instructions file. **Why:** `urzua init` now writes `harness: {outside_writes: shared}` into every fresh configuration, and it was followed by a warning that a harness it never asked for was not installed. | **substantive** |
> | 2026-11-09 | Bumped to `0.21`. Adds the check `authority-decider-table`, which warns when `authority` declares gates and `decider_table` is undeclared. **Why:** `ADR-89` amendment of 2026-11-09 (decider table), accepted by the maintainer. | **substantive** |
