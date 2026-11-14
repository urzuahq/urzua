---
Stable-Id: 01M6NH5DR5GP88XJEVRXGBA1B1
Status: Accepted
Accepted-on: 2026-11-05
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: Verified
Supersedes / Superseded-by: null
Derives-from: ADR-73
---
# 87 — A rule may have the level info: its findings are reported and counted but never trip fail_on

## Context

A rule's level is `off`, `warn` or `error`, and nothing else loads. `RuleLevel` has three variants and its
deserializer refuses any other string with "expected \"off\", \"warn\", or \"error\""
(`rust/crates/urzua-core/src/config.rs:502-541`). The gate turns the level into a finding's severity at
`rust/crates/urzua-core/src/rules/runner.rs:122-125`, and a `field_levels` entry is mapped the same way at
`rules/shared.rs:546-551`. The severity a finding carries has two values, `FindingSeverity::{Error, Warning}`
(`report.rs:11-16`), and `SPEC-2` says so ("`Error`/`Warning` only, no `Info`", "Output contract"). `Info` exists
only as `NoticeSeverity::{Info, Warning}` (`report.rs`, after `CheckReport::exit_code`), a separate type for
non-file-scoped notices that cannot affect the exit code.

How a finding fails a run:

- `blocking` is true when an unwaived finding has `FindingSeverity::Error` (`commands/check.rs:370-372`, and
  the per-run copy at `check.rs:970`).
- `fail_on` is `error` or `warning` (`config.rs:96-100`; `SPEC-3`, "Exit threshold: `fail_on`"). It is read in
  one place, `CheckReport::exit_code` (`report.rs:633-650`): exit 1 when `blocking`, or when `fail_on` is
  `Warning` and an unwaived finding is `Warning`. The report is byte-identical under either value.
- `gate::derive_status` reports `findings-present` for any unwaived finding of any severity
  (`gate.rs:41-63`), so a warning-only run is `findings-present` with `blocking: false`.

The gap: this repository runs `fail_on: warning` (`.urzua/config.yaml:4`) so CI keeps it at zero, which makes a
`warn` rule a failing rule. An advisory rule has no level to sit at. `MILE-189`'s rule,
`embodiment.locator-on-back-pointer-path`, reports 308 findings on 129 records here (188 `code:` and 120
`test:` locators), so it cannot be enabled at `warn` without red CI, and enabling it at `error` is worse.
`MILE-189`'s Done means asks for the rule enabled in this repository's configuration, and its revision log
of 2026-11-05 records that this needs a decision the milestone does not carry. The only alternatives today are
to keep the rule off or to waive 308 findings. The maintainer decided on 2026-11-05 to add a level that reports
without failing.

Output formats: stdout is JSON only (`ADR-23`, `ADR-26`). There is no SARIF writer in the repository, so
nothing maps to SARIF today; `severity` is serialized kebab-case from `FindingSeverity`
(`report.rs:12`).

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Leave levels as they are; keep advisory rules off here | Nothing to build | `MILE-189`'s Done means cannot be met; 308 findings are never seen; an adopter with `fail_on: warning` cannot take an advisory rule at all |
| B. Waive the findings of an advisory rule | Uses an existing record | 308 waivers, or one broad waiver that hides the finding instead of reporting it; a waived finding is listed, but waiver is meant to excuse, not to say "advisory" |
| C. Lower `fail_on` to `error` here | Warnings stop failing | Loses the zero-warnings guarantee `MILE-145` added; every warning rule becomes advisory at once |
| D. A per-rule `fail: false` flag beside `level: warn` | No new severity | A second axis that every reader of a rule must combine with its level; the finding still says `warning`, so a consumer cannot tell the advisory ones apart |
| E. Route advisory findings through `notices` | Notices already never affect the exit code | Notices are not file-scoped findings: no rule, line, population, waiver or suggested action (`ADR-73`), so the advisory rule loses what makes it actionable |
| F. A new opt-in `level: info` whose findings are real findings with severity `info` (chosen) | One value on the axis the config already has; findings keep rule, file, line, population, action and waiver; configs that do not use it are unchanged | A third severity every consumer must handle; `RuleLevel` and `FindingSeverity` both grow |

## Decision

In the context of advisory rules that cannot be enabled in a repository that fails on warnings, facing 308
findings from `MILE-189`'s rule here that no existing level can carry, we decided **to add `level: info`
for rules, as an opt-in third reporting level**, to achieve a rule whose findings are reported and counted
without failing the run, accepting a third severity in the report.

1. **`info` is a rule level.** `RuleLevel` gains `info` beside `off`, `warn` and `error`, and a rule can be set
   to it under `rules:`. `field_levels` accepts it for the same reason (a field's finding may be advisory
   while the rule's others are not). Any other value is still refused at load, and the message lists four.
2. **A finding at that level has severity `info`.** `FindingSeverity` gains `Info`, serialized `"info"`. The
   rule runs, its findings are reported with rule, file, line, population and `suggested_action` exactly as at
   `warn`, and they are counted wherever findings are counted (the report's `findings`, `rules_executed`
   populations, and `status`: a run with an unwaived info finding is `findings-present`, as for a warning).
3. **An info finding never trips `fail_on`, whatever its value.** It does not set `blocking`, and
   `exit_code` ignores it under `fail_on: warning` as under `error`. This is the point of the level.
4. **Suggested actions are kept.** An info finding carries the `ADR-73` action its rule declares; an
   exemption with a reason stays an exemption. Nothing about actions depends on severity.
5. **A config that does not use `info` is unchanged.** No existing level is reinterpreted, no existing report
   changes a byte, and `fail_on` keeps its two values.
6. **`fail_on: info` does not exist.** The key's meaning is the lowest severity that fails a run, and the
   purpose of `info` is to be below that line. `fail_on: info` would make advisory findings fail, which is
   `warn` under another name, and it would give a repository a way to reach red from the level that exists to
   avoid it. It is refused at load like any other value. A repository that wants every finding to fail uses
   `fail_on: warning` and levels `warn` or `error`.
7. **Waivers cover info findings like any other** (`waiver::Waiver::covers` matches on rule and path, not
   severity, `ADR-11`). A waived info finding is listed with its `waived` field and is excluded from the active
   count. There is rarely a reason to waive one, since it fails nothing.
8. **Output formats.** The JSON report is the only output (`ADR-23`); `severity` can now be `"info"`.
   `notices` keep their own `NoticeSeverity::{Info, Warning}` and are unchanged: a notice is not a finding. If
   a SARIF or other writer is added, `info` maps to its lowest advisory level (SARIF `note`), `warning` to
   `warning` and `error` to `error`; that mapping is decided with the writer, not here. The editor hook
   already treats every non-error finding as a shown warning (`scripts/hooks/record-edit-hook.sh`, the
   `others` set) and needs no change.
9. **This repository enables `MILE-189`'s rule at `info`.** Once the level is built, `MILE-189`'s Done means is
   met by enabling `embodiment.locator-on-back-pointer-path` at `info` in `.urzua/config.yaml`; its 308
   findings are reported and fail nothing. `MILE-189` stays Planned until then.
10. **A declared severity never overrides `info`, and no level maps silently.** Where a rule's findings take a
   severity from a declaration (a boundary's `severity`, a `field_levels` entry), `info` is an accepted value
   there too, and a rule set to `info` never emits a finding above `info` because a declaration said
   otherwise (today `rules/runner.rs:133` lets a declared severity win). Any mapping from a level string to a
   severity is exhaustive: an unknown or unhandled value is a load error, never a fallback to `error`
   (today `options.rs:82-84` falls back with `_ => Error`). The build tests both.

## Amendment (2026-11-14): `blocking` follows the `fail_on` threshold

Decided by the orchestrator on 2026-11-14, pending maintainer review.

The Decision and the Context above say the report is byte-identical under either `fail_on` value, and that `blocking` counts only `Error` findings. That split was deliberate and is reversed for `blocking` only: `blocking` is now true exactly when the run exits 1, so an unwaived warning under `fail_on: warning` (or `--fail-on warning`) reads `"blocking": true`. Under `fail_on: error` it reads `false`, as before. Nothing else in the report depends on the threshold: `status`, `findings`, severities and counts are unchanged, and an `info` finding still never counts.

Why: consumers branch on stdout alone. The MCP server has no exit code, so an agent that read `blocking: false` on a change this repository's CI fails had no way to know. Nothing in this repository gates on `blocking`: the gate action, `render.sh` and the hooks read the exit code, so the change moves no gate. The external review (finding 5.1) found the disagreement. `SPEC-2` (0.162) and `MILE-145` record the same change.

The Decision text is unchanged.

## Reversibility

Moderate. Adding the level is additive and configs without it are unchanged. Walking it back later means
removing a value from `RuleLevel` and `FindingSeverity` and rewriting any config that used it, and any
consumer that parsed `"info"` from the report would see the field vanish. Because `severity` is part of the
JSON contract (`SPEC-2`), a consumer that matched `error` and `warning` exhaustively could fail on the new
value, so the cost of being wrong rises once an adopter relies on it. Mitigated by being opt-in: no report
contains `info` unless the repository chose it.

## Consequences

- `RuleLevel` and `FindingSeverity` each gain a variant; every `match` on them is revisited
  (`runner.rs:122-125`, `shared.rs:546-551`, `options.rs:70-84` for a boundary's severity, `relation.rs:318` for
  the provisional level, `boundary.rs:251`), and the places that read `level != Off` are unchanged
  (`commands/check.rs:262`, `refs.rs:44`, `doctor.rs:283`).
- `blocking` (`check.rs:370`, `check.rs:970`) and `exit_code` (`report.rs:640-645`) need no logic change
  beyond not counting `Info`; a test must pin that an info finding exits 0 under `fail_on: warning`.
- `SPEC-2` (the severity sentence and the output contract) and `SPEC-3` (`level`, `field_levels`,
  `fail_on`) change when this is built, and the generated rule table if it shows levels.
- A report that is `findings-present` with exit 0 is already possible (warnings under `fail_on: error`), so no
  consumer meets a new combination.
- Advisory findings can be ignored forever. A repository that enables a rule at `info` and never reads the
  report gets no pressure from it; that is the stated trade.
- Boundary severity (`BoundarySeverity::{Error, Warn}`) and the provisional level are separate axes and are not
  extended here.
- `MILE-194` builds this.

## References

- `ADR-73` -- suggested actions, kept at this level.
- `ADR-11` -- waivers, which cover info findings like any other.
- `ADR-23`, `ADR-26` -- stdout is JSON only.
- `MILE-145` -- `fail_on`.
- `MILE-189` -- the advisory rule that motivates the level; its Done means is met by enabling it at `info`.
- `MILE-194` -- builds this decision.
- `SPEC-2`, `SPEC-3` -- the check output contract and the configuration.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** Maintainer decision, 2026-11-05, recorded here; the maintainer accepts it in a later change. | **substantive** |
> | 2026-11-05 | Status changed from Proposed to Accepted. Added point 10 from the independent review: a declared severity never overrides `info`, and a level string never maps silently to `error`. **Why:** Maintainer decision, 2026-11-05. | **substantive** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `no Realized-by field; cited by test back-pointers` -- stated no value disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Records edit: the Consequences bullet "Boundary severity ... not extended here" is superseded by point 10 for the `info` value: a boundary `severity` accepts `info` and a rule set to `info` caps it. The provisional level (`ProvisionalLevel`) stays `warn`/`error`; the cap in point 10 keeps it from lifting an `info` rule. Status stays Accepted. **Why:** point 10 was added at acceptance and the Consequences section was not updated; `MILE-194` follows point 10. | **structural** |
> | 2026-11-14 | Amendment added, decided by the orchestrator on 2026-11-14, pending maintainer review: `blocking` follows the `fail_on` threshold (true exactly when the run exits 1), reversing the "byte-identical under either value" split for that one key; the Decision is unchanged. **Why:** the MCP server and other agent consumers branch on stdout alone and have no exit code; no gate reads `blocking`. External review finding 5.1. | **substantive** |
