---
Stable-Id: 01M2PZJBP3TGY087GAFSNQ3F53
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/config.rs, code:rust/crates/urzua-cli/src/commands/init.rs, test:rust/crates/urzua-cli/src/commands/init.rs
Date: 2026-09-17
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: ADR-12
---
# 52 — One format for everything urzua reads and writes: YAML

## Context

Three formats were converging. Records carry YAML frontmatter (`ADR-17`/`RFC-16`). Config is TOML
(`ADR-12`). And `RFC-33`'s exploration pointed at JSON Schema for per-record header validation, which
would have made a third.

An adopter would meet all three on their first day: YAML in every record's frontmatter, TOML to
configure the tool, JSON to describe what a record must look like. Three syntaxes for one tool, none
of them chosen for a reason that survives being asked.

Three facts settle it.

**`urzua-core` already carries both parsers, and YAML is the load-bearing one.** `yaml_serde` has ten
call sites; `toml` has two, both reading config. Moving config to YAML does not add a dependency — it
**removes** one, and `toml` leaves the purity allowlist (`urzua-core/tests/purity.rs:9`).

**The YAML footguns are already accepted.** This project has been bitten twice — `BUG-12` (a broken
header reporting a generic message instead of the real parse error) and `BUG-6` (unescaped values in
generated YAML). Both came from *records*, which are YAML regardless of what config uses. A TOML
config does not reduce that exposure; it adds a second parser and a second mental model beside it.

**Every candidate rule model uses YAML or JSON; none uses TOML.** Vale's rules, Spectral's rulesets,
Semgrep's patterns, JSON Schema — the entire neighbourhood this tool lives in. Whichever model
`RFC-33` eventually lands on, the format answer is the same, which makes this decision safe to take
before that one.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| **YAML everywhere** | One format; drops a dependency; matches what records already are and what the surrounding ecosystem uses; comments work in schemas, which raw JSON cannot give | Indentation-sensitive for a config that is mostly flat key-values; YAML's own footguns, already accepted |
| Keep TOML, add JSON Schema files | Each format at what it is best at; smallest migration | Three formats, all met on day one. The problem this decision exists to remove |
| Everything JSON | One format; JSON Schema native | **No comments.** In a tool whose subject is recording *why*, a config that cannot say why a rule is enabled is disqualifying |
| Keep TOML, express schemas in TOML | One format | JSON Schema is deeply nested and JSON-native; TOML is at its worst there |

### The strongest counter-argument, tested

An adversarial review of `RFC-33`'s rule model closed with *"do it in TOML with named functions and
named keys -- because agents are the primary authors."* It argued from `OPA` issue #8426: 26% of Rego
users learn the language from an LLM, and maintainers collected reports that *"LLMs are not as helpful
for Rego coding, they seem to hallucinate more"* -- low corpus volume plus a syntax break poisoning the
training data for a language whose onramp is now a chatbot.

The argument is right and does not reach this decision. What it indicts is a **bespoke vocabulary with
no corpus** -- a hand-rolled DSL, or a noun x verb grid. Between TOML and YAML the corpus argument runs
the other way: YAML is the config language of GitHub Actions, Kubernetes, Docker Compose, OpenAPI,
Ansible, and of `Vale` and `Spectral` specifically. A model has read far more YAML config than TOML
config.

So the review's load-bearing requirement -- named functions, named keys, a declared options schema, and
a hard error listing valid keys when one is wrong -- is preserved exactly, and is if anything easier to
satisfy here.

## Decision

In the context of a tool that already reads YAML in every record and was about to acquire a third
format, we decided: **YAML is the one format for everything urzua reads and writes as data.**

`.urzua/config.toml` becomes `.urzua/config.yaml`. Any schema files are YAML-authored. Records are
unchanged — they already are YAML.

Markdown is unaffected: record *bodies* and templates stay markdown. This decision is about
structured data, not prose.

## Reversibility

Moderate. The config structs are `serde`-derived and format-agnostic, so the parser swap is two call
sites; the cost is in the corpus migration and the specs below. `ADR-12`'s `schema_version` survives
unchanged — it is a key in the document, not a property of the format, and it is what makes the
migration detectable rather than silent.

## Consequences

- **`toml` leaves `urzua-core`'s dependency allowlist.** One fewer parser to reason about inside the
  purity boundary (`ADR-5`).
- **`ADR-12` is amended, not superseded.** Its decision — a required `schema_version` from day one,
  in a config format about to exist in repos this project does not control — is exactly what makes
  this change survivable. The format changes; the versioning policy it established does not.
- **A flat config gets slightly worse to write.** `dir: docs/adr` under indentation is less pleasant
  than `dir = "docs/adr"`. Accepted knowingly: the cost is paid in one file, the benefit in every
  file an adopter touches.
- **Thirteen specs carry a TOML surface and must follow**, in three groups:

  | Group | Specs | What carries TOML |
  |---|---|---|
  | Config prose | `SPEC-1`, `SPEC-2`, `SPEC-3`, `SPEC-5`, `SPEC-10`, `SPEC-15`, `SPEC-18`, `SPEC-19` | Describe `.urzua/config.toml` by name and shape. `SPEC-3` is the configuration spec itself. |
  | Type schemas | `SPEC-6`, `SPEC-9`, `SPEC-16`, `SPEC-17` | A ```toml block showing that type's `[record_types.*]` declaration. |
  | Fixture format | `SPEC-4` | Defines `manifest.toml` and a per-case `<case-id>.toml` -- a **second urzua-owned TOML surface**, unbuilt, and one this decision reaches before it is written. |

  `SPEC-20` is **out of scope and stays TOML**: its references are `Cargo.toml`, `knope.toml` and
  `rust-toolchain.toml`, third-party files whose format this project does not choose. Named here so
  the follow-up change does not "fix" them.

  **None of these are updated by this decision.** They describe what ships today, and today that is
  TOML. Rewriting them now would make thirteen specs assert something the code contradicts --
  precisely the failure `MILE-94` records and this project spent a session repairing. They follow in
  the implementing change, and `Embodiment: Not started` on this record is the honest marker until
  then.

  Each one takes a **revision-log entry**, and the class differs by group: config prose and the
  fixture format are **substantive** (the described mechanism changes), type schemas are
  **structural** (the same declaration, re-rendered).

- **`urzua init` writes `config.yaml`**, and the adopt path (`SPEC-5`) changes with it.
- **An existing `config.toml` needs a migration path** — read-both-write-one, or a `migrate config`
  step. Undecided here; it affects exactly one repository today, which is the cheapest this will ever
  be.

## References

- ADR-12 -- `schema_version` from day one; amended, and the reason this migration is detectable.
- ADR-17, RFC-16 -- records are YAML frontmatter; the format this converges on.
- ADR-5 -- the purity allowlist `toml` leaves.
- BUG-6, BUG-12 -- YAML's real costs, both incurred through records rather than config.
- RFC-33 -- the exploration that surfaced the third format; this decision is independent of which
  rule model it lands on.
- MILE-94 -- why the thirteen specs are not rewritten ahead of the code.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Initial decision. **Why:** asked why TOML at all, rather than what good TOML syntax would be -- and the answer was that nothing chose it against YAML, while records had already chosen YAML. The deciding evidence was mechanical: `urzua-core` carries both parsers already, YAML at ten call sites to TOML's two, so consolidating removes a dependency rather than adding one. | **structural** |
> | 2026-09-17 | `Embodiment: Not started` → `Verified`, with `Realized-by` naming the code and the test. **Why:** the parser, `urzua init`'s renderer, the fourteen call sites and this repo's own config all moved to YAML, `toml` left both crates and the purity allowlist, and the thirteen specs this record named were updated in the same change -- so nothing here asserts what the code contradicts any more. | **substantive** |
