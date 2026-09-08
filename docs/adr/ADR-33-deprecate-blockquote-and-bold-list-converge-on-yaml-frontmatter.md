---
Status: Accepted
Embodiment: Not started
Date: 2026-09-07
Author: '@beauwilliams'
Deciders: '@beauwilliams'
Supersedes / Superseded-by: —
Derives-from: RFC-5 (Accepted), RFC-16 (Accepted), RFC-17 (Draft)
---
# 33 — Deprecate blockquote and bold-list; converge on YAML frontmatter

## Context

Three header shapes exist today: `blockquote` (`> Key: Value`), `bold-list` (`- **Key:** Value`),
and `yaml-frontmatter`. Once parsed, all three produce the identical internal `Header` structure —
every rule, `fix`, `explain`, and `graph` is already shape-agnostic. So the honest case for YAML
isn't "blockquote is unstructured text" — RFC-10's closed-header model already makes blockquote a
real, mechanically parsed, closed structure, not prose a regex scrapes.

The real, durable difference is what RFC-5 already committed this project to building:
`realized_by` as a claim graph — `AND`/`OR` composites, nested arbitrarily (RFC-5 §3). Blockquote
and bold-list are both fundamentally flat: one key, one value, on one line. Neither has a clean way
to express a nested composite without inventing a bespoke mini-language inside a value string —
exactly the kind of custom, fragile parsing surface RFC-10 was written to eliminate for the
header's own boundaries. YAML supports arbitrary nesting natively. Since RFC-5's fuller model is
a real, planned destination — not a hypothetical one, per its own MVP-staging language ("ships once a
real case is found needing it," not "never built") — the two flat shapes have a structural ceiling
the schema is already committed to eventually exceeding.

A secondary, smaller factor: blockquote/bold-list closure logic is real code this project wrote and
maintains itself (anchor detection, boundary detection, the `x-` escape hatch); YAML frontmatter
delegates parsing to a standard library, so there's less bespoke surface to keep correct forever.

## Decision

In the context of RFC-5's claim graph being a committed future destination that flat header
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
- **This repo's own `adr`/`rfc`/`spec` types migrate to `yaml-frontmatter`** via a one-time,
  unshipped conversion pass — not a new public `urzua` subcommand. Parsing support staying
  permanently means no adopter is under any actual pressure to migrate anything today; the only real
  forcing function right now is this repo dogfooding the shape it recommends. Building and
  committing to a general, permanently-maintained `migrate header-shape` command on the strength of
  that alone would be exactly the kind of speculative capability this project avoids elsewhere
  (RFC-5's claim graph, `fix` Tier 2/3, `migrate schema --apply` — all explicitly staged "once a
  real case demands it, not before"). A general command is not ruled out permanently — it's
  deferred until an actual external adopter asks for one, same as everything else on that list.
- **Full removal of blockquote/bold-list parsing is an explicit non-decision here** — a future
  major-version step, decided separately, once real adoption data exists on how much the deprecated
  shapes are still actually in use.

This also resolves RFC-17's open question ("should the tool drop blockquote support entirely") —
answered here as deprecate-with-a-migration-path, not immediate removal.

## Reversibility

The deprecation itself (a warning rule, a changed `init` default) is cheap and reversible. The
corpus migration is higher-stakes — 30+ real, accepted decision records, rewritten — which is exactly
why byte-preservation of everything below the header is a hard requirement of the one-time
conversion pass, verified before it's trusted on this repo's own real corpus, not assumed.

## Consequences

- `urzua new` currently has a bug (found designing this decision, tracked separately): it prefers an
  existing template over the configured `header_shape` unconditionally, so flipping this repo's
  config to `yaml-frontmatter` without fixing that first would make `new` silently emit unparseable
  records. **This must be fixed before this repo's own config is flipped.**
- Bold-list's own decision record (ADR-16) is not rewritten — it remains the accurate record of
  why bold-list was added, narrowed by this ADR the same way ADR-23 was narrowed by ADR-26,
  not silently edited.
- RFC-5's claim graph remains unbuilt — this ADR removes the *structural* obstacle (flat header
  shapes) to eventually building it, it does not schedule building it.

## Amendment (2026-09-08): migration scope widens to all six configured types

The original Decision scoped this repo's own conversion to `adr`/`rfc`/`spec` only, explicitly
excluding `milestone`/`bug`/`waiver` on the grounds that no adopter was under pressure to migrate
those. That reasoning held while the migration mechanism was still hypothetical. It no longer does:
building and verifying a real, byte-preserving conversion tool against this repo's own corpus (MILE-3)
is the same amount of work regardless of how many of the six configured types it's pointed at, and
maintaining two header shapes side by side in the same corpus indefinitely — three types converged,
three still `Blockquote` — has no justification of its own once the tool exists and is trusted.

**Scope widens to all six configured types**: `adr`, `rfc`, `spec`, `milestone`, `bug`, `waiver`.
Every consequence the original Decision named for `adr`/`rfc`/`spec` applies identically to the other
three: `header_layout` is removed for all of them (ADR-38's own amendment, same date), and
`milestone`/`bug`/`waiver`'s own schema specs (`SPEC-6`/`SPEC-9`/`SPEC-10`) each gain a revision-log
entry documenting the shape change, the same treatment the three newly-backfilled type specs
(`SPEC-16`/`SPEC-17`/`SPEC-18`) receive from the start.

Nothing else about the original Decision changes: parsing support for `blockquote`/`bold-list` stays
(still needed to read an external adopter's un-migrated corpus, and for `check` to run against a
first-time evaluator's existing docs); full removal of that parsing support remains its own,
separately-decided future step, now tracked as MILE-85 rather than left as prose only.

## References

- RFC-5 — the claim-graph destination this ADR's evidence rests on.
- RFC-10 — the closed-header model both flat shapes already satisfy, which is why "unstructured"
  was never the honest framing.
- RFC-16/ADR-17 — YAML frontmatter's original introduction, as one option among three; this ADR
  narrows that to the recommended option.
- RFC-17 — the section-content-checks RFC whose open question this ADR resolves.
- ADR-16 — the bold-list decision this ADR deprecates without rewriting.
- ADR-38 — amended the same day, removing `header_layout` once this amendment's widened scope
  empties that ADR's own declared axis.
- MILE-85 — the deferred full-removal decision this amendment names explicitly rather than leaving
  as unstructured prose.
