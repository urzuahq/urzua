---
Stable-Id: 01M6MKSV30JT6N72111MQPCC5X
Status: Proposed
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: RFC-38
---
# 79 — audit folds into check and the command surface keeps one read-only validator

## Context

`RFC-38` (Accepted 2026-11-04) concludes that a read-only subset of a command does not earn a command,
and proposes folding `urzua audit` into `urzua check`. This ADR records the part of it that is now
being built: the `audit` half. The `fix` detect-mode half of `RFC-38` names its obstacles and is not
decided here.

- **`audit` and `check` are the same operation.** `audit` runs two rule functions `check` already runs
  (`pointer.resolution`, `relation.supersession-reciprocity`, and later `identity.collision`), over the
  same corpus, producing the same `CheckReport` with the same exit codes.
- **`audit` owns a rule set, and `ADR-53` made that wrong.** Every rule is a declared, opt-in policy;
  a hardcoded subset inside the tool overrides the adopter's declaration. `BUG-42` was that
  contradiction surfacing once and `BUG-84` once more.
- **`BUG-99` is the third instance.** On a corpus with no type prefix, `urzua init` correctly declines
  the identity-dependent rules, and `audit`, whose rules are exactly those, has nothing declared to
  run. Its triage of 2026-11-04 records that the reproduction was closed incidentally and the root
  cause, `audit` owning a rule set, is not.
- **Nothing depends on `audit`.** `MILE-62` built it and is closed; the only roadmap references are
  bugs it caused.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Patch each instance (`BUG-42`, `BUG-84`, `BUG-99`) | No surface change | A third patch to a command that keeps producing the bug; `audit` keeps its own rule set |
| B. Keep `audit` and give it a rule set read from config | `audit` stops overriding the adopter | Still a second command for the same read-only operation, with a preset to maintain |
| C. Fold `audit` into `check` through rule selection, deprecate it for one release, then remove it (chosen) | One validator; the root of all three bugs is gone; no adopter build breaks on the warning | One release of two names for one operation |
| D. Remove `audit` at once | Smallest surface soonest | An adopter calling it in CI gets a hard failure with no release to migrate in |

## Decision

In the context of a read-only `audit` that duplicates `check` and owns a hardcoded rule pair, facing
three bugs rooted in that ownership, we decided that **`audit` is folded into `check`: `check` gains
rule selection, `audit` stops running a rule set of its own and becomes a deprecated read-only alias of
`check` over that selection, and it is removed the release after**, to achieve one read-only validator
whose rules are only ever the ones the repository declared, accepting one release in which the same
operation has two names.

- **`check --rules <id>[,<id>...]`** restricts a run to the named declared rules. Selection narrows what
  runs and never enables anything, so `ADR-53` holds. A requested rule the config does not declare is an
  invocation error, exit 2: asking for a policy that does not exist is a mistake in the command, not a
  finding about the corpus.
- **An excluded rule is omitted from `rules_executed`**, and the report gains a top-level
  `rules_selected` naming the filter. It is never reported as `not-enabled`, which would say the
  repository declined a policy it declared. `status` and the exit code are computed over the selected
  set exactly as over the full set.
- **`audit` is an alias, not a rule runner.** It runs through `check`'s engine and the declared config,
  not through its own rule calls, and its report equals `check --rules` over the rules it selected,
  including `rules_selected`. A consumer migrating from `audit` changes the invocation and nothing in
  the shape of what comes back.
- **One deprecation release.** `audit` emits a `Notice` naming the replacement invocation; a `Notice`
  never moves the exit code (`ADR-46`), so no adopter's build breaks on the warning. It is removed the
  release after, with a clear message for a caller that still invokes it.
- **`BUG-99` closes when `audit` is removed**, not by a separate patch; its candidate fixes were all
  patches to the command being retired.
- **Not decided here:** `RFC-38`'s `fix` half (detect mode consuming the rule's result), and whether
  rules declare a category so `--rules cross-record` is expressible. Both stay `RFC-38`'s open questions.

## Reversibility

Moderate. Rule selection is additive and cheap to keep. Removing `audit` is a command-surface break, so
re-adding it after the removal release costs a deprecation cycle of its own; the deprecation release is
what limits the cost of being wrong. The report-shape change (`rules_selected`) is additive.

## Consequences

- `urzua --help` loses a command after the removal release, and `SPEC-1` and `SPEC-2` change to describe
  `--rules` and `rules_selected`; `SPEC-8` and `SPEC-11` change where they name `audit`.
- Adopters, if any, have one release to move `urzua audit` to `urzua check --rules ...` in CI.
- `BUG-42`, `BUG-84` and `BUG-99` stop being reachable, because no command owns a rule set the config
  does not declare.
- `RFC-38` records that this does not ship in `0.4.0`, which already carries a breaking report-contract
  change; the CLI-surface break wants its own release and a migration note.
- The deprecation release still lets `audit` exit 2 on a prefixless corpus where the selected rules are
  not declared, until it is removed.

## References

- `RFC-38` -- the proposal; sections 1 to 3 are built here.
- `RFC-20` -- the command taxonomy whose axes `RFC-38` applies.
- `ADR-30` -- `audit` reusing `check`'s rule functions.
- `ADR-53` -- every rule is a declared policy, which a hardcoded subset overrides.
- `ADR-46` -- a `Notice` never moves the exit code.
- `BUG-42`, `BUG-84`, `BUG-99` -- the bugs rooted in `audit` owning a rule set.
- `MILE-62` -- built `audit`; closed.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** Maintainer decision, 2026-11-05, recorded here; the maintainer accepts it in a later change. | **substantive** |
