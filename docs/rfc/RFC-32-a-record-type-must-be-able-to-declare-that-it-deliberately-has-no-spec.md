---
Stable-Id: 01M2P88HWDE7A083501VRYXPZ1
Status: Draft
Date: 2026-09-16
Author: beauwilliams
---
# 32 — A record type must be able to declare that it deliberately has no spec

## Summary

`type.no-declared-spec` warns for every record type with no `spec` pointer. `spec` is
`Option<String>` and there is no value meaning "deliberately none", so an adopted corpus carries a
finding it cannot answer — and the only ways to silence it are a fabricated pointer or a waiver, both
of which are the escape hatch `MILE-51` forbids.

## Motivation

From `MILE-51`. After adopting `npryce/adr-tools`, `check` reports:

```
1 warning  type.no-declared-spec
```

`init` never writes a `spec` key (`init.rs:89-98`), so this fires for **every** adopted type on the
very first run. That is the tool's first impression of a new corpus: a finding about a concept the
adopter has not encountered, with no documented way to resolve it.

`ADR-41`/`ADR-43` are explicit that this is *"a signal to review, never a mandate; a type can
permanently have none declared."* The rule has no way to record that a reviewer has reviewed and
decided. It re-signals on every run, forever.

Three ways to silence it today, all wrong:

| | Why it fails |
|---|---|
| Write `spec = "SPEC-N"` | Fabricates a pointer to a record that does not exist; `pointer.resolution` then reports it as dangling |
| Write a waiver record | Puts a governance record into someone else's corpus to quiet a rule about governance records |
| Leave it | Permanent noise, and the "flood" that makes a finding list unreadable |

## Proposal

Sketch, not settled. Widen the field so absence and decision are distinguishable:

```toml
[record_types.adr]
spec = "none"        # decided: this type needs no spec
# spec omitted       # not yet considered -- keeps the current warning
```

That preserves `ADR-41`'s intent exactly — the signal still fires for a type nobody has thought
about, and stops once someone has.

## Open questions

- **Is a magic string right?** `spec = "none"` collides with a hypothetical record named `none`.
  `spec_declared = false` as a separate key is uglier but unambiguous. This repo has faced the same
  choice before: `Supersedes / Superseded-by: —` uses an em dash as an explicit "not applicable"
  (`field_state.rs:49-52`), which is a third option and already an established convention here.
- **Does this generalise?** `pointer.resolution`, `narrative-field.stale` and others all surface
  "this is absent" without a way to say "absent on purpose." A per-field "deliberately none" may be a
  schema-wide concept rather than one key — in which case this RFC is the wrong size.
- **Does `init` propose it?** Writing `spec = "none"` at adoption decides on the adopter's behalf
  something they have not thought about, which is exactly what `ADR-41` says the signal is for.
  Probably not — but then a fresh adoption always starts with one warning, and that should be a
  deliberate choice rather than an accident.
- **Severity.** `MILE-80` would let a repo downgrade this rule instead. That solves the noise without
  solving the expressiveness, and the two overlap enough to decide together.

## References

- MILE-51 -- the validation run; four sibling gaps.
- ADR-41, ADR-43 -- "a signal to review, never a mandate", which the current shape cannot honour.
- MILE-80 -- configurable severity; an adjacent answer to the noise half.
- RFC-31 -- the sibling gap from the same run.
- `field_state.rs` -- the em-dash "not applicable" convention already in use here.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Back to `Draft`. `ADR-51` accepted this and was rejected the same day: the remedy was disproportionate to one non-blocking warning, `MILE-80` covers it generically, and the per-type argument for a sentinel describes no real repository. The false doc comment that motivated it is corrected instead. **Why:** worth keeping open rather than rejecting outright -- the problem is real, only the proposed shape was wrong. | **substantive** |
> | 2026-09-16 | `Draft` -> `Accepted`, decided by `ADR-51`, since reversed. **Why:** decided before building, rather than patching around the gap -- the open questions this proposal listed are answered in the decision, including one it did not anticipate. | **substantive** |
> | 2026-09-16 | Initial proposal, `Status: Draft`. **Why:** this is the finding an adopted corpus cannot clear by any configuration, which makes it the sharpest counterexample to "one config, not a fork per repo" that `MILE-51` produced -- a rule with no expressible answer. | **structural** |
