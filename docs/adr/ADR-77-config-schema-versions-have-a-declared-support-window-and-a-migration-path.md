---
Stable-Id: 01M6MKSSRDSS40P16A4C93VBYY
Status: Accepted
Accepted-on: 2026-11-05
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: RFC-49
---
# 77 — Config schema versions have a declared support window and a migration path

## Context

`urzua` reads exactly one config schema version and rejects every other with a bare "unrecognized
`schema_version`" error that names the version it understands, not what changed or how to reach it.
`RFC-49` examines this and proposes a support window and a migration path; the maintainer accepted
that proposal as written on 2026-11-05.

- **A loud break is the floor, not the contract.** `ADR-12` and `ADR-52` made `schema_version`
  mandatory so a break is loud. A loud break tells an adopter their config no longer loads, not what
  to do.
- **`ADR-54` set the default that a break ships no migration aid**, and argued it from a measured
  migration population of zero (one asset download, a single TOML config converted by hand). Its
  Consequences say a break with real adopters behind it "is a different question".
- **That population is about to stop being zero.** Phase 0 ends with urzua governing its own
  repository; Phase 1 (`MILE-102`) is a real adopter switching over. From then on a config urzua does
  not own exists outside this repository, and the next schema break is the first with a nonzero
  migration population.
- **Breaks are already frequent.** `MILE-85` removed the blockquote and bold-list header shapes
  outright and turned configs that named them into load errors; Phase 0 added `fail_on`, per-type
  `revision_log`, `embodiment`, `statuses.delivery` and `issue_refs`. A rename of any declared key is
  a break.
- **`MILE-34`'s Done means** a config on an older schema version is reported with a migration path
  rather than a bare equality reject. Nothing decided what that path is, so it could not be built.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Keep `ADR-54`'s rule: every break ships no migration aid | Nothing to maintain; no code about formats we no longer read | The first adopter's first upgrade fails with no path; the premise it rests on (a population of zero) ends at `MILE-102` |
| B. Read N and N-1, upgrade N-1 in memory with a warning, structured error outside the window, `urzua migrate config` (chosen) | One upgrade step per release; the adopter keeps running every other check through an upgrade; the error says what to do | Each release carries one tested upgrade step; an adopter who skips two releases must step through an older binary |
| C. A wider window with a chain of upgrade steps | Helps an adopter who skips releases | Every step in the chain needs tests and stays in the code indefinitely |
| D. Never break: keep reading every version ever released | No adopter is ever stranded | The loader accumulates every old shape, the thing `ADR-54` exists to prevent |

## Decision

In the context of a config schema that has broken before and will break again once a real adopter's
config exists, facing a bare rejection that names no path forward, we decided that **each release
reads the current schema version and the one before it, upgrades the older one in memory with a
warning, rejects anything outside that window with a structured error, and ships one mechanical
upgrade step, `urzua migrate config`**, to achieve a break an adopter can recover from without
reading the source, accepting that every release carries exactly one tested upgrade step.

- **The window is N and N-1.** A version older than N-1 or newer than N is outside it. `SPEC-3` gains
  a "Supported schema versions" table (release, versions read, version written).
- **Reading N-1.** A config at N-1 loads and is upgraded in memory to N before any rule runs, so rules
  only ever see the current shape. `check` reports one warning finding, `config.schema-version-outdated`,
  on the config file, naming the version found, the current version and the command that rewrites the
  file.
- **Outside the window** loading fails with exit 2 and a structured error
  `{found, supported, current, migration}`, where `migration` names the oldest release that still reads
  `found` and the command to run with it.
- **`urzua migrate config`** rewrites an N-1 config to N in place, reports every key it renamed, moved
  or dropped, and reports nothing to do on a second run. It never reads a version outside the window,
  so there is no chain of steps.
- **When a bump is required.** `schema_version` changes only for a break: a removed or renamed key, a
  changed meaning for an existing key, or a default that changes behaviour for an unchanged config.
  An additive optional key whose default preserves behaviour is not a break. The rule is written into
  `SPEC-3` beside the table, so a reviewer can apply it to a config change.
- **`ADR-54` is amended, not superseded.** Its "a break ships no migration aid" default holds only
  while no real adopter config exists. Once one does, this policy applies; `ADR-54`'s reasoning (do not
  carry knowledge of formats the tool no longer reads) is what keeps the window at one version.

## Reversibility

Moderate. The window and the upgrade step are code and a `SPEC-3` table, so removing them returns to
the bare rejection with no data change. What is hard to walk back is the promise: once an adopter
relies on N-1 loading, shrinking the window is itself a break that needs the same notice.

## Consequences

- An adopter upgrading one release keeps a green run apart from one warning, and has a command that
  fixes it. Under `fail_on: warning` that warning fails a strict repository until `migrate config` is
  run, which is the intended pressure.
- `SPEC-3` gains the supported-versions table and the bump rule, and `SPEC-2` (`check`) gains the
  `config.schema-version-outdated` rule; both change when this is built.
- The loader gains an in-memory upgrade for N-1, and a fixture config at every supported version is
  needed in the acceptance corpora, with a case proving each loads to the same effective config.
- `ADR-54`'s default no longer covers a break once a real adopter exists; a reader of that record
  needs this one.
- The "no external adopters yet" premise holds through Phase 0 and expires at the Phase 1 switchover
  (`MILE-102`); this ADR's compatibility contract takes over from that point (`MILE-118`).
- `RFC-49`'s open questions are answered by its proposal as written: a window of N and N-1, "outdated"
  as a warning, `migrate config` as its own command, and a fixture at every supported version.

## References

- `RFC-49` -- the proposal this accepts as written.
- `ADR-54` -- amended: its "no migration aid" default now holds only while no real adopter config exists.
- `ADR-12`, `ADR-52` -- `schema_version` is required and a mismatch is loud.
- `ADR-22` -- `migrate schema` stays report-only for record schemas; `migrate config` is a separate
  subject.
- `MILE-34` -- the milestone this lets be built.
- `MILE-102`, `MILE-118` -- the Phase 1 switchover and the premise it ends.
- `SPEC-3` -- where the supported-versions table and the bump rule live.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** Maintainer decision, 2026-11-05, recorded here; the maintainer accepts it in a later change. | **substantive** |
> | 2026-11-05 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-05. | **substantive** |
