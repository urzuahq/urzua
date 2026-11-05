---
Stable-Id: 01M6NVYQ2ZVXST5R4NSQHBE5BR
Status: Proposed
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: ADR-85
---
# 88 — The closing-verb reading of claim.status-agreement can be turned off where the structured field is declared

## Context

`ADR-85` added a structured `closes` field to `claim.status-agreement` and, with `strict`, made it
required on every fragment. It kept the older reading, a closing verb followed by a reference in prose,
as "a deprecated convenience" that "stays on until a later decision removes it", and it has the report
disclose how many verb claims it still finds. This repository now has `closes` and `strict: true`
(`.urzua/config.yaml`, rule `claim.status-agreement`) and every fragment carries `closes:`. The
maintainer decided on 2026-11-05: strict mode on, structured data in and out.

With both readings running, a sentence can still make a finding the structured field did not. A
repository that has made the field its one claim has no way to say so: the prose reading cannot be
switched off, and it reads English, which the engine should not own (`BUG-59`, `BUG-65`, `MILE-98`).

**Measurement, taken before writing this decision.** The current verb reading, run over the 184 fragments
in `.changeset/` at `origin/main` (`0fa6d374`, built `urzua check`):

- It finds **69** closing-verb claims, in 18 fragments. This is the number the rule's own notice prints
  (`claim_field_notices`, `rust/crates/urzua-core/src/rules/claim.rs:341-352`). Per-claim detail came from
  a script that re-implements `claimed_closed` (`claim.rs:9-47`) and reproduces the total of 69 exactly.
- All 184 fragments carry `closes:`; none lacks it.
- **67** of the 69 name a record that the same fragment lists in its own `closes:`.
- **2** name a record `closes:` omits: `BUG-60` and `BUG-61` at `.changeset/claim-closes-field.md:8`,
  in the prose "write `closes: [BUG-60, BUG-61]`". That is the fragment documenting the option, and its
  `closes:` is `[BUG-66]`. These are not closures and not a wrong `closes` value: the reading took the
  key's own name followed by a list as the verb "closes" followed by two references. They raised no
  finding only because both records are `Fixed`. A different example id would have been a false error.

So turning the reading off here loses no claim, and the one place it disagrees with the field it is wrong.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Leave both readings on | Nothing to build | The prose reading can still contradict the field (the 2 above, harmless only by luck); the disclosure stays a permanent nag |
| B. Remove the verb reading from the engine now | Smallest engine | A repository without the field loses its only claim check; breaks the "undeclared behaves as today" opt-in (`ADR-53`); `ADR-85` said removal is a separate decision |
| C. A config option that turns the verb reading off, valid only with `closes` (chosen) | Opt-in; a repository that has made the field its claim says so; no new vocabulary | One more option to document and test |
| D. Make `strict: true` turn it off | No new option | Conflates "every fragment must carry the key" with "prose is not read"; a repository may want the first without the second |

## Decision

In the context of a repository whose release fragments all declare the records they close, facing a
prose reading that still runs beside the structured field and was measured to add 2 non-claims and no
claim, we decided to **add an option `verb_claims` to `claim.status-agreement` that, set to `false`,
turns the closing-verb reading off, valid only when `closes` is declared**, to achieve a rule whose
only claim is the structured field, accepting a new option and that a repository using it gives up the
prose reading by choice.

- **The option.** `verb_claims: true|false`, default `true`. An undeclared option behaves as today,
  so every existing configuration reports byte-identically.
- **Valid only with `closes`.** `verb_claims` is refused at load when `closes` is not declared on the
  rule, as a load error, the same place and kind as `strict` without `closes`
  (`rust/crates/urzua-core/src/config.rs:2548-2553`, `ConfigError::InvalidClaimField`). Declaring
  `verb_claims: true` without `closes` is refused too: the option is only meaningful beside the field,
  and an option that can never apply is refused, not ignored (the comment at `config.rs:2524-2526`). It
  is also refused on any other rule (`OptionNotApplicable`, `config.rs:2103-2113`).
- **Off means prose is never read for claims.** With `false`, no line of a fragment, frontmatter or body,
  is passed to `claimed_closed` (`claim.rs:93-98` and the `claim.rs:341-345` count both go through
  `verb_claim_lines`, `claim.rs:298-311`, which is where the option applies). Only the structured field
  is a claim. The census then holds listed ids and non-fragments only (`claim.rs:147-178`); the
  per-occurrence verb finding loop (`claim.rs:182-211`) has nothing to iterate.
- **The disclosure changes with it.** The notice at `claim.rs:346-352` says how many verb claims were
  found; with the option off it states that the reading is off and counts none, so a repository can see
  the setting took effect. It never silently drops the line.
- **Independent of `strict`.** `strict` governs whether every fragment must carry the key; `verb_claims`
  governs whether prose is read. They compose. Without `strict`, a fragment that lacks the key has no
  claim at all, which is the `BUG-66` shape `ADR-85` already said stays undetected there; a repository
  that turns prose off should keep `strict` on, and the specs say so.
- **This repository sets it.** `.urzua/config.yaml` gets `verb_claims: false`, since `strict` is on and
  all 184 fragments carry `closes:` (measured above).
- **Not decided here: removing the reading from the engine.** `claimed_closed` and the verb vocabulary
  stay in the code, on by default. Removing them is a separate, later decision and ADR, for when
  adopters can be shown not to need them (`ADR-85`, "Decision", the bullet on the deprecated reading).

## Reversibility

Easy. The option is additive and opt-in, default unchanged; setting it back to `true` or deleting it
restores the reading with no data change. The only hard-to-reverse part, the name `verb_claims`, is one
key in one rule's options.

## Consequences

- `SPEC-3` gains the option, its load error and its default; `SPEC-2` changes the rule's description
  (the reading "still runs" unless turned off) and the notice, each with a revision row.
- A test must fail before the change: with the option off, a fragment whose body says `Fixes BUG-n` for
  an open record raises no finding, while the same fragment's `closes: [BUG-n]` does.
- The two non-claims above stop being read here, so the fragment documenting the option can use any
  example id.
- A repository that turns the reading off and later re-enables it gets the 2026-11-05 behaviour again,
  including its false positives on documentation examples.
- The engine still owns a three-word English vocabulary (`claim.rs:10`) until the separate removal
  decision.

## References

- `ADR-85`: the structured field and the deprecated reading this option turns off.
- `ADR-81` (Rejected) and `RFC-36`: the prose-based remedy and its measurement, which is why prose is
  not trusted to carry a claim.
- `BUG-66`: the incident the field closes; `ADR-64`: structured in, structured out; `ADR-53`: opt-in.
- `BUG-59`, `BUG-65`, `MILE-98`: the vocabulary the engine should not own.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** Maintainer decision, 2026-11-05 ("we want strict mode on, and we want to use structured data in and out"); the maintainer accepts it in a later change. | **substantive** |
