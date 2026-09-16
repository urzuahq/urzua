---
Status: Planned
Stable-Id: 01M1Z2T44DHPRE5C4DVENWXY58
Phase: '1'
Track: schema-governance
Implements: —
Blocked-on: —
---
# 80 — Configurable rule severity, per SPEC-1's own unbuilt promise

## What

`Severity` is a hardcoded two-value enum (`Error`/`Warning`) baked directly into ~13 call sites
across `rules.rs`, with zero config surface -- despite SPEC-1 explicitly listing "which checks are
errors vs. warnings" as configurable from v0. This milestone owns *making a rule's severity
configurable per repo* in `.urzua/config.toml` (a config-surface question); whether `Severity`
itself gains more values (e.g. `Info` — MILE-46, found independently, same day) is a separate
schema/enum question, kept apart deliberately rather than conflated. Also separate: whether the CLI
should gain a general log-verbosity control (debug/info-as-diagnostic-noise while a command runs) —
orthogonal to findings entirely, since a finding is either a real problem or it isn't, while
verbosity is about how much the tool narrates while deciding that.

## Why

Raised live: rules today only distinguish two severities, hardcoded in source, with no way for a
repo to downgrade or upgrade a given rule's default -- exactly the gap SPEC-1 named as a v0
configuration surface and never built. Worth resolving the findings-severity-vs-log-verbosity
distinction explicitly before building either, rather than conflating "how bad is this finding"
with "how much should the tool print" into one enum.

## A worked instance

`ADR-45` carried `Embodiment: Verified` from its first commit (`3b0fe66`, 2026-09-10) while every
`Realized-by` locator was a `code:` one and nothing tested the flow. The computed tier was
`Implemented`. `embodiment.consistency` reported the disagreement correctly on every run for six
days, and nothing happened, because:

- the rule emits `Warning`, hardcoded at its call site;
- `check` computes `blocking` from `Error`-severity findings only;
- the corpus carries **204 findings, all warnings, zero errors**, so `blocking` is `false` and
  `urzua check docs/` exits 0.

It was found only because someone opened that record for an unrelated reason. This is the concrete
cost of the gap: a record asserting a higher evidence tier than it holds is an untrue claim in the
corpus -- the class this project exists to catch -- and it is currently indistinguishable from 203
routine observations.

Note what this milestone is *not*: the rule is correct and its default of `Warning` is defensible,
since drift is usually a signal to review. What is missing is any way for this repo to say that
*this* rule, here, should block.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Added a worked instance: `ADR-45`'s false `Embodiment: Verified`, reported correctly and non-blockingly for six days among 204 warnings. **Why:** this milestone's motivation was a principle ("rules only distinguish two severities"); it now has a dated case where the missing configurability let an untrue claim survive in the corpus, which is a stronger argument for building it than the principle was. | **substantive** |
