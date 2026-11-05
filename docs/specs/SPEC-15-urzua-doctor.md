---
Version: '0.9'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/doctor.rs, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
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
| `config-parses` | The config parses with no unrecognized keys (`deny_unknown_fields` — a typo'd key is a hard parse error, not a silently-ignored one). `error` if not (short-circuits, exit 1). |
| `record-types-declared` | At least one `[record_types.*]` entry exists. `error` if the map is empty — `check` would never examine anything. |
| `rules-enabled` | At least one rule is enabled (level other than `off`). `error` if none is: every rule is opt-in (`ADR-53`), so `check` would report not-run and examine nothing (`BUG-174`). |
| `record-type-dir` | One check per declared type: does `dir` actually exist on disk. `error` if not — this fires today for `waiver` in this very repo, correctly, since no waiver record has been created yet (MILE-59) and its directory was deliberately never pre-created. |
| `record-type-required-fields` | One check per declared type with an empty `required_fields`. `warn`, not `error` — a type with no required fields is unusual, not necessarily wrong. |
| `ci-wired` | Some workflow under `.github/workflows/` contains `urzua check`, `make records` or a reference to urzua's reusable `actions/urzua-gate` action (`MILE-180`), which runs the checker for the caller. `warn` if not — a check that exists but is never run reports nothing to anyone. |

| `hook-wired` | One check per local hook layer (`pre-commit`, `pre-push`), carrying `layer` and a machine-readable `state` (`MILE-179`, `ADR-71`). The hooks directory is `core.hooksPath` (relative to the working tree root) when set, else the repository's own `hooks`. `state` is `not-installed`, `installed-running-urzua` (the file mentions `urzua` or `make records`), `installed-not-running-urzua`, or `installed-not-executable` (git ignores a hook without the execute bit; not checked on non-Unix hosts). `ok` only for `installed-running-urzua`; the other states are `warn`, since a local layer is a convenience and only CI is the guarantee. `error` when the hooks directory or hook file cannot be read: not knowing is not the same as absent. Hook-manager configuration (lefthook, pre-commit framework) is not read; a manager that installs real hook files, or sets `core.hooksPath`, is seen. |

## What's aspirational, not yet built

SPEC-3's original doctor language named more than what exists today; the gap is named here rather
than compounded:

- **Which config resolved and from where** — today's `doctor` only checks the default
  `.urzua/config.yaml` path; it doesn't report whether a `--config` override was in play or surface
  the resolution path explicitly.
- **Which rules are off** — per-rule levels exist (`ADR-53`, every rule opt-in) and `doctor` errors
  when none is enabled (`rules-enabled`), but it does not list which individual rules are off or what
  level each has.
- **Whether the binary is invoked from CI, a hook, or ad hoc** — `doctor` checks whether the CI
  *workflow file* mentions `urzua check`/`make records` as text; it doesn't detect the actual
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
