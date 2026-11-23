---
Stable-Id: 01M83BCJG7VB8XYRZ0EVTRK3QJ
Status: Proposed
Date: 2026-11-23
Author: beauwilliams
Deciders: beauwilliams
Embodiment:
Supersedes / Superseded-by:
---
# 106 — The check report's examined is an object of per-unit counts added beside files_examined

## Context

`MILE-358` asks `urzua check` to report what it examined as an object with one count per unit, so a consuming
gate that shares those unit names reads urzua's report without an adapter. Today the report carries a top-level
integer `files_examined` (`ADR-46`, `SPEC-2`, `SPEC-26`).

The maintainer accepted on 2026-11-23 that "the shared contract is urzua's existing shape, and urzua conforms by
addition only". A review of the alternative, changing the report's shape, found three costs:

- Consumers read the existing keys: `commands/hooks.rs` reads `next_action` with `as_str()`, and
  `harness/record.rs` reads `files_examined`. Renaming or retyping a key breaks them silently.
- `RFC-3` §3 and its non-goals deliberately leave a shared cross-tool status enum out; one shared vocabulary
  imposed on every report contradicts it.
- `ADR-46` rejected a shared envelope type; a contract that required one would reintroduce it.

The published agent CLI contract's gate profile requires `examined` as an object of non-negative integer counts
on every report whose `status` is not `not-run`.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Add `examined: {files: N}` beside `files_examined`, keep both | Nothing a consumer reads changes; a consuming gate reads `examined` directly | Two keys say the same number, and must never disagree |
| Replace `files_examined` with `examined` | One key | Breaks every reader of `files_examined`; a rename of the output contract |
| Make `files_examined` an object | One key | Retypes a key consumers read as an integer |
| Leave the report alone and have the gate adapt | No change here | Every gate carries a mapping for the one field that proves a check did not run on nothing |

## Decision

In the context of a consuming gate that reads `examined` as per-unit counts, facing a report whose existing key
is an integer others read, we decided that `urzua check` adds `examined`, an object of per-unit counts (`files`
now), beside `files_examined`, which stays with no removal date, to achieve a native reading of the report with
no consumer broken, accepting that two keys carry the same number.

- `examined.files` equals `files_examined` over the same corpus, in every report that prints `files_examined`.
- A report with `status: "not-run"` that could not start (it carries `error`) prints neither key; `examined` is not invented there. A `not-run` run that started and selected nothing prints both, at `0`.
- A unit `check` does not count is absent from the object, not `0`; a new unit is a later addition under this
  decision. The units a gate shares are `files`, `tests` and `packages`; `check` counts only `files`.
- This is the maintainer's addition-only rule (2026-11-23, "the shared contract is urzua's existing shape, and
  urzua conforms by addition only") applied to one key. It is not a decision to adopt a shared status enum or a
  shared envelope type.

## Reversibility

Adding a key is cheap to walk back before a consumer reads it and a breaking change after. `files_examined`
having no removal date is deliberate: removing it is a rename of the output contract and needs its own record.

## Consequences

- A consuming gate reads `examined.files` directly. Existing readers of `files_examined` are untouched.
- The two keys are one number written twice; a test asserts they agree, so a future change to how
  `files_examined` is counted cannot move one without the other.
- `urzua check`'s report validates against the published gate profile, which a vendored copy and a test
  show (`MILE-359`, fourth Done means item).

## References

- `MILE-358`, the work this records; `MILE-359`, which carries `schema_version` and `describe`.
- `ADR-46` (the shared output contract, no shared envelope); `RFC-3` §3 and its non-goals.
- `SPEC-2`, `SPEC-26`: where the report is described.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-23 | Drafted from `MILE-358`'s first Done means item, quoting the maintainer's addition-only acceptance of 2026-11-23. Left `Proposed`: acceptance is the maintainer's. | **structural** |
