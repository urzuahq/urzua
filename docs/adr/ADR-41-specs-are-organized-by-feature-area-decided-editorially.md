---
Status: Accepted
Stable-Id: 01M1Z6WVWN81TYEJ19XJYY3JCC
Embodiment: Not started
Date: 2026-09-07
Author: '@beauwilliams'
Deciders: '@beauwilliams'
Supersedes / Superseded-by: —
Derives-from: ADR-14
---
# 41 — Specs are organized by feature area decided editorially

## Context

No rule had ever existed for when a subject (a configured record type, a command) warrants its own
spec versus an ADR being sufficient forever. In its absence, this corpus already produced
inconsistent precedent: `milestone` (ADR-34) got both an ADR and a spec (SPEC-6); `bug` (ADR-35) and
`waiver` (ADR-11) — the same shape of decision — got no spec. `check`/`init`/config (SPEC-2/3/5) had
specs; `new`/`fix`/`audit`/`migrate`/`explain`/`graph` (ADR-27/19/20/30/22/24) didn't, despite
comparable command-surface complexity. `doctor` had been folded into SPEC-3 (Configuration) by
default, not by any deliberate call.

A first attempt at resolving this reached for a mechanical trigger: "an ADR that needs a second
amendment gets promoted to a spec," based on the observation that `milestone`'s ADR-34 was the only
ADR with 2+ amendments and the only subject with a companion spec. **This was rejected directly and
explicitly**: a spec is not a count of anything. It groups a coherent *feature area* together — the
same call that already, correctly, put `doctor` inside SPEC-3 (a real feature-area judgment, just
made in the wrong direction) and gave `milestone` its own SPEC-6 (a real feature-area judgment, made
in the right direction). Counting amendments would have answered a different question than the one
actually being asked.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| No rule; decide case by case forever | Flexible | Already produced the inconsistent precedent above before any rule existed |
| Mechanical trigger (e.g. amendment count, ADR count per subject) | Cheap to check, no judgment required | Answers "how much has this been revised," not "is this a coherent feature area" — rejected directly during this decision |
| Editorial judgment: does this subject constitute a coherent, buildable feature area | Matches how `milestone`/SPEC-6 and `doctor`/SPEC-3 were already (correctly and incorrectly, respectively) decided | Requires a human call each time, not a formula anyone can run unattended |

## Decision

In the context of no existing rule having already produced inconsistent precedent, and a mechanical
alternative having been tried and rejected, we decided: **a spec is warranted when a subject
constitutes a coherent, buildable feature area — decided editorially, per subject, not by any
formula.** A single ADR stays sufficient for a subject with no larger feature area to bundle it
into. This is a judgment call, made the same way every time: does bundling this subject's design
into one living, buildable document serve a real reader better than reading its ADR(s) alone.

Applied directly, the same day: `fix` (SPEC-8, bundling ADR-15/18/19/20), `bug` (SPEC-9), `waiver`
(SPEC-10), `audit` (SPEC-11), `new` (SPEC-12), `explain`/`graph` (SPEC-13, bundled as one feature
area), `migrate ids`/`migrate schema` (SPEC-14, bundled). `doctor` moved the other direction: split
out of SPEC-3 into its own SPEC-15, since it is a real, standalone feature area with its own output
shape and bug history (BUG-4), not a detail of how configuration is structured.

## Reversibility

A spec that turns out not to have earned its own document can be folded back into a parent or
sibling spec later — the record shape doesn't force a wrong call to be permanent. Nothing about this
decision constrains later re-grouping; it only replaces "no rule" with "a named, deliberate call."

## Consequences

- MILE-77 is resolved: every ADR-only command/type has been reviewed once against this criterion,
  and specs created for the ones judged to be real feature areas.
- No future promotion happens automatically. A new command or record type stays ADR-only until
  someone deliberately judges it a coherent enough feature area to bundle — the same discipline
  `milestone` and `doctor` were already (inconsistently) held to before this ADR named the rule.
- Specs may cross-reference each other (`Parent`, `Implements`) freely — SPEC-13 citing SPEC-8,
  SPEC-14 citing SPEC-10, etc. — and those references are now checked by `pointer.resolution`
  (ADR-40), not left as unverified prose.

## References

- ADR-14 — the ADR-vs-spec accountability model (visible amendments vs. living document) this
  decision assumes but doesn't itself decide.
- ADR-34/SPEC-6 — the `milestone` precedent this decision generalizes from.
- ADR-40 — the pointer-resolution fix that makes spec-to-spec `Parent` references checked, not just
  possible.
- MILE-77 — the milestone this ADR resolves.
