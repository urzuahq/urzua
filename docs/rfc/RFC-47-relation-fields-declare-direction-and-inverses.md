---
Stable-Id: 01M6D5B1RDDZ5FJPDP635XB6W4
Status: Accepted
Date: 2026-11-02
Author: beauwilliams
---
# 47 — relation fields declare direction and inverses

## Summary

A relation field today carries no declared direction and no declared inverse. The one pair the engine
checks, `Supersedes / Superseded-by`, is a single combined header key whose value mixes both
directions, and its reciprocity rule is hardcoded to that one role. This RFC proposes a config-declared
**relation pair**: per pair, the forward field, its inverse, whether reciprocity is required, and
whether the pair may span record types, plus a way to read a legacy combined key without guessing
which direction each reference points. One declared mechanism then replaces the per-pair hardcoding
that `BUG-17` and `BUG-41` each record from a different side.

## Motivation

**1. A combined key hides direction.** `docs/adr/ADR-29-*.md` carries `Supersedes / Superseded-by:
ADR-28, ADR-45`. Read against its neighbours, that means "supersedes ADR-28, superseded by ADR-45":
`ADR-28` lists `ADR-29` and `ADR-45` lists `ADR-29`, so the three headers are consistent. Nothing in the
file or in `.urzua/config.yaml` says which reference is which; the config lists the key as one
`known_fields` entry; `relation_fields` is optional and not set, so the engine applies its built-in
`Supersession` role default to that one key. A reader recovers
direction only from the target's own header or from the target's status, which is exactly the
inference a tool should not leave to a human.

**2. Reciprocity is direction-blind and single-pair.** `relation.supersession-reciprocity`
(`supersession_reciprocity` in `rust/crates/urzua-core/src/rules.rs`) resolves each reference in the
role's field, then asks only whether the target's same-named field mentions this record's id. A
reference in the wrong direction passes as long as the other side mentions the record at all: if
`ADR-29` and `ADR-45` each wrote the other in the combined field as "supersedes", both would be
satisfied. The rule is also reachable only through `RelationRole::Supersession`, a fixed role set
(`RFC-42`) with role-specific behaviour, so every other inverse pair is unchecked: `Amends` has no
inverse field or check (`BUG-17`), companion pairs are checked for one pair and hardcoded to it
(`BUG-41`), and a dependency pair such as `Depends-on` / `Feeds-into` has no check at all.
`MILE-122` (on the unmerged `round-28-bootstrap-backlog` branch) records the generalisation as planned
work; this RFC is the design it needs first.

**3. Field semantics differ.** Supersession and amendment are lineage: they say a record replaced or
corrected another, and usually pair a terminal status on the older record. A dependency says one record
needs another to be done. Applying one reciprocity or staleness policy to both is wrong. A parallel
change adds a `fields:` scope to `pointer.target-status`, so that rule can already be pointed at a
named subset of fields; that is the narrow precedent for field-scoped policy, and this RFC does not
duplicate it. It proposes what that scoping lacks: a declared relationship *between* two fields.

The thesis in `AGENTS.md` applies: a hardcoded pair is a type-specific assumption in Rust source.

## Proposal

**A `relation_pairs` declaration in config** (name open), one entry per pair:

```yaml
relation_pairs:
  - name: supersession
    forward: "Supersedes"        # this record -> the record it replaces
    inverse: "Superseded-by"     # the replaced record -> its replacement
    reciprocity: required        # required | advisory | none
    cross_type: true             # the two ends may be different record types
  - name: amendment
    forward: "Amends"
    inverse: "Amended-by"
    reciprocity: required
    cross_type: false
```

- **forward / inverse** name header fields. A claim `A.forward = B` is echoed by `B.inverse = A`, and the
  reverse. Direction is the field name, so it is machine-readable by construction.
- **Per-type resolution (unresolved).** Two types may need to spell a pair differently. The existing
  `relation_fields` maps only the fixed `RelationRole` values, so it cannot bind an arbitrary pair's
  forward and inverse to per-type field names. How that binding is declared is an open question
  below; this RFC does not claim the current machinery provides it.
- **reciprocity** sets the severity ceiling of the new check; `none` declares direction only.
- **cross_type** says whether the target of a claim may be a different record type. A claim to a type
  not allowed by the pair is excluded from the reciprocity check as out of scope (disclosed in its
  population), and is reported by a separate rule, `relation.cross-type-disallowed`, so it is never a
  silent skip.
- **A direction-mismatch check**, `relation.direction-mismatch`: a record names a target in its
  `forward` field, and the target's `forward` field (not `inverse`) also names the record. Today's rule
  cannot see this; with directions declared it is a plain set comparison.
- **`relation.supersession-reciprocity` becomes the `supersession` pair** of the general rule, keeping
  its id as an alias for one release: config validation accepts the old id, execution and reports
  name the canonical rule, and waiver matching treats the two ids as the same rule, so existing
  configs and waivers keep resolving.
- Both check rules disclose their `Population` per `RFC-45`, in one unit: claims. Examined claims,
  claims out of scope (target type outside `cross_type`) and claims on records with unreadable headers
  are counts in that population. The number of declared pairs is a configuration fact, reported as a
  separate metric beside the population, not as a population count.
- A declared pair whose field is absent from a type's `known_fields` is a config finding, mirroring
  `config.relation-field-not-known`.

**Handling the combined key.** Three options, kept open (see Open questions) with a leaning:

- *Migrate to two keys.* The corpus rewrites `Supersedes / Superseded-by` into `Supersedes` and
  `Superseded-by`. Unambiguous, but a breaking rewrite of Accepted ADRs' headers, which are frozen.
- *A declared ordering convention.* A pair may declare `combined: "Supersedes / Superseded-by"` with
  the rule that references before a marker (or in a fixed position) are forward. Requires a delimiter
  the current values do not contain, so it too means rewriting values.
- *A parser that derives direction from the reciprocal side.* Without rewriting anything, a reference
  `X` in the combined field, where `X`'s own combined field lists this record, is forward
  (`Supersedes`) when this record is the newer of the two and inverse (`Superseded-by`) when it is the
  older, or by status. Works on today's corpus but derives a fact from
  data that is itself under check, so a wrong header can validate itself.

The leaning is to declare two keys for new pairs and accept a legacy `combined` key as read-only input
that yields a **direction-unknown** claim: reciprocity (the symmetric check) still runs, the
direction-mismatch check reports it as unverifiable rather than passing it, and `urzua migrate` offers
the split as a tool-authored repair (`RFC-8`). Direction-unknown is a third state, distinct from
correct and from wrong, in the spirit of the three-field-states rule in `AGENTS.md`.

### Alternatives considered

- **Add more fixed `RelationRole` variants** (`Amendment`, `Dependency`, ...). Rejected: each is
  another hardcoded pair in Rust source, the exact shape `BUG-41` records; it scales per pair, not per
  config.
- **Generalise reciprocity to "any field mentioning a record names it back", no declared pair.**
  Rejected: it cannot tell a dependency from a lineage claim, and still has no direction.
- **Leave direction to prose.** Rejected: the combined key is the evidence that prose direction is
  not recoverable by a tool.
- **Infer direction from record status or date only.** Rejected as the sole mechanism: a stale or
  wrong status makes the check validate its own input.

## Open questions

- **Per-type binding.** How a type maps a pair's forward and inverse to its own field names, given
  that `relation_fields` binds only fixed roles today.
- **Config shape.** Top-level `relation_pairs`, or per-type under `relation_fields`? Pairs that span
  types suggest top-level; per-type spelling suggests the field names live with the type.
- **Does the fixed `RelationRole` set survive?** `Supersession` carries role-specific behaviour
  elsewhere in the engine. Folding it into a pair may mean the role becomes a pair with a well-known
  name, or stays and the pair generalises beside it.
- **Combined-key handling.** Which of the three options, and whether the legacy-key claim is
  direction-unknown or inferred. Frozen Accepted ADRs weigh against migration; correctness against
  inference.
- **Reciprocity severity and enforcement of Amends.** `BUG-17` has two fixes: add `Amended-by` and
  check it, or treat `Amends` as one-way. Which pairs are one-way by design (a record that depends on
  another need not be named by it)?
- **Three-way relations and non-record targets.** Whether a pair target may be an external reference
  or a set; whether many-to-many needs a count policy.
- **Write-time linking.** `MILE-43` asks the tool to write the missing side when a record supersedes
  another. Whether a declared pair is the substrate for that, or that stays a separate feature.
- **Relationship to `pointer.target-status`'s `fields:` scope.** Whether pair names can be used as
  that scope's vocabulary, or the two stay independent.

## Non-goals

- Does not change the `pointer_fields`/`narrative_fields` split (`ADR-44`); a pair's two fields keep
  whichever kind each is declared as.
- Does not verify the *meaning* of a relation, only that its declared shape and reciprocity hold.
- Does not rewrite any Accepted record; the migration path, if chosen, is an offered repair.
- Does not add rule primitives beyond the pair declaration (`RFC-33` governs how new primitives are
  admitted).

## References

- `RFC-33` — rule primitives; a declared pair is a new configurable primitive.
- `ADR-44` — pointer and narrative fields are config-declared per type; pairs build on that.
- `RFC-45` — population disclosure the new rules must follow.
- `RFC-42` — the fixed relation role set this RFC proposes generalising.
- `BUG-17` — `Amends` has no reciprocal field or check.
- `BUG-41` — companion-field pairs checked for one pair and hardcoded.
- `MILE-122` — the planned generalisation this RFC designs (on `round-28-bootstrap-backlog`).
- `MILE-43` — write-time reciprocity auto-linking.
- `docs/adr/ADR-28-*.md`, `ADR-29-*.md`, `ADR-45-*.md` — the evidence for the combined key.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed as `Draft`. **Why:** the combined `Supersedes / Superseded-by` key hides direction and only that one pair is reciprocity-checked. | **substantive** |
> | 2026-11-02 | Review corrections: the config's actual supersession declaration; per-type binding marked unresolved; one population unit; cross-type claims reported by a named rule; alias handling spelled out; reciprocal-side direction corrected; alternatives moved under Proposal. **Why:** review found each stated as more settled or different than the code. | **substantive** |
> | 2026-11-04 | `Draft` -> `Accepted`. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
