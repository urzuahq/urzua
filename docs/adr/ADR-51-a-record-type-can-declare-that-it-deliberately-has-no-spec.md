---
Stable-Id: 01M2PCCCM8EJDJH9GCN2J8T257
Status: Accepted
Embodiment: Not started
Realized-by: —
Date: 2026-09-16
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: RFC-32
---
# 51 — A record type can declare that it deliberately has no spec

## Context

`type.no-declared-spec` warns for every type whose `spec` key is absent. `init` never writes one, so
it fires on every adopted corpus on the first run — the tool's first words to a new user are a finding
about a concept they have not met, with no documented way to answer it.

**The capability is already documented. It was never built.** `config.rs`'s own doc comment for the
field:

> Declared, not voted, same principle as `header_layout`/`known_fields`: omitted means no spec
> currently covers this type, which `type.no-declared-spec` surfaces as an inventory signal, not a
> mandate — **a type can permanently have none declared if that's the right editorial call.**

There is no way to record that call. `ADR-41`/`ADR-43` say the same thing — *"a signal to review,
never a mandate"* — and the rule cannot tell a reviewed type from an unreviewed one, so it re-signals
forever.

The comparison it draws is the sharpest evidence. `header_layout` is also `Option`, also "declared,
not voted", and its comment says *"omitted means this axis isn't checked for the type."* Two optional
keys, the same claimed principle, opposite behaviour: one is silent when absent, the other warns
permanently.

The three ways to silence it today are each worse than the finding:

| | Why not |
|---|---|
| `spec = "SPEC-N"` for a record that does not exist | `pointer.resolution` then reports it dangling — trading a warning for an error and a lie |
| A waiver record | Writes a governance record into someone else's corpus to quiet a rule about governance records |
| Leave it | Permanent noise; `MILE-51` found this is the one finding an adopted corpus cannot clear by any configuration |

## Options considered

| Option | Pros | Cons |
|---|---|---|
| **`spec = "none"`** | Directly records the editorial call the doc comment already promises; absence still means "not yet considered", preserving `ADR-41`'s signal | A sentinel string in a field that otherwise holds record IDs |
| A second key, `spec_declared = false` | Unambiguous | Two keys for one fact — the shape `RFC-1` §1c names as worse than one field answering two questions |
| Make absence silent, matching `header_layout` | Consistent; no new vocabulary | Destroys `ADR-41`'s inventory signal entirely — the rule exists to surface types nobody has thought about |
| Downgrade severity instead (`MILE-80`) | Solves the noise | Not the expressiveness: a quieter unanswerable finding is still unanswerable |

## Decision

In the context of a rule whose own field documentation promises an editorial call that cannot be
recorded, we decided: **`spec` accepts the value `none`, meaning this type deliberately has no spec.**

- `spec = "none"` → `type.no-declared-spec` does not fire. The call has been made.
- `spec` **absent** → it fires, unchanged. Nobody has considered it yet, which is exactly the signal
  `ADR-41` wants.

The sentinel is safe because the field holds a record ID — every real value in this repo is
`SPEC-16`, `SPEC-17`, `SPEC-18`. `none` is not a record-ID shape, so it cannot collide with a
pointer someone meant to write.

**Deliberately not generalised.** `RFC-32` asked whether "deliberately absent" is a schema-wide
concept, since `pointer.resolution` and `narrative-field.stale` also surface absences. There is one
instance today. Building a general mechanism for a pattern with a single member is the speculative
capability `AGENTS.md` prohibits; if a second key needs it, that is the evidence to generalise, and
this decision is cheap to fold into a broader one.

## Consequences

- **A newly adopted corpus can reach a clean run.** Combined with `ADR-50`, the two findings
  `MILE-51` measured against a Nygard corpus both become expressible in config — which is that
  milestone's literal criterion, met by widening the schema rather than by silencing a rule.
- **`init` does not write it.** Writing `spec = "none"` at adoption would decide on the adopter's
  behalf the exact editorial question `ADR-41` says the signal exists to prompt. A fresh adoption
  therefore still starts with one warning — deliberately, and now answerable in one line.
- **The doc comment stops being aspirational.** `config.rs`'s promise that "a type can permanently
  have none declared" becomes true; it should cite this decision rather than continuing to describe a
  capability by implication.
- **`MILE-80` is unaffected and still worth doing.** Configurable severity addresses noise in general;
  this addresses one finding's unanswerability. Neither substitutes for the other.

## Reversibility

Remove the sentinel handling; configs containing `spec = "none"` then resolve as a dangling pointer —
visible and loud, not silent. No data format change.

## References

- RFC-32 -- the proposal this decides.
- MILE-51 -- the validation run; this was gap 2 of five, and the only finding no configuration could clear.
- ADR-41, ADR-43 -- "a signal to review, never a mandate", which the current shape cannot honour.
- ADR-50 -- the sibling gap from the same run, decided alongside.
- RFC-1 §1c -- one field answering one question; why the second-key option was rejected.
- MILE-80 -- configurable severity; adjacent, not a substitute.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Initial decision. **Why:** the deciding evidence turned out to be in the code, not the corpus -- `config.rs`'s own doc comment already promises "a type can permanently have none declared" and nothing implements it, while the `header_layout` it cites as the same principle behaves oppositely. | **structural** |
