---
Stable-Id: 01M6K4FP7JT62G3SJG8X1BDHKP
Status: Accepted
Date: 2026-11-04
Accepted-on: 2026-11-04
Author: beauwilliams
Deciders: beauwilliams
Derives-from: RFC-7
---
# 70 — Drafting commands scaffold through urzua new and carry no schema knowledge of their own

## Context

`RFC-7` section 3 puts drafting-assistance commands in scope, with one condition: their output is still verified by `urzua check`, never trusted for having come from the command. The repository's own evidence shows both why a command is wanted and how one goes wrong.

- Hand-authoring persists although `urzua new` exists: several records here were hand-written and two were rejected by `check` for headers `urzua new` would have produced (`MILE-103`); two more were pushed with headers `check` rejected (`MILE-99`).
- A generator that disagrees with the checker is two sources of truth for one shape (`MILE-151`, `BUG-182`).
- `urzua new` leaves its file untracked and `check` governs tracked files only, so a reference to a fresh record was reported dangling twice and misdiagnosed as a stale build (`ADR-56`).
- `AGENTS.md` says to check `.urzua/config.yaml` before writing any header or field, and warns against freehanding a convention because it matches nearby records. A command that restated field sets or status values in prose would be exactly that copy, and would go stale when the config changes.
- `urzua new` fills the template in, not the decision (`ADR-27`); and a record's `Status` is a human decision (`AGENTS.md`).

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. No commands; `AGENTS.md` prose only (today) | Nothing to maintain | Hand-authoring continues; the prose is an instruction |
| B. Self-contained commands that spell out the schema | Work with no tool call | A second copy of the schema that drifts from `.urzua/config.yaml` |
| C. Thin commands that call `urzua new`, fill content, stage, and run `urzua check` (chosen) | Schema lives in one place; the checker verifies the result | Needs the binary built |
| D. A new `urzua` subcommand that takes content and writes the whole record | One path for every harness | A write command with its own policy; it is `MILE-79` (Phase 2), not decided here |

## Decision

In the context of agents drafting records, facing hand-authoring that `check` keeps rejecting, we decided that **drafting commands scaffold through `urzua new` and carry no schema knowledge of their own**, to achieve a single source of truth for record shape with the checker as the verifier, accepting that a command is only as complete as `urzua new` and the declared templates.

- **Scaffold through the tool.** A command creates the record with `urzua new` and edits the content of the sections; it does not write a header, a status vocabulary or a field list.
- **Stage, then check.** The command stages the new file so `check` governs it, runs `urzua check`, and reports the findings together with the number of records examined, so a clean run that never saw the new file is visible. Success is never reported on a record `check` did not examine.
- **Never decide.** A command creates `Draft`, `Open`, `Planned` or `Proposed` and refuses to write `Accepted`, `Done`, `Fixed` or an acceptance date. This restates `AGENTS.md` as a refusal in the command.
- **Walked by a test.** Each command's steps are run against a fixture corpus; the test fails when the record is not examined or when the command emits a field `urzua new` did not.
- **Per harness.** The files are harness-specific (`.claude/commands/` here). Distribution to other repositories and harnesses is a separate decision (`RFC-9` section 4 sketches harness skills).

## Reversibility

Easy. The commands are markdown files; deleting them returns to `AGENTS.md` prose. The decision constrains what they may contain, not the record format.

## Consequences

- When the template or config changes, the command is right without an edit.
- A command cannot make a record valid that `urzua new` cannot; a type with no template is refused, as `ADR-27` already requires.
- The command depends on a built binary, the same precondition as every other local `urzua` use here.
- The suggestion rule in `AGENTS.md` ("offer the command when a decision is noticed") stays an instruction; no tool catches a missed suggestion.

## Amendment (2026-11-05): a lifecycle command may set a status only when the engine computes it from evidence

*Status of this amendment: proposed, pending maintainer acceptance (decided by the maintainer on 2026-11-05, drafted with `ADR-89`). `Status` stays `Accepted` and the Decision text above is unchanged.*

The Decision says a drafting command "refuses to write `Accepted`, `Done`, `Fixed` or an acceptance date", and the Context says a record's `Status` is a human decision. `ADR-89` adds lifecycle commands, among them `/done`, which finishes a milestone or a bug, and a reading of that command as outside this decision would leave a standing tension between the two. This amendment resolves it.

**1. Drafting commands are unchanged.** A drafting command creates `Draft`, `Open`, `Planned` or `Proposed` and refuses every other status, as above.

**2. A lifecycle command may set a status only when the engine computes it from evidence.** The status of a type that declares an evidence field follows that evidence (`ADR-84`). A lifecycle command records the evidence (for `/done`, `Realized-by` or the fix's regression test) and then runs `urzua fix --apply`, so the status written is the one the engine computed. The command never writes a status itself and never takes one from the agent. An agent's choice of status is still a human decision, as the Context says.

**3. A status an acceptance gate names stays out of reach.** `ADR-89`'s `authority` gates (`Accepted`, `Rejected` for a decision) are not computed from evidence, and no lifecycle command sets them.

**4. Nothing else changes.** `urzua check` still verifies every result, and a command still reports the number of records examined.

## References

- `RFC-7` -- section 3, drafting-assistance commands.
- `MILE-177` -- the milestone that builds this.
- `MILE-103`, `MILE-151`, `BUG-182` -- hand-authoring and generator/checker disagreement.
- `ADR-27`, `ADR-56` -- what `urzua new` writes and why a new file is invisible to `check`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-05 | Amendment added: a lifecycle command may set a status only when the engine computes it from the record's evidence (`urzua fix --apply`), never by an agent's choice; drafting commands are unchanged and gated statuses stay out of reach. **Why:** `ADR-89` adds `/done`, which writes `Done` and `Fixed`, and the Decision forbids a command to write them; maintainer decision, 2026-11-05; proposed, pending maintainer acceptance, `Status` stays `Accepted` and the Decision text is unchanged. | **substantive** |
