---
Stable-Id: 01M260GHW11S1VEZ5EKMHQ2JBM
Status: Planned
Phase: '1'
Track: embodiment-model
Implements: ADR-18
Blocked-on: —
---
# 93 — Build the claim record type for embodiment.locator-promotion-candidate

## What

A `claim` record type -- its own directory, config entry, required-field set (a single locator plus
whichever records cite it) -- so `embodiment.locator-promotion-candidate`'s findings have somewhere
real to be promoted to. Deliberately the minimal slice: no AND/OR composite operators, no
weakest-link confidence, no cycle detection -- those stay `MILE-16`'s scope, gated on a real
multi-part-AND case that hasn't been found yet. This milestone only needs a place for "N records
independently repeat the same evidence locator" to become "N records point at one claim that states
it once."

## Why

`ADR-18` named this exact follow-up explicitly when it scoped the Embodiment MVP down from `RFC-5`'s
full claim graph: *"A `claim` record type needs a directory, a config entry, and its own
required-field set before promotion has anywhere to land -- tracked as follow-up work, not designed
here."* No milestone number was ever actually created for it, so the finding it names has had nowhere
to go since `ADR-18` shipped. Concretely compounding: `embodiment.locator-promotion-candidate`
currently reports 12 findings against this repo's own corpus (`rust/crates/urzua-core/src/rules.rs`
alone is cited by 11 different records), pure background noise on every `check` run with no action
available -- the exact "a rule that correctly finds something with no path to close it" gap this
project's own history keeps naming when left untracked.

Sequenced directly after `ADR-45`/`SPEC-20` (the CI/CD release-flow work) rather than left to compete
indefinitely with `RFC-22`/`MILE-87`/`MILE-88`/`MILE-89` -- the noise is real and growing with every
PR that adds another `Realized-by` citation, unlike those other open items which aren't actively
compounding.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-10 | Initial milestone, `Status: Planned`. Not yet scoped into a concrete schema for `claim`'s own required fields (the locator itself, which records cite it, and how `Realized-by` on a citing record points at a `claim` instead of a raw locator) -- that design decision comes before implementation starts. | **structural** |
