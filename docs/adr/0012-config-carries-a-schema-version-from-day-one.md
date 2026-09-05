# 0012 — `.urzua/config.toml` carries a `schema_version` from day one

> Status: Accepted
> Embodiment: Not started
> Date: 2026-09-04
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: SPEC-0003

## Context

SPEC-0003 flagged "does the config need a version field?" as open, noting it costs nothing now and
is unaddable later without a migration — the moment v0 ships, `.urzua/config.toml` lives in repos
this project doesn't control, and every future format change becomes someone else's migration
without a version to key it on.

## Decision

In the context of a config format about to exist in repos this project doesn't control, facing the
choice between adding a version field now or if a breaking change is ever needed, we decided to add
**`schema_version = 1`** to `.urzua/config.toml` from its first shipped version, required rather
than optional: a config missing it fails to parse, and that failure is what `urzua doctor` surfaces
and explains, the same way it already surfaces any other parse failure.

## Reversibility

The field itself is cheap and additive. What it protects against — a config format change with no
way to detect which version a given file is — is expensive to retrofit once real configs exist in
repos this project doesn't control. This is exactly the "cheap now, unaddable later" case SPEC-0003
already named.

## Consequences

- `Config`'s deserializer requires `schema_version`; a config missing it is a parse concern surfaced
  by `doctor`, not silently defaulted.
- Every config-shape change from this point forward must consider whether it bumps
  `schema_version` — this is now a live question for every future schema PR, not a hypothetical one.
- `urzua init`'s generated config includes `schema_version = 1` from its first adopt-mode run.

## References

- SPEC-0003 — the open question this decides.
- `.urzua/config.toml` — this repository's own config, updated to carry the field.
