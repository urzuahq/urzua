---
Stable-Id: 01M6K4FNPS6S867HWF3HN5Q5CC
Status: Proposed
Date: 2026-11-04
Author: beauwilliams
Deciders: beauwilliams
Derives-from: RFC-7
---
# 69 — A record edit is judged by urzua before it is written, and the harness hook only relays the verdict

## Context

`RFC-7` proposes a pre-write hook so that an edit to a computed field is judged before it lands, and leaves open how it blocks, what it does when it cannot judge, and who owns the derivation. This repository has the pieces and the failures that bear on those choices.

- The computed value exists: `urzua fix` derives `Embodiment` from `Realized-by` (`ADR-15`, `ADR-20`). `AGENTS.md` asks agents to run it rather than hand-assert `Verified`, which is an instruction; `RFC-27` section 5 names the same gap for acceptance.
- The tool cannot yet be asked cheaply: `fix` is whole-corpus, has no path argument, and an editor hook must parse the whole repair list to find its own file (`MILE-159`).
- A gate that cannot run has reported success here before. A shallow checkout makes drift detection unable to see history and the run reads clean (`MILE-159`'s record of the earlier false zero, and the `fetch-depth: 0` comment in `checks.yml`); `.git-hooks/pre-commit` exits 0 with a notice when the binary is not built; the release workflow reported success for six days while preparing nothing (`BUG-27`).
- A gate that blocks too much is switched off: `MILE-99` records that a hook refusing on any finding would refuse every push today, because 61 findings are non-blocking.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Instruction only (today) | Nothing to build | Depends on the agent remembering; the scenario `RFC-7` opens with |
| B. A hook that recomputes the value in its own code | Self-contained | A second implementation of the derivation, the duplication `SPEC-22` forbids, and the two can disagree |
| C. A hook that relays urzua's verdict for the edited file (chosen) | One derivation; the hook and `check` cannot disagree | Needs the path-scoped, prospective-content form (`MILE-159`) first |
| D. Block every edit to a record when evidence is unavailable | No unsupported claim slips through | One missing binary freezes all record edits; the hook gets disabled |

## Decision

In the context of agents editing record fields that `urzua` computes, facing an instruction that nothing enforces and a history of gates that read as success when they could not run, we decided that **a record edit is judged by `urzua` before it is written and the harness hook only relays the verdict**, to achieve an enforcement that fires whether or not the agent remembered the rule, accepting that an edit made where `urzua` cannot run is not stopped at the editor.

- **One derivation.** The hook calls `urzua` with the edit's prospective content and reads its JSON. It contains no copy of the derivation, so it cannot disagree with `check` or `fix`.
- **Blocks the value, not the work.** It refuses an edit that sets a computed field to a value other than the computed one, in either direction (claiming more or less than the evidence), and names the computed value and its evidence. Every other edit to the same record passes.
- **Cannot judge means permit, loudly.** When `urzua` is missing, reports `not-run`, or cannot read history, the edit proceeds and the hook states in output the agent sees that it did not judge. A silent pass is a defect (`AGENTS.md`, no silent no-op). The commit and CI layers still apply (`ADR-71`).
- **Override is explicit.** Proceeding past a block needs a named override whose value appears in the hook's output. It is never a silent bypass.
- **The contract is urzua's JSON.** Only a thin adapter is harness-specific. A second harness is a second adapter over the same call.
- **Not decided here.** Where an override is persisted beyond the hook's output, whom an unacknowledged block routes to, and whether other computed fields join `Embodiment`. These stay `RFC-7`'s open questions (`MILE-12`).

## Reversibility

Easy. The hook is configuration plus an adapter script; removing it returns to the instruction-only state with no data change. The cost of being wrong is a permissive hook that lets an unsupported value through to the commit and CI layers, or a strict one that is overridden often.

## Consequences

- An agent that sets `Embodiment` by hand is stopped at the edit, with the correct value in the message.
- The hook is only as fast as a single-file `urzua` run, so `MILE-159` is a precondition.
- Permitting when `urzua` cannot run means the editor layer is not a guarantee; that role belongs to CI (`ADR-71`).
- The override is a named, visible event, but without persistence beyond the output it is an audit trail only for as long as the transcript lives.

## References

- `RFC-7` -- the enforcement harness this builds a part of (sections 2 and 3).
- `RFC-27` -- instructions versus mechanism for acceptance.
- `MILE-159` -- path-scoped, prospective-content `fix` the hook calls.
- `MILE-178` -- the milestone that builds this.
- `ADR-71` -- the other layers this one is not a substitute for.
