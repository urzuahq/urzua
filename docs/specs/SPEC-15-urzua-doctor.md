---
Version: '0.3'
Date: 2026-09-07
Status: Accepted
Author: '@beauwilliams'
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

`status` is the worst severity across `checks` (`error` beats `warn` beats `ok`). Exit codes: `2` if
`.urzua/config.toml` doesn't exist at all (run `urzua init` first); `1` if the config fails to parse,
or any check reports `error`; `0` otherwise — a `warn`-only report still exits `0`, since only an
error blocks.

## Checks, by id (shipped)

| `check` id | Meaning |
|---|---|
| `config-exists` | `.urzua/config.toml` is present. `error` if missing (short-circuits, exit 2). |
| `config-parses` | The config parses with no unrecognized keys (`deny_unknown_fields` — a typo'd key is a hard parse error, not a silently-ignored one). `error` if not (short-circuits, exit 1). |
| `record-types-declared` | At least one `[record_types.*]` entry exists. `error` if the map is empty — `check` would never examine anything. |
| `record-type-dir` | One check per declared type: does `dir` actually exist on disk. `error` if not — this fires today for `waiver` in this very repo, correctly, since no waiver record has been created yet (MILE-59) and its directory was deliberately never pre-created. |
| `record-type-required-fields` | One check per declared type with an empty `required_fields`. `warn`, not `error` — a type with no required fields is unusual, not necessarily wrong. |
| `ci-wired` | `.github/workflows/ci.yml` contains `urzua check` or `make records`. `warn` if not — a check that exists but is never run reports nothing to anyone. |

## What's aspirational, not yet built

SPEC-3's original doctor language named more than what exists today; the gap is named here rather
than compounded:

- **Which config resolved and from where** — today's `doctor` only checks the default
  `.urzua/config.toml` path; it doesn't report whether a `--config` override was in play or surface
  the resolution path explicitly.
- **Which rules are off** — there is no per-rule enable/disable or severity config yet at all
  (MILE-80 tracks configurable severity); `doctor` can't list what doesn't exist as a config surface.
- **Whether the binary is invoked from CI, a hook, or ad hoc** — `doctor` only checks whether the CI
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
