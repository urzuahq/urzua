---
Stable-Id: 01M2PZJBP3TGY087GAFSNQ3F53
Status: Accepted
Embodiment: Not started
Realized-by: —
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
- **Eight specs describe a TOML config and must follow** — `SPEC-1`, `SPEC-2`, `SPEC-3`, `SPEC-5`,
  `SPEC-10`, `SPEC-15`, `SPEC-18`, `SPEC-19`, with `SPEC-3` being the configuration spec itself.

  **They are deliberately not updated by this decision.** They describe what ships today, and today
  that is TOML. Rewriting them now would make eight specs assert something the code contradicts —
  precisely the failure `MILE-94` records and this project spent a session repairing. They follow in
  the implementing change, and `Embodiment: Not started` on this record is the honest marker until
  then.
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
- MILE-94 -- why the eight specs are not rewritten ahead of the code.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Initial decision. **Why:** asked why TOML at all, rather than what good TOML syntax would be -- and the answer was that nothing chose it against YAML, while records had already chosen YAML. The deciding evidence was mechanical: `urzua-core` carries both parsers already, YAML at ten call sites to TOML's two, so consolidating removes a dependency rather than adding one. | **structural** |
