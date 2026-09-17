---
Stable-Id: 01M2Q31SRJKWBATJBX9CG69176
Status: Accepted
Embodiment: Not started
Realized-by: —
Date: 2026-09-17
Author: beauwilliams
Deciders: beauwilliams
Derives-from: RFC-33
Supersedes / Superseded-by: —
---
# 53 — A repository's governance is its configuration, not our binary

## Context

`check` ships seventeen rules. All of them are always on, none can be declined, and several encode
governance choices *this project* made rather than anything intrinsic to decision records. Two fire
against a corpus of **zero records** -- `type.no-declared-spec` and `header.deprecated-shape` -- run
directly against an empty corpus, not reasoned about.

The founding claim is *"one config, not a fork per repo."* `MILE-51` tested it against the first
foreign corpus, `npryce/adr-tools`, and it **failed**:

| | Result |
|---|---|
| `urzua init` | Cannot run -- `docs/` is hardcoded; that repo uses `doc/adr/` |
| Hand-written config | **9 blocking errors**, none of which that corpus asked for |
| Rules examining zero records | **10 of 17** |

Every record errored with *"no header-shaped region found -- required fields [] cannot be checked"*:
nothing was required, and it failed anyway.

The cause is not seventeen separate defects. It is one: **the engine holds governance opinions an
adopter cannot decline.** A tool whose subject is *recording decisions* was imposing undeclared ones.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| **Governance is configuration; the engine ships primitives** | An adopter declines what does not apply; the founding claim becomes testable rather than asserted | Thirteen rules are a rewrite; a migration abandoned halfway leaves the engine worse than either endpoint |
| Keep rules in Rust, add an ignore-list | Smallest change | The ad-hoc exclusion `RFC-11` and `RFC-13` both reject elsewhere. Also asymmetric: a rule can be silenced but never *added* without shipping a binary |
| Keep rules in Rust, add a profile per corpus style | No new config surface | A fork per repo, wearing a different name. Exactly the claim `MILE-51` falsified |
| Plugins -- compiled, loaded at runtime | Unbounded extensibility | Every surveyed engine that opened a general escape hatch retreated from it; see Consequences |

## Decision

In the context of an engine whose seventeen rules are compiled in and always on, facing a first
foreign corpus that received nine errors it never asked for, **we decided that a repository's
governance lives in its configuration and the engine ships only general-purpose primitives**, to make
*one config, not a fork per repo* a property the tool can actually hold, accepting that thirteen
rules must be rewritten and that a partial migration is worse than either endpoint.

Two things are decided here. A third is deliberately left open.

**Decided: every rule is a policy, and every policy is opt-in.** There is no structural/policy split.
That distinction was tested against a zero-record corpus and did not survive -- two rules fired
anyway. `ADR-7` still constrains *how*: a declined rule appears in `rules_executed` as deliberately
skipped, never absent, so "off" and "ran clean" stay distinguishable.

**Decided: the document model is declared, not assumed.** A record type declares how its identity,
fields and sections are found. `header_shape`'s enum becomes one case of `fields.from`; `ADR-50`'s
unbuilt `"none"` becomes another; `header.deprecated-shape` becomes **unwritable**, because no value
is blessed to deprecate against.

**Left open: the function vocabulary.** `RFC-33` proposes nine closed functions. They are not decided
here. See Reversibility.

## Reversibility

The principle is expensive to reverse and unlikely to need it -- it removes opinions rather than
adding them, and every alternative above is a variant of keeping them.

The **vocabulary** is a different matter, and is why it is not decided here. This project has already
run the *name-it-specifically → declare-it-per-type* migration once, in `ADR-40` → `ADR-44`, and
reversed the position this record also argues against. It ran a second time this month: `RFC-33`'s
original noun x verb grid was withdrawn after failing its own falsification test on its own examples.
Freezing nine functions on two corpora of evidence would be the third instance of the same mistake.

The cheap gate is on paper and outstanding: **a third corpus**. Two corpora catch overfitting to one;
they do not close a vocabulary.

## Consequences

- **Thirteen rules are rewritten and several are deleted rather than ported.** `pointer.resolution`
  is one function doing two jobs. Measured on this repo, 2026-09-17: **172 of 213 findings -- 81% of
  the entire report -- are its success reports**, `Implements: ADR-27 resolves; target Status =
  Accepted`. Four fifths of what `check` prints is the tool announcing that a reference worked. That
  half is a graph query, and `urzua graph` already exists.
- **`type.no-declared-spec` is deleted, not rewritten.** This repo keeps today's behaviour by
  declaring it, which is the whole point.
- **No general escape hatch, ever.** Not scripting, not a policy language, not compiled plugins. Four
  surveyed engines each shipped one and retreated: `Semgrep` guarded arbitrary Python behind a flag
  named `--dangerously-allow-arbitrary-code-execution-from-rules`, then deleted it; `Vale` removed
  external executables in 2017 and returned them in 2021 sandboxed to near-uselessness; `JSON Schema`
  reserved the space in Core §7.9 and shipped nothing; `CUE` never had one. Growth is one named
  function at a time.
- **Rules fail closed.** A reference naming a record that does not exist is a finding, never an
  absence, and a selector matching zero records reports that it matched none. `CUE` -- the most mature
  system solving this problem declaratively -- silently exits 0 on a dangling reference when the
  schema omits `close()`, which is the most natural way to write it, and the behaviour is
  undocumented. That is `is_terminal_status`'s `_ => &[]` reproduced in a mature tool.
- **A rule's options are schema-declared, and an unknown key is a load-time error naming the valid
  set.** This project is agent-native (`RFC-3`, `ADR-7`, `RFC-27`), so agents will be the primary rule
  authors, and a wrong guess must fail loudly rather than skip a check silently.
- **`MILE-80` moves behind the document model.** `MILE-51` measured the model, not the rule
  vocabulary, as the blocker: the engine could not read the foreign corpus at all, so no amount of
  rule configurability would have reached it.
- **A configured rule set can be wrong in a new way.** Today a corpus cannot decline a check; after
  this it can decline one it needed. That is the cost of the decision, and it is accepted: an opinion
  an adopter can see and decline beats one they cannot.
- **`urzua check` on an empty config does nothing, loudly.** Under all-opt-in that is correct
  behaviour rather than a defect, and `doctor` is where a corpus learns it has configured nothing.

## References

- RFC-33 -- the proposal this decides, including the withdrawn grid and the surviving mechanism.
- MILE-51 -- the validation whose failure forced this.
- ADR-40, ADR-44, MILE-90, BUG-8 -- the same migration, already run once and reversed.
- ADR-7 -- a declined rule is reported as skipped, never absent.
- ADR-8 -- "treat the header as a closed structure", which the document model generalises.
- ADR-50 -- `header_shape = "none"`, Accepted and unbuilt, absorbed by `fields.from`.
- RFC-28, RFC-29, RFC-31 -- proposals that dissolve into the declared model.
- ADR-52 -- the format that config is written in, decided independently.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Initial decision, `Status: Accepted`. **Why:** `MILE-51` falsified the founding claim against the first foreign corpus, and the cause was one thing rather than seventeen -- the engine holds governance opinions an adopter cannot decline. Decides the principle and the declared document model; deliberately leaves the function vocabulary open, because `ADR-40` → `ADR-44` and `RFC-33`'s withdrawn grid are two prior instances of naming things specifically ahead of the evidence, and a third corpus is the cheap gate still outstanding. | **substantive** |
