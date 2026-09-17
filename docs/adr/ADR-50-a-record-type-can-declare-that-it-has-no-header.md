---
Stable-Id: 01M2PCCC1HH2K67J9WRATN8BQP
Status: Accepted
Embodiment: Not started
Realized-by: —
Date: 2026-09-16
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: RFC-31
---
# 50 — A record type can declare that it has no header

## Context

`header_shape` offers `yaml-frontmatter`, `blockquote`, `bold-list`. A corpus whose records carry no
header block at all cannot be described, and `MILE-51` measured the cost: nine valid Nygard records,
nine blocking errors, `blocking: true`.

`required_fields = []` does not express it. `header.required-fields` fails on the missing *region*,
not on unsatisfied fields. And silencing that case wholesale is not available either:
`check_integration.rs:380-416` asserts the opposite deliberately, because `init` writes
`required_fields = []` and `header_shape = "yaml-frontmatter"` for every adopted corpus (`ADR-33`) —
so that finding is how an adopter learns their declared shape does not match their records.

Two situations the tool must tell apart and currently cannot:

| | Should be |
|---|---|
| "my records have no header" | quiet |
| "my declared shape does not match my records" | loud |

Only a declaration separates them, because both look identical from inside the parser: no region found.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| **A fourth `header_shape` value, `none`** | Declares the fact directly; the mismatch case stays loud because declaring `yaml-frontmatter` on a headerless corpus is still a mismatch | Touches shape dispatch, one existing rule, and adds a config-validation rule |
| Treat `required_fields = []` as "no header expected" | No new schema | Inverts an existing test; deletes the adoption signal for its only audience; cannot distinguish the two rows above |
| Infer it — no region in any record means headerless | No config change | `RFC-10` is explicit that shape is declared, never sniffed, and inference would ratify a corpus that is merely broken |
| Leave it | Nothing to build | The founding claim stays false for an entire family of corpora |

## Decision

In the context of a corpus family the schema cannot describe at all, we decided: **`header_shape`
accepts `none`, meaning this type's records carry no header block.**

Under `none`:

- **`header.required-fields` skips the type**, rather than reporting a missing region. This matches
  what `header.layout-consistency` and `header.field-set-consistency` already do for a type that
  declares nothing — skip before incrementing, so `records_examined` stays 0 and the report does not
  claim coverage it did not perform (`ADR-7`).
- **`required_fields` must be empty.** Requiring a field of a type that has nowhere to put one is a
  contradiction, and a new config-level rule reports it — the same shape as
  `config.pointer-narrative-overlap`, which exists for exactly this class of self-contradictory
  declaration.

## Consequences

- **`header.deprecated-shape` must stop being a negative test.** It reads
  `if type_config.header_shape != HeaderShape::YamlFrontmatter` (`rules.rs:263`), so a `none` type
  would immediately be reported as using a deprecated shape. It isn't deprecated; it is a different
  axis. The rule becomes an explicit enumeration of the two deprecated shapes. **Without this the
  feature ships broken**, and it is the kind of thing a negative test hides until a fourth variant
  arrives.
- **A `none` type is checkable but not writable.** `urzua new` renders a header; `migrate ids`
  backfills a `Stable-Id` into one. Neither has anywhere to write for a `none` type, so both must
  refuse with a clear reason rather than producing a record with no identity. That is a real
  narrowing: such a corpus can be linted and cannot be generated into.
- **`init` is unchanged.** `ADR-33` has adopt mode always propose `yaml-frontmatter` — "where to
  grow", not a preservation of what exists — and that reasoning is not revisited here. The change is
  that an adopter can now *fix* it: one config line, where previously no edit could express their
  corpus. Whether adopt mode should propose `none` when it observes no header region is a separate
  decision about what adoption is for, and is left open.
- **`RFC-10`'s declared-never-sniffed rule is preserved.** This adds a value to declare, not a
  detection mechanism.
- **It does not make a Nygard corpus *checked*.** It makes it quiet. The metadata still lives in
  sections nothing reads (`MILE-4`) and a filename convention nothing can express (`RFC-29`). This
  closes one of `MILE-51`'s five gaps.

## Reversibility

Remove the variant and the config-validation rule; any config declaring `none` then fails to parse,
which is a loud failure rather than a silent behaviour change. No data format changes.

## References

- RFC-31 -- the proposal this decides.
- MILE-51 -- the validation run that measured the cost; gap 1 of five.
- ADR-33 -- `init` always proposes `yaml-frontmatter`; deliberately unchanged here.
- RFC-10 -- shape is declared, never sniffed.
- ADR-7 -- `records_examined` must not claim coverage that did not happen.
- ADR-8 -- the closed-header model `none` opts a type out of entirely.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Initial decision. **Why:** decided before building, because the obvious implementation (`header.deprecated-shape`'s negative test) would have flagged the new variant as deprecated on its first run -- found by reading the rule while writing this, not while debugging it. | **structural** |
