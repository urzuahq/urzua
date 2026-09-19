---
Stable-Id: 01M2VHJHA4F5PMMA9DGW0CM6Y5
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/discovery.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
Date: 2026-09-19
Author: beauwilliams
Deciders: beauwilliams
Derives-from: ADR-52
Supersedes / Superseded-by: —
---
# 54 — A break is a break: no migration diagnostics for formats we no longer read

## Context

`ADR-52` moved the config from TOML to YAML as a hard break. Nothing parses TOML; `toml` is not a
dependency.

A pre-release review then found that `init` and `doctor` did not *notice* a leftover `config.toml`
(`BUG-47`). `doctor` advised running `init`, and `init` wrote a freshly auto-detected config beside
the existing one, leaving its `required_fields`, `known_fields`, `pointer_fields` and `spec` pointers
ignored, and exited 0. The first fix added a four-line detector shared by three commands.

It reads nothing and parses nothing. It exists solely to produce a better message. The question it
raises is whether that is worth carrying, and the answer turns on how many repositories it serves.

**Measured, 2026-09-19:**

| release | published | asset downloads |
|---|---|---|
| v0.3.0 | 2026-09-16 | **1** |
| v0.2.1 | 2026-09-16 | 4 |
| v0.1.0 | 2026-09-05 | 10 |

The population holding a TOML config is this repository, whose config was converted by hand three days
ago. The diagnostic serves nobody.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| **Remove it now** | No code encoding a dead format; nothing to find and be afraid of later | A future adopter on v0.3.x gets a less helpful message |
| Keep it, time-boxed to a release | Helps the hypothetical upgrader; has an end | The end is a date nobody will act on, and it is four lines of dead weight until then |
| Keep it indefinitely | Maximally helpful | Every format change accretes one, and none is ever removed |

## Decision

In the context of a format break whose migration population is measurably empty, facing four lines
that exist only to describe a format the tool no longer reads, **we decided to remove the diagnostic
and to treat this as the general rule: a break ships no migration aid**, to keep the codebase free of
knowledge about formats it does not support, accepting that an adopter upgrading across such a break
gets a plain "file not found" rather than an explanation.

The general rule matters more than this instance. `MILE-98` will change how fields, identity and
sections are declared, and `RFC-33` will change what a rule looks like. Each is a candidate for its
own compatibility shim, and *"we kept the last one"* is the argument that makes the second one
inevitable.

## Reversibility

Trivial, and that is part of the argument. The diagnostic is four lines and one message; if a real
adopter ever hits it, re-adding it costs an afternoon and arrives with evidence instead of a guess.

Removing it is cheap to undo. Keeping it is what compounds.

## Consequences

- **`BUG-47`'s defect is not reintroduced by this, but its fix is.** Nothing is deleted from disk
  either way: a leftover `config.toml` is ignored, not destroyed. What is lost is the *notice*, and
  the notice served a population of zero.
- **A future break must decide this again, and now has a default.** Not a precedent to follow blindly:
  a break with real adopters behind it is a different question, and this record exists so that
  question is asked with numbers rather than instinct.
- **`init` writing a config beside a leftover one is now ordinary behaviour.** A reader of `init.rs`
  will not find a guard explaining why; this record is the explanation.
- **The version gate still exists.** `schema_version` remains the mechanism that makes a break loud
  (`ADR-12`), and this decision does not touch it. A config from a *supported* schema version that
  cannot be read is still an error that names the version.

## References

- ADR-52 -- the break this follows from.
- ADR-12 -- `schema_version`, the mechanism that stays.
- BUG-47 -- the defect, and the fix this removes before it ships.
- MILE-98, RFC-33 -- the next two breaks, and why the general rule is worth settling now.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Initial decision, `Status: Accepted`. **Why:** a review fix added a legacy-config detector, and the question of whether to carry it turned on how many repositories it serves. v0.3.0 has one asset download three days after release, so the population is this repository and its config was converted by hand. Decided as a general rule rather than a one-off, because `MILE-98` and `RFC-33` are each a candidate for the same shim and "we kept the last one" is what makes the second inevitable. | **substantive** |
