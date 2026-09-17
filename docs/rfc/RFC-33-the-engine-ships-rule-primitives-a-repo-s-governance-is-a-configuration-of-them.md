---
Stable-Id: 01M2PGCECD2YDR7V0KGNKBGAK7
Status: Draft
Date: 2026-09-16
Author: beauwilliams
---
# 33 — The engine ships rule primitives; a repo's governance is a configuration of them

## Summary

`check` ships seventeen rules, all of them always-on and none declinable. Several encode governance
choices *this project* made — two fire against a corpus of zero records. Propose one organising principle: **the engine ships general-purpose
primitives, and a repository's governance is a configuration of them.** Under it the seventeen become
eleven, five of today's rules collapse into one, and the corpus `MILE-51` could not express becomes
expressible with no code path of its own.

Proposed as a **Phase 0 requirement**, not Phase 1 work, for the reason in the phase section below.

## Motivation

### The claim being tested

*"One config, not a fork per repo."* `MILE-51` tested it against `npryce/adr-tools` and it failed. The
five gaps that run produced (`RFC-29`, `RFC-31`, `RFC-32`, `MILE-4`, `BUG-36`/`BUG-37`) were each filed
as a missing *config key*. That framing was too small. They share a cause: the engine holds opinions
it should not hold.

`type.no-declared-spec` is the clearest case. It warns when a record type has no `spec` key — when a
type is not documented by a spec record. That is this project's editorial practice. A Nygard adopter
has no such concept, and the rule reads `config: &Config` and iterates every declared type
(`rules.rs:213-219`), firing on the *existence of a type* while reading zero records. There is nothing
an adopter can not-declare to make it stop.

### The project has already proven this twice, and reversed itself once

**`ADR-40` (Accepted, 2026-09-07) refused to generalize**, adding `"Parent"` to `pointer_resolution`'s
field list and stating: *"this ADR doesn't generalize the rule to 'any field that looks like a
reference,' it names `Parent` specifically."* Six weeks later `ADR-44`/`MILE-90`/`BUG-8` overturned
that and made the whole list config-declared. **The arc named-specifically → declared-per-type has
already run once, end to end, and "name it specifically" lost.** The rules still holding hardcoded
field names, status vocabularies and token lists are residue of the position that lost.

**`ADR-50` (Accepted, today) records the failure mode in miniature.** `header.deprecated-shape` reads
`if type_config.header_shape != HeaderShape::YamlFrontmatter` (`rules.rs:263`), so adding any fourth
shape value makes it fire wrongly — *"without this the feature ships broken."* A rule written as *"≠
the one value this project blessed"* is not checking the corpus. It is checking whether the adopter
agrees with this project, and it breaks the moment the schema grows.

**And the `FieldKindSpec` table already promises this design.** Its doc comment (`rules.rs:314-317`):
*"A real third kind, if one is ever decided, is one new `const` here, never a new hardcoded
function."* `relation.supersession-reciprocity` is that third kind — reciprocal, symmetric — shipped
as a hardcoded function.

### Five copies of one loop

`type_no_declared_spec` (`rules.rs:213`), `header_deprecated_shape` (`:257`),
`config_pointer_declaration_missing` (`:380`), `config_pointer_field_not_known` (`:427`) and
`config_pointer_narrative_overlap` (`:488`) open with a byte-identical preamble — collect
`config.record_types.keys()`, sort, iterate, increment `examined`, attribute the finding to
`config_path`. Five hand-written copies of "assert a property of the config, per declared type."

**`RFC-28` proposes adding two more.** That is the evidence the primitive is real rather than a
tidying exercise: the same invariants are being re-implemented per axis.

### Two failure directions, and the quieter one is worse

Everything above is *noise* — a rule firing where no answer exists. The mirror is **silence**:

- `is_terminal_status` (`rules.rs:759-769`) matches on this repo's five type names with `_ => &[]`. A
  team using `Draft → In review → Ratified` gets **no error, no warning, nothing** — every rule keyed
  on terminal status silently stops applying.
- `relation.supersession-reciprocity` fires only on the exact literal `"Supersedes / Superseded-by"`.
- `revision-log.change-class-required` fires only on the literal `**Revision log**`.

`MILE-51` measured eleven of seventeen rules examining zero records against the adopted corpus. A rule
that silently does nothing is worse than one that complains, because nothing distinguishes it from a
rule that ran and found the corpus clean.

## Proposal

### 1. Three categories, not two

Forcing everything into primitive-vs-policy distorts two real things:

| | | Configurable? |
|---|---|---|
| **Rules** | the eleven primitives below | yes, entirely |
| **Input providers** | impure functions supplying data a pure rule consumes — today exactly one, `compute_drifted_records` (`discovery.rs:136`), which runs `git blame` | no; irreducible |
| **Leaf predicates** | pure content shapes (a y-statement, a classified table) | bespoke code, but *selected* by config |

`ADR-5`'s purity boundary already made this split: `embodiment_consistency`'s signature is
`(records, drifted: &HashSet<PathBuf>)` — the git work is already outside the rule. So the drift
*input* is irreducible; the *rule* is not.

### 2. An ontology, not a list

An earlier draft named eleven primitives. They were *invented*, and an invented list is one someone
has to remember. The rules operate over a small set of nouns, asking a small set of questions, so the
list should be **derived** from that grid instead.

**Nouns** — everything the engine can see: `record` (a file the config's `dir` claims), `field` (a
key/value in its header), `reference` (a field value naming another record), `relation` (a reference
with direction), `section` (a `##` block), `identity` (filename, number, title — how a record says
which one it is).

**Verbs** — everything a check can ask:

| verb | asks | today |
|---|---|---|
| **exists** | a declared thing is present — *nothing missing* | `header.required-fields` |
| **closed** | no undeclared things present — *nothing extra* | `header.field-set-consistency` |
| **conforms** | a value matches a declared **shape** | `field.quality`, `header.pointer-field-clean` |
| **resolves** | a reference points at something real | `pointer.resolution` |
| **agrees** | two derivable things match | `embodiment.consistency`, `filename.title-consistency`, `relation.supersession-reciprocity` |

`exists` and `closed` are duals, and `closed` is not invented here — `ADR-8` already names it: *"treat
the header as a closed structure."*

**`conforms` is parameterized by a declared shape, not split per rule.** `field.quality` and
`header.pointer-field-clean` are the same verb over different shapes (`non-empty`,
`reference-list`), which is why they collapse:

```toml
"field.conforms"   = { level = "error", shape = "reference-list" }
"section.conforms" = { level = "error", shape = "classified-table" }
```

That vocabulary *is* the leaf-predicate category named above. A new kind of content check is then
usually **a new shape, not a new rule** — `y-statement`, `classified-table`, `iso-date` become entries
in one list rather than functions in `rules.rs`.

The value of the grid is that a rule name becomes predictable rather than remembered, and a gap
becomes visible: `section.*` barely exists today (one hardcoded scan for the literal
`**Revision log**`), and `identity.*` is a single rule with `ADR-36`'s scheme baked in.

**One thing does not fit, and is named rather than forced.**
`embodiment.locator-promotion-candidate` asks *"is this value cited by two or more records?"* — an
aggregate over the corpus, not a property of one record. Either a sixth verb (`shared`) or an honest
admission that it is a different kind of thing.

Consolidations, in order of value:

- **5 → 1.** The five config-level rules become one `config.integrity`, parameterized over the
  `FieldKindSpec` table that already exists (`rules.rs:318-335`). `type.no-declared-spec` is *deleted*,
  not rewritten — this repo keeps today's behaviour with a declared key list. `RFC-28`'s two proposed
  additions then come free.
- **3 → 1.** `narrative-field.stale`, `pointer.resolution`'s success-report half, and `RFC-28`'s
  proposed `implements.target-not-settled` are one predicate — *is this reference's target terminal* —
  over a declared `terminal_statuses`. `RFC-28` already concedes this.
- **2 → 1.** `header.pointer-field-clean` and `field.quality` both ask "does this value match a
  declared shape," with the shapes inlined as Rust constants today.
- **1 → 0 new functions.** `relation.supersession-reciprocity` becomes a capability flag on the
  existing table.

**Two rules need no work at all**: `header.layout-consistency` and `header.field-set-consistency`
already skip an undeclared type before incrementing `examined`. They are the model the other fifteen
should copy.

### 3. Sequencing, and the one hard blocker

**A `[rules]` table is a parse error today.** `Config` carries `#[serde(deny_unknown_fields)]`
(`config.rs:13-18`). Verified by running it against a real config:

```
unknown field `rules`, expected `schema_version` or `record_types`
```

So `RFC-28` — which specifies a `[rules."implements.target-not-settled"]` block with a `severity` key
— is **unbuildable as written**. `MILE-80` owns that surface and has not designed it.

Proposed order, each step unblocking the next:

1. **`MILE-80`: the `[rules]` table.** Nothing else can switch itself on without it. A
   `schema_version` question under `ADR-12`.
2. **The 5 → 1 consolidation.** Deletes `type.no-declared-spec` and defuses `header.deprecated-shape`
   as a side effect, and pre-pays `RFC-28`'s cost.
3. **`ADR-50`'s `header_shape = "none"`** — already Accepted, unbuilt.
4. **`RFC-28`'s status vocabulary** — closes the loudest *silence* failure.
5. **Last, the two genuine redesigns**: `filename.title-consistency` (entangled with `ADR-3`,
   `urzua new`'s numbering, `BUG-32`, `BUG-37`) and `RFC-17`'s section parser.

## Why this is a Phase 0 requirement

`MILE-80` is **Phase 1**. `MILE-4` and `MILE-51` are **Phase 0**, and both depend on it. Phase 0
cannot close, because its prerequisite is scheduled after it.

That inversion is the mechanical answer to why this work keeps producing blockers behind blockers.
Each session finds a real defect, fixes what it can reach, and hits a dependency scheduled later. The
remedy is not better planning — the defects are being found by *running* the tool, which is the right
way to find them, and no plan survives that. The remedy is to **order by dependency depth rather than
by symptom severity**, and to move the root blocker into the phase that needs it.

Concretely: `MILE-80` moves to Phase 0, and this RFC becomes a Phase 0 exit condition alongside
`MILE-51`.

## The `[rules]` surface — decided 2026-09-17

Four questions were open when this was filed. All four are now answered, and the design is borrowed
from established linter practice rather than invented.

**1. Can a rule be turned off? Yes, any of them — no `forbid` level.** rustc's `forbid` exists so a
downstream config cannot relax an upstream one. Urzua has no config inheritance, so it would solve a
problem that does not exist; revisit if shared configs ever land. `ADR-7` still constrains *how*: a
disabled rule appears in `rules_executed` as deliberately skipped, never absent, so "off" and "ran
clean" stay distinguishable.

**2. Syntax: a scalar shorthand, widening to an inline table when a rule takes options.**

```toml
[rules]
"header.conforms"         = "error"
"reference.target-status" = "off"
"config.key-required"     = { level = "warn", keys = ["spec"] }
```

The shape Cargo's `[lints]` uses — but chosen on ergonomics, not familiarity. Urzua ships as a binary
and its users are not Rust developers. The common case is one word; the named `level` key says what it
means, where ESLint's positional `["warn", {...}]` requires knowing that the first slot is severity.

**3. `schema_version` bumps to 2.** `ADR-12` added the field from day one *"in the context of a config
format about to exist in repos this project doesn't control."* This is the first real breaking change
and the mechanism exists for it. An old binary then reports "schema_version 2 not supported" rather
than "unknown field `rules`" — the same failure, a better error.

**4. Every rule is a policy, and every policy is opt-in. Nothing runs undeclared.**

An earlier draft of this decision split the rules into "structural" (on by default) and "policy" (off),
with the test *can an adopter disagree and still be right?* **That split does not survive inspection.**
At least two of the eight rules it called structural carry this project's content: `field.quality`'s
placeholder list is ours (`TBD`, and two of *this repo's own retired conventions*,
`(project lead)`/`(session author)`), and `filename.title-consistency` assumes `ADR-36`'s numbering
scheme. The line was not where the draft drew it, and a line that has to be argued per rule is a line
someone must maintain and relitigate.

So: **there is one kind of rule, and it is opt-in.** A rule runs because a repository declared it,
never because the binary preferred it. That removes a classification entirely rather than getting it
right.

Measured, not asserted. Against a config declaring almost nothing and a corpus of **zero records**,
two rules still produced findings — `type.no-declared-spec` and `header.deprecated-shape`, both with
opinions about a corpus that does not exist. Every other rule was silent. Those two are the ones this
RFC already proposes deleting.

**`ADR-7`'s no-silent-no-op rule is the real constraint here, and it is satisfiable.** If nothing runs
undeclared, a corpus with no `[rules]` gets a green run that checked nothing — precisely the failure
this project exists to prevent. Three things keep that honest:

- `rules_executed` is empty and visibly so, which is what that field is for.
- A run with no rules enabled emits a `Notice` saying exactly that, rather than passing quietly.
- `urzua init` writes a starter `[rules]` table, so a fresh adoption is never empty by accident.

**"The Urzua way" becomes a named preset, not a hidden default.** Deferred, and the shape is the
industry's: `eslint:recommended` is opt-in, not implicit. This repo's five policy rules then live in
its own config or in a preset it publishes — visible, readable, and copyable by someone who wants the
same practices, instead of being implied by the binary.

## Still open

- **Is the eleven-primitive set right?** Derived from today's seventeen. A set derived from what
  corpora actually need might differ, and only a second and third adopted corpus would tell us.
- **Does this supersede `MILE-51`'s five gaps or subsume them?** `RFC-32` dissolves entirely — under
  this proposal that rule does not exist. The others become configurations rather than new keys.
- **Rule-family selection.** With eleven primitives a flat list is readable; Ruff's `select`/`ignore`
  over code prefixes is the prior art if it ever isn't. Not needed at this size.
- **Presets.** Named, shareable rule sets — the `eslint:recommended` shape — are how "the Urzua way"
  ships once rules are opt-in. Deliberately deferred: opt-in works without them, and a preset
  mechanism designed before anyone has asked for a second preset would be speculative.
- **What `urzua init` writes.** It must produce a starter `[rules]` table, but which rules? Writing
  everything reintroduces the flood; writing nothing reintroduces the silent no-op.

## Confidence, and what would falsify this

Recorded deliberately: this stays `Draft` because the evidence under it is uneven, and the parts are
not equally strong.

**Measured, and true regardless of what is decided here:**

- Two rules produce findings against a corpus of **zero records** (`type.no-declared-spec`,
  `header.deprecated-shape`) — run directly, not reasoned.
- Five config-level rules share a byte-identical preamble; `RFC-28` proposes two more copies.
- A `[rules]` table is a hard parse error under `deny_unknown_fields`, so `RFC-28` is unbuildable as
  written — verified by running it.
- `MILE-80` sat in Phase 1 while two Phase 0 milestones depended on it.
- `ADR-40` → `ADR-44`/`MILE-90`/`BUG-8` already ran the named-specifically → declared-per-type
  migration once, and reversed the position this RFC also argues against.

**Proposed, and resting on a single corpus:** the noun/verb grid, the 17 → 11 consolidation, and
opt-in as the posture. `MILE-51` ran against one foreign corpus. The ontology is derived from *today's
seventeen rules*, which is circular — a grid derived from what corpora actually need could differ.

**What would falsify it, cheaply and before any rewrite:**

- **Express a second foreign corpus in the proposed config, on paper.** MADR is the obvious candidate:
  YAML frontmatter, `decision-makers`, `consulted`/`informed` fields, a different section set. If a
  noun or verb is missing, the grid is wrong and one afternoon found it.
- **Try to place every one of today's seventeen rules in the grid without a residual.** One already
  does not fit (`locator-promotion-candidate`, an aggregate). A second residual would suggest the
  verbs are the wrong axis.
- **Check whether the consolidation makes a rule harder to explain.** `field.conforms` with
  `shape = "reference-list"` should be *more* legible than `header.pointer-field-clean`, not less. If
  reviewers find the parameterized form harder to reason about, the abstraction is not paying.

**The cost of being wrong is high**, which is why this is not queued for implementation: thirteen rules
are a rewrite, and a migration begun on one corpus's evidence and abandoned halfway leaves the engine
worse than either endpoint.

## What this does not propose

No implementation. No `schema_version` bump. No change to `ADR-5`'s purity boundary, `ADR-8`'s closed
header, or `ADR-44`'s declared-fields model — this generalizes `ADR-44`, it does not revisit it.

## References

- MILE-51 -- the validation run; its five gaps share the cause this RFC names.
- ADR-40, then ADR-44 / MILE-90 / BUG-8 -- the same migration, already completed once, against the
  position this RFC also argues against.
- ADR-50 -- `header.deprecated-shape`'s negative test; the failure mode in miniature.
- ADR-51 -- rejected; under this proposal its question does not arise.
- RFC-28 -- the closest existing statement of the opt-in model, and currently unbuildable.
- RFC-17 -- the section parser; the last redesign in the sequence.
- MILE-80 -- the root blocker, currently in the wrong phase.
- ADR-5, ADR-7, ADR-8, ADR-12, ADR-44 -- the boundaries this works within.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Initial proposal, `Status: Draft`. **Why:** `MILE-51`'s five gaps were each filed as a missing config key, which was the wrong size -- they share a cause, and the cause is that the engine holds governance opinions an adopter cannot decline. Filed as a Phase 0 requirement because the root blocker is scheduled in Phase 1 while Phase 0 depends on it, which is why this work keeps producing blockers behind blockers. | **structural** |
