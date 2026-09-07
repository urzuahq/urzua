# 80 — Configurable rule severity, per SPEC-1's own unbuilt promise

> Status: Planned
> Stable-Id: 01M1Z2T44DHPRE5C4DVENWXY58
> Phase: 1
> Track: schema-governance
> Implements: —

## What

`Severity` is a hardcoded two-value enum (`Error`/`Warning`) baked directly into ~13 call sites
across `rules.rs`, with zero config surface -- despite SPEC-1 explicitly listing "which checks are
errors vs. warnings" as configurable from v0. Two distinct things need deciding before building
either: (a) should `Severity` itself gain more values (e.g. `Info`), which is a *findings*-severity
question feeding `blocking`/exit-code computation the same way `Error`/`Warning` already do; and (b)
separately, should the CLI gain a general log-verbosity control (debug/info-as-diagnostic-noise
while a command runs), which is orthogonal to findings entirely -- a finding is either a real
problem or it isn't, verbosity is about how much the tool narrates while deciding that. Once
resolved, make whichever severity axis is chosen configurable per rule in `.urzua/config.toml`,
fulfilling SPEC-1's original promise.

## Why

Raised live: rules today only distinguish two severities, hardcoded in source, with no way for a
repo to downgrade or upgrade a given rule's default -- exactly the gap SPEC-1 named as a v0
configuration surface and never built. Worth resolving the findings-severity-vs-log-verbosity
distinction explicitly before building either, rather than conflating "how bad is this finding"
with "how much should the tool print" into one enum.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
