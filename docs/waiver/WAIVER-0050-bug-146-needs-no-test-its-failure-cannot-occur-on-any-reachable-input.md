---
Stable-Id: 01M6NKH4J5H0VKWEWHY5HV01TF
Rule: status.delivery-agreement
Scope: docs/bugs/BUG-146-init-silently-swallows-a-yaml-serialization-error-into-an-empty-config-body.md
Reason: "Fixed by replacing `unwrap_or_default()` with `unwrap()` on the YAML serialization in `render_config_yaml`. The value serialized is built only from plain strings, sequences and mappings of valid UTF-8, so serialization cannot fail on any input the function can be called with, and no test can make the `unwrap` fire."
Category: unreachable-failure
---
# WAIVER-0050 — BUG-146 needs no test: its failure cannot occur on any reachable input

## What this covers

`status.delivery-agreement` reports `BUG-146`: its `Status` is `Fixed`, but no `Realized-by` evidence computes to a delivered tier. Fixed by replacing `unwrap_or_default()` with `unwrap()` on the YAML serialization in `render_config_yaml`. The value serialized is built only from plain strings, sequences and mappings of valid UTF-8, so serialization cannot fail on any input the function can be called with, and no test can make the `unwrap` fire.

The waiver has no `Expires`: the cause is a permanent fact about the record, and the `permanent` declaration in `.urzua/config.yaml` allows it under `unreachable-failure`. It is reported by `waiver.matches-nothing` if the record gains evidence and the finding stops.

## References

- `docs/bugs/BUG-146-init-silently-swallows-a-yaml-serialization-error-into-an-empty-config-body.md` — the record whose status this waives.
- `ADR-84` — the rule; `ADR-86` — permanent waivers under a declared category; `MILE-193`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed with the `ADR-84` build (`status.delivery-agreement` on bugs). **Why:** the record is `Fixed` with a reason no test can cover, stated in `Reason`. | **substantive** |
