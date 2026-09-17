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

### 2. A declared document model, then rules as data

An earlier draft of this section proposed a **grid**: six nouns (`record`, `field`, `reference`,
`relation`, `section`, `identity`) crossed with five verbs (`exists`, `closed`, `conforms`, `resolves`,
`agrees`), so a rule name like `field.conforms` would be *derived* rather than remembered.

**The grid is withdrawn. It fails this RFC's own falsification test, on this RFC's own examples.**

Test #2 below asks to *"place every one of today's seventeen rules in the grid without a residual."* It
does not survive the six rule keys the draft itself wrote:

| key the draft wrote | legal cell? |
|---|---|
| `field.conforms` | yes |
| `section.conforms` | yes |
| `reference.target-status` | noun yes -- but `target-status` is not one of the five verbs |
| `header.conforms` | no: `header` is not one of the six nouns |
| `config.key-required` | no: neither half |
| `implements.target-not-settled` | no: neither half |

Two of six. A naming scheme whose own worked examples are 67% illegal is not making names predictable,
it is adding a second thing to remember. Recorded rather than deleted, because the failure is the
finding: the residuals are not exotic edge cases, they are ordinary checks the engine already performs.
The nouns were not the wrong nouns -- the premise that a rule's *name* should encode its *structure*
was wrong.

**What replaces it is not invented either.** `Spectral` -- a mature linter for a superficially
different subject (OpenAPI/JSON documents) but structurally the same problem (validate a declared
document model against user-configurable rules) -- uses `given` / `then` / `severity`, where `given`
selects and `then` names a function. The name carries no structure; the function does.

#### Layer 1: a declared document model

This is what `MILE-51` measured as missing, and it comes before any rule work. The engine cannot
express a foreign corpus's rules while it cannot express the corpus's *shape*:

```yaml
record_types:
  adr:
    dir: doc/adr
    identity:
      from: filename
      pattern: '^(?P<number>\d+)-(?P<slug>.+)$'
    fields:
      from: prefix-lines        # a bare `Date: 2016-02-12` line, not a header block
    sections:
      from: headings
      depth: 2                  # `##` and `###`
      items: true               # bullet lists within a section are addressable
```

Consequences, all of them subtractive:

- `header_shape`'s three-value enum becomes **one case** of `fields.from`.
- `ADR-50`'s `header_shape = "none"` -- Accepted and unbuilt -- becomes another case rather than a
  fourth enum variant.
- **`header.deprecated-shape` becomes unwritable**, because no value is blessed to deprecate against.
- `RFC-29` (filename/numbering convention) and `RFC-31` dissolve into `identity`.
- `RFC-17`'s section parser and `MILE-4`'s section checks get a home to attach to.

Without this layer, rule authors re-implement the document parser once per rule. That is not
speculative: it is what `Vale`'s Tengo scripting produced in the wild.

#### `sections` carries depth, because rules must be able to address the whole document

A record *is* a markdown file, and its `##`/`###` structure is content the rules are meant to reach --
"this type must carry a Consequences section", "these bullets must be classified". Whatever the model
cannot name, no rule can ever address, and no configuration can recover. So the model's reach is the
ceiling on extensibility, independent of how good the function vocabulary is. That is the general
claim; MADR is only what exposed the limit.

Falsification test #1 -- express a second foreign corpus on paper -- was **run against MADR**
(`adr.github.io/madr`, the main alternative to the Nygard shape) before any of this was built.

**The nine functions survived.** Every rule a MADR adopter would want maps onto one already listed:
`status` against its vocabulary is `enumeration`, *superseded by X* is `resolves` then `target-status`,
`decision-makers` is `pattern` with `split`/`each`, number uniqueness is `occurrence`. No tenth
function was needed, which is the vocabulary clearing its second corpus.

MADR's frontmatter is also **entirely optional**, which under all-opt-in means simply no rule -- the
first independent corroboration of that decision from outside this repo.

**Layer 1 did not survive, and `sections.from = "h2"` is why.** Two MADR rules cannot be written
against a flat section list:

1. *Every option in `## Considered Options` has a matching `### {option}` under `## Pros and Cons of
   the Options`.* The function is `agrees`; what is missing is that **`###` is not in the document
   model**, so the right-hand extractor cannot be named.
2. *Every consequence bullet reads `Good, because` / `Bad, because` / `Neutral, because`.* Needs list
   items inside a section to be addressable. They are not.

Both are expressible by regexing raw content -- which is a rule author re-implementing the parser, the
precise failure this layer exists to prevent. So `sections` gains `depth` and `items`, above -- not as
an accommodation for one template, but because a rule that cannot name a subsection cannot be written
by anyone, for any corpus.

**This gap is not only an adopter's.** Measured on this repo, 2026-09-17: **18 of 84 records use `###`
headings**, this RFC among them. A fifth of the corpus has structure `check` is blind to, which is why
no rule has ever fired on any of it.

Depth was invisible at n=1 by construction: neither this repo's checked structure nor the `adr-tools`
corpus nests, so nothing exercised it until a corpus that nests was tried.

#### Layers 2 and 3: rules as data

A rule is a selector, a named function, and a level. Nine functions, closed and versioned:

| function | asserts | options |
|---|---|---|
| `defined` / `undefined` | present / absent | -- |
| `closed` | the set is a subset of a declared list | `allowed` |
| `pattern` | the value matches a shape | `match`, `not_match`, `split`, `each`, `shape` |
| `enumeration` | the value is in a list | `values` |
| `resolves` | names a record that exists | -- |
| `target-status` | the resolved target's `Status` | `one_of` / `none_of` |
| `reciprocal` | A→B implies B→A | `inverse_field` |
| `agrees` | two extractors are equal | `left`, `right` |
| `occurrence` | a count across the corpus | `min`, `max` |

`occurrence` absorbs `embodiment.locator-promotion-candidate`, which the grid could not place without
inventing a sixth verb. It is an aggregate, and aggregates are a function, not a noun.

**Growth is one named function at a time, and there is never a general escape hatch.** Four surveyed
systems each tried one and retreated: `Semgrep` shipped arbitrary Python behind a flag named
`--dangerously-allow-arbitrary-code-execution-from-rules`, deprecated it, and deleted it -- its
replacement is a *closed* mini-grammar that a core maintainer describes as covering 95% of what the
hatch was used for. `Vale` removed external-executable rules in 2017 and returned them in 2021
sandboxed to near-uselessness. `JSON Schema` reserved the space in Core §7.9 and shipped nothing. `CUE`
never had one, deliberately.

**Rules must fail closed.** `resolves` against a missing target is a finding, never an absence, and a
selector matching zero records reports that it matched none. This is not a preference: `CUE` -- the
most mature system that solves this problem declaratively -- silently exits 0 on a dangling reference
if the schema omits `close()`, which is the most natural way to write it, and the behaviour is
undocumented. That is `is_terminal_status`'s `_ => &[]` reproduced in a mature tool, which is the exact
failure direction this RFC names as the quieter and worse one.

#### What today's noisiest rule becomes

`pointer.resolution` is one function doing two jobs (`rules.rs:548-620`). Run against this repo's own
corpus today:

```
total findings: 213
   172  pointer.resolution      <- 81% of the report
    26  embodiment.consistency
    15  embodiment.locator-promotion-candidate
```

**All 172 are success reports** -- `Implements: ADR-27 resolves; target Status = Accepted`. Four fifths
of what `check` prints is the tool announcing that a reference worked.

It splits into `resolves` (an error) and `target-status` (a query), and the second half is **deleted
rather than ported**. "This reference resolved and its target is Accepted" is a graph query, not a
finding. `urzua graph` already exists and is where it belongs.

#### Options take a declared schema, and an unknown key is a hard error

Every function's options are schema-declared, and a key that is not in the schema fails at load time
with the valid set named. This is `ESLint`'s `meta.schema` discipline, and it is worth more here than
there: this project is positioned as agent-native (`RFC-3`, `ADR-7`, `RFC-27`), so agents will be the
primary rule authors. `OPA`'s maintainers collected the failure mode -- 26% of Rego users learn the
language from an LLM, and the reports are that *"LLMs are not as helpful for Rego coding, they seem to
hallucinate more."* A wrong guess must be a load-time error listing the alternatives, never a silently
skipped check.

#### Consolidations, in order of value

- **5 → 1.** The five config-level rules become one `config.integrity`, parameterized over the
  `FieldKindSpec` table that already exists (`rules.rs:318-335`). `type.no-declared-spec` is *deleted*,
  not rewritten. `RFC-28`'s two proposed additions then come free.
- **3 → 1.** `narrative-field.stale`, `pointer.resolution`'s success half, and `RFC-28`'s proposed
  `implements.target-not-settled` are one function -- `target-status` -- with the terminal set written
  at the rule site. That finally kills `is_terminal_status`'s `_ => &[]`, because there is no central
  function left to fall through.
- **2 → 1.** `header.pointer-field-clean` and `field.quality` are both `pattern` over a declared shape.
- **1 → 0 new functions.** `relation.supersession-reciprocity` is `reciprocal`.

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

0. **Layer 1: the declared document model.** Moved to the front. `MILE-51` measured this as the
   blocker, not the rule vocabulary -- the engine could not read the foreign corpus at all, so no
   amount of rule configurability would have helped. It also subtracts before it adds: `ADR-50`'s
   unbuilt `"none"` and `header.deprecated-shape` both resolve into it rather than being built.
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

```yaml
rules:
  reference-resolves: error
  reference-target-status: off
  spec-declared:
    level: warn
    function: defined
```

Cargo's `[lints]` shape, chosen on ergonomics rather than familiarity: urzua ships as a binary and its
users are not Rust developers. The common case is one word; the named `level` key says what it means,
where ESLint's positional `["warn", {...}]` requires knowing the first slot is severity.

One correction, from a decision taken after this section was written: **the keys are rule *names*, not derived `noun.verb` cells.** Section 2 withdrew the grid, and this
section's own original example was part of the evidence -- `"header.conforms"` and
`"config.key-required"` are not cells in it.

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

- ~~**Express a second foreign corpus in the proposed config, on paper.**~~ **RUN against MADR**
  (2026-09-17), with a split result recorded in section 2: the nine functions passed and needed no
  tenth; `sections.from = "h2"` failed and gained `depth`/`items`. The test cost an afternoon and
  changed a primitive in an unimplemented proposal rather than in shipped Rust, which is what it was
  for. **A third corpus is now the open question** -- two is enough to catch overfitting to one, not
  enough to call the vocabulary closed.
- ~~**Try to place every one of today's seventeen rules in the grid without a residual.**~~ **RUN, and
  the grid failed it** (2026-09-17). It was not necessary to reach the seventeen: four of the six rule
  keys *this RFC itself wrote* are not legal cells. Section 2 records the table and withdraws the grid.
  The test cost one `grep` and was available from the day the section was written -- which is the
  argument for writing falsification tests concrete enough to actually run.
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
> | 2026-09-17 | Section 2's noun x verb grid withdrawn and replaced by a declared document model plus nine named functions over `given`/`then`/`level`. **Why:** the grid failed this RFC's own falsification test #2, on this RFC's own examples -- four of the six rule keys it wrote are not legal cells. The failure is kept in the section rather than deleted, because the residuals are ordinary checks the engine already performs, not edge cases. `MILE-51` measured the document model, not the rule vocabulary, as the blocker, so it moves to step 0. Adds two requirements the surveyed prior art makes non-optional: no general escape hatch ever (four engines shipped one and retreated), and rules fail closed (`CUE` silently exits 0 on a dangling reference without `close()`). Measured while writing: 172 of 213 findings on this repo's own corpus are `pointer.resolution` success reports, all of which the `resolves`/`target-status` split deletes. | **substantive** |
> | 2026-09-17 | Config examples converted to YAML per `ADR-52`, and the `[rules]` example's keys corrected. **Why:** this RFC describes config that does not exist yet, so unlike the specs -- which describe shipped TOML and deliberately stay as they are until the implementing change -- it should show the decided format. The same block still carried two withdrawn grid keys. A draft of this entry also proposed quoting a bare `off` against YAML 1.1's boolean reading; that was dropped after testing the parser, which follows the 1.2 core schema and returns `String("off")` -- nothing but urzua reads this file, so there is no 1.1 reader to defend against. | **substantive** |
> | 2026-09-17 | `sections` gains `depth` and `items`; falsification test #1 run against MADR and its result recorded. **Why:** the nine functions cleared a second foreign corpus without needing a tenth, but `sections.from = "h2"` could not express two ordinary MADR rules, because `###` and list items are not in the document model at all. The model's reach is the ceiling on extensibility -- what it cannot name, no rule can address and no configuration can recover -- so this is a general limit rather than one template's accommodation. Measured the same day: 18 of this repo's own 84 records use `###`, this RFC among them, and `check` is blind to all of it. | **substantive** |
