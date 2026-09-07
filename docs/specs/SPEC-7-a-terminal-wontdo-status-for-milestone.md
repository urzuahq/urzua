# SPEC-7 — A terminal `WontDo` status for `milestone`

> Version: 0.1 | Date: 2026-09-07 | Status: Accepted
> **Implements:** ADR-34 (Amendment, 2026-09-07)
> **Parent:** SPEC-6 (the `milestone` record type).

## Purpose

Documents the `WontDo` addition to `milestone`'s `Status` enum as its own spec, rather than a
silent revision to SPEC-6 -- SPEC-6 already shipped and is `Accepted`; a schema addition after that
point is treated as new build-level detail under a new spec, not an edit to one already executed.
Whether that's the right general rule for every spec is exactly the open question this spec's own
References section tracks as unresolved (RFC-6's own admitted gap) -- this spec doesn't decide that
question, it's written under the assumption that it applies here.

## Schema

`Status` on `milestone` becomes:

```
Planned | InProgress | Blocked | Done | WontDo
```

`WontDo` is terminal, for a milestone deliberately deprioritized or decided against -- distinct
from:

- `Blocked`, which implies a specific, nameable blocker in the `Blocked on` section and an
  expectation of eventually moving.
- Leaving `Planned` in place, which misrepresents a reversed decision as still intended.

No `Superseded`-style pointer to a replacement exists at the milestone level; if a `WontDo`
milestone's work is picked up again later under a different plan, that's a new milestone with its
own `Implements`, not a resurrection of the old one.

## Why no code change

Checked directly against `rust/crates/urzua-core` before writing this spec: nothing validates
`Status` against an enum for any record type today. The four (now five) values are a documented
convention, not a code-enforced one, so this is a documentation-only addition -- the template
(`.urzua/templates/milestone.md`) and this spec are updated; `check`, `fix`, and every rule are
unaffected because none of them read `Status` against a closed set in the first place.

## What's deliberately not resolved here

- **Whether `Status` should be validated against a closed set at all, for any record type.**
  Adjacent to MILE-55 (roles/self-ack rule family), not the same scope, and not decided by this
  spec.
- **Whether "spec already executed, needs a new spec for further change" is the right general rule**
  for every future spec revision, or whether RFC-6's living-spec/revision-log model should still
  apply in other cases. Tracked as its own milestone (deciding RFC-6's own admittedly open
  question) rather than settled by precedent here.

## References

- ADR-34 -- the decision this amends: `milestone` as a configured type. See its 2026-09-07
  amendment for the `Status` change itself.
- SPEC-6 -- the parent spec this one adds to without editing.
- RFC-6 -- the living-spec/revision-log model, whose own Open Questions section already names
  "what does 'spec drifted enough to need a new ADR' look like operationally" as unresolved. This
  spec's own existence is a live instance of that exact question.
- `bug`'s `WontFix` and `adr`'s `Rejected`/`Superseded` -- the sibling terminal-status precedents
  this follows.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec. | **structural** |
