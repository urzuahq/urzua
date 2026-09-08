# 80 — Configurable rule severity, per SPEC-1's own unbuilt promise

> Status: Planned
> Stable-Id: 01M1Z2T44DHPRE5C4DVENWXY58
> Phase: 1
> Track: schema-governance
> Implements: —
> Blocked-on: —

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

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
