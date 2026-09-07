# 0033 — Deprecate blockquote and bold-list; converge on YAML frontmatter

> Status: Accepted
> Embodiment: Not started
> Date: 2026-09-07
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0005 (Accepted), RFC-0016 (Accepted), RFC-0017 (Draft)

## Context

Three header shapes exist today: `blockquote` (`> Key: Value`), `bold-list` (`- **Key:** Value`),
and `yaml-frontmatter`. Once parsed, all three produce the identical internal `Header` structure —
every rule, `fix`, `explain`, and `graph` is already shape-agnostic. So the honest case for YAML
isn't "blockquote is unstructured text" — RFC-0010's closed-header model already makes blockquote a
real, mechanically parsed, closed structure, not prose a regex scrapes.

The real, durable difference is what RFC-0005 already committed this project to building:
`realized_by` as a claim graph — `AND`/`OR` composites, nested arbitrarily (RFC-0005 §3). Blockquote
and bold-list are both fundamentally flat: one key, one value, on one line. Neither has a clean way
to express a nested composite without inventing a bespoke mini-language inside a value string —
exactly the kind of custom, fragile parsing surface RFC-0010 was written to eliminate for the
header's own boundaries. YAML supports arbitrary nesting natively. Since RFC-0005's fuller model is
a real, planned destination — not a hypothetical one, per its own MVP-staging language ("ships once a
real case is found needing it," not "never built") — the two flat shapes have a structural ceiling
the schema is already committed to eventually exceeding.

A secondary, smaller factor: blockquote/bold-list closure logic is real code this project wrote and
maintains itself (anchor detection, boundary detection, the `x-` escape hatch); YAML frontmatter
delegates parsing to a standard library, so there's less bespoke surface to keep correct forever.

## Decision

In the context of RFC-0005's claim graph being a committed future destination that flat header
shapes cannot represent, we decided: **`blockquote` and `bold-list` are deprecated. `yaml-frontmatter`
is the one shape this project actively grows.** Deprecated, not removed — parsing support for both
stays, for two concrete reasons that don't disappear just because a shape is discouraged: a corpus
has to be *readable* before it can be migrated, and someone evaluating Urzua for the first time needs
`check` to run against their existing, unmodified docs before deciding whether to adopt anything.

Concretely:

- **`urzua init`'s adopt-mode proposes `header_shape = "yaml-frontmatter"`** for any newly-adopted
  record type going forward, regardless of what shape the existing corpus happens to use — it still
  detects and reports the corpus's current shape, but no longer recommends preserving it.
- **A new `check` rule, `header.deprecated-shape`**, warns (non-blocking) when a record type's
  configured shape isn't `yaml-frontmatter` — visible, not silent, without breaking anyone's CI on
  this decision alone.
- **`urzua migrate header-shape`** (new command, not yet built) performs the actual conversion:
  parse the existing shape's fields, re-emit as YAML frontmatter, byte-preserve everything below the
  header. Dry-run by default, `--apply` to write — the same UX precedent `migrate ids` already
  established for a header-region edit that must not touch body content.
- **This repo's own `adr`/`rfc`/`spec` types migrate to `yaml-frontmatter`** using that tool, once
  built — dogfooding the mechanism on 30+ real, already-accepted records is the actual proof it's
  safe, not a synthetic fixture.
- **Full removal of blockquote/bold-list parsing is an explicit non-decision here** — a future
  major-version step, decided separately, once real adoption data exists on how much the deprecated
  shapes are still actually in use.

This also resolves RFC-0017's open question ("should the tool drop blockquote support entirely") —
answered here as deprecate-with-a-migration-path, not immediate removal.

## Reversibility

The deprecation itself (a warning rule, a changed `init` default) is cheap and reversible. The
corpus migration is higher-stakes — 30+ real, accepted decision records, rewritten — which is exactly
why byte-preservation of everything below the header is a hard requirement of the migration tool,
verified before it's trusted on this repo's own real corpus, not assumed.

## Consequences

- `urzua new` currently has a bug (found designing this decision, tracked separately): it prefers an
  existing template over the configured `header_shape` unconditionally, so flipping this repo's
  config to `yaml-frontmatter` without fixing that first would make `new` silently emit unparseable
  records. **This must be fixed before this repo's own config is flipped.**
- Bold-list's own decision record (ADR-0016) is not rewritten — it remains the accurate record of
  why bold-list was added, narrowed by this ADR the same way ADR-0023 was narrowed by ADR-0026,
  not silently edited.
- RFC-0005's claim graph remains unbuilt — this ADR removes the *structural* obstacle (flat header
  shapes) to eventually building it, it does not schedule building it.

## References

- RFC-0005 — the claim-graph destination this ADR's evidence rests on.
- RFC-0010 — the closed-header model both flat shapes already satisfy, which is why "unstructured"
  was never the honest framing.
- RFC-0016/ADR-0017 — YAML frontmatter's original introduction, as one option among three; this ADR
  narrows that to the recommended option.
- RFC-0017 — the section-content-checks RFC whose open question this ADR resolves.
- ADR-0016 — the bold-list decision this ADR deprecates without rewriting.
