---
Stable-Id: 01M2M4E71ZBRVZ216SEWXVQJPA
Status: Draft
Date: 2026-09-16
Author: beauwilliams
---
# 28 — Status lifecycle is declared, and realization claims are checked against it

## Summary

Two coupled proposals. **First**, a record type's status vocabulary — which values exist, and which
of them are terminal — becomes a declared `.urzua/config.toml` key, the same way `pointer_fields`
and `narrative_fields` became declared keys under `ADR-44`. It is hardcoded in Rust today, against
hardcoded type names, and that is the last axis of the schema still decided by the tool rather than
by the repository using it.

**Second**, a new, config-gated rule: a source file that claims to implement a record must name a
record whose status is terminal. Claiming to implement a `Draft` spec is a contradiction — either
the spec is no longer draft, or the code is not really its implementation — and today nothing says
so. This is the rule this repository actually needs, but it is not built as a fixed behaviour of the
engine: it is off unless a repository configures it on.

## Motivation

### The status vocabulary is hardcoded

`rules.rs::is_terminal_status` matches on hardcoded record-type names to reach a hardcoded status
list:

```rust
fn is_terminal_status(record_type: &str, status: &str) -> bool {
    let terminal: &[&str] = match record_type {
        "bug" => &["Fixed", "WontFix"],
        "adr" | "rfc" => &["Accepted", "Rejected", "Superseded"],
        "spec" => &["Accepted"],
        "milestone" => &["Done", "WontDo"],
        _ => &[],
    };
    terminal.contains(&status)
}
```

`config.rs` has no status key at all. So an organisation adopting this engine with a `Draft →
In review → Ratified` lifecycle, or a type named anything other than the five above, gets `_ => &[]`
— no status is terminal, and every rule keyed on terminal status silently stops applying to them.
Not an error, not a warning: silence. That is the identical failure `BUG-8` described for
relationship fields before `ADR-44`/`MILE-90` made them declared, and the same argument applies
unchanged. `SPEC-1` already lists "status enums" as configurable; the code never caught up.

### Nothing checks that implemented code points at a settled record

`urzua-cli/src/main.rs` declares, in its module header:

```rust
//! Implements: SPEC-0001, SPEC-0002, SPEC-0003
```

All three are `Status: Draft`. The CLI they describe is built, shipped, and covered by integration
tests. Five such claims exist across the crates (`urzua-agdr`, `urzua-cli`, `urzua-core`,
`urzua-id`, `urzua-io`), and no rule reads any of them. Meanwhile `SPEC-2` — which describes
`urzua check`, the most-built command in the tool, and is at version `0.8` after eight revisions —
is still `Draft` purely because nobody flipped it.

The effect is a usability failure, not a philosophical one: a contributor or agent reading `SPEC-5`
finds `urzua init --types adr,rfc,spec` documented as a supported invocation. Running it returns
`error: unexpected argument '--types' found`. The engine that exists to keep records true to the
code cannot currently detect that its own specs are describing a tool that does not exist.

There is a related, weaker signal already firing — `pointer_resolution` surfaces 27 live warnings of
the form `Parent: SPEC-1 resolves; target Status = Draft`. That is the record-to-record half of the
same question, and it is only a warning. It does not fail CI, and it says nothing about code.

### Why the record side alone is not enough

`RFC-5` §2 decided the record-side pointer (`Realized-by`) is the universal addressing mechanism,
rejecting in-code `Implements:` comments as a way to *discover* what governs a file. That decision
is not reopened here. But it leaves this specific case uncovered: `SPEC-2` through `SPEC-5` carry no
`Embodiment` or `Realized-by` field at all, and `SPEC-1` claims `Embodiment: Not started`. There is
no realization claim on the record to contradict its `Draft` status, so a purely record-side rule
would find nothing. The only mechanical evidence those specs are implemented lives in the source
files.

This proposal therefore reads source files for an existing claim and validates it — it does not
treat the comment as an addressing mechanism, and it does not make the comment required.

## Proposal

### 1. `statuses` and `terminal_statuses` become declared per type

```toml
[record_types.spec]
dir = "docs/specs"
statuses = ["Draft", "Accepted", "Superseded"]
terminal_statuses = ["Accepted"]
```

Same `Option<Vec<String>>` shape and "declared, not inferred" semantics as `known_fields` and
`pointer_fields`: absent means the engine has no opinion, never a silent built-in default. Two
config-level validation rules follow the three `ADR-44` already established:

- `config.terminal-status-not-declared` — every entry in `terminal_statuses` must also appear in
  `statuses`.
- `config.status-declaration-missing` — a type declaring one key must declare both, even as `[]`.

`is_terminal_status` loses its `match` and reads the declared list. Every existing rule keyed on
terminal status (`narrative_field_stale`, `pointer_resolution`'s status surfacing) changes input
source, not behaviour, once this repository's own config declares the values already hardcoded.

### 2. A new rule: `implements.target-not-settled`

For each source file carrying an `Implements: <refs>` claim, resolve each reference and compare the
target's status against that type's declared `terminal_statuses`.

- Reference does not resolve → Error. A claim pointing at nothing is the same defect
  `pointer_resolution` already treats as an error for record headers.
- Reference resolves, target status is not terminal → finding (severity configurable, see below).
- Type declares no `terminal_statuses` → rule does not fire for that target. No silent default.

**The rule is off unless configured.** It does not fire for a repository that has not opted in:

```toml
[rules."implements.target-not-settled"]
enabled = true
severity = "error"
source_globs = ["rust/crates/**/*.rs"]
```

`source_globs` matters: which files can carry a claim is a per-repository fact, not something the
engine should assume. A repository with no such convention configures nothing and the rule never
runs.

### 3. `check` reads declared source globs

This is the genuinely new capability and the reason this is an RFC rather than a config key.
`check` today reads only records. Under this proposal it additionally reads files matched by
`source_globs`, scanning for the claim pattern.

Constraints this must hold to:

- **Discovery stays git-tracked** (`ADR-6`): `source_globs` filters the already-discovered tracked
  file set, never a raw filesystem walk.
- **`urzua-core` stays pure** (`ADR-5`): the file read happens in `urzua-io`/`urzua-cli` and the
  parsed claims are handed to the rule as plain data, the same shape `full_text` and the drift set
  already use.
- **`rules_executed` stays honest** (`SPEC-2`): the rule reports how many source files it actually
  examined, so "zero findings" stays distinguishable from "never ran".

## Open questions

- **Is `Implements:` the right claim marker in source?** It matches the existing convention in these
  five files, but the pattern should probably itself be configurable, since a repository whose
  language or house style forbids that comment shape has no way in otherwise. Leaning configurable,
  undesigned here.
- **Should a non-terminal target be an error or a warning by default?** This proposal says
  configurable with no default, because the answer differs by repository maturity — a young corpus
  with many drafts would drown. But "configurable with no default" may just be deferring a decision
  every adopter then has to make cold.
- **Does this need a reciprocal check?** Nothing would detect a record claiming `Realized-by:
  code:foo.rs` where `foo.rs` carries no matching `Implements:`. Deliberately out of scope — that is
  the full bidirectional claim graph `RFC-5` already decided against, and nothing here revisits it.
- **What about `Superseded` targets?** `Superseded` is terminal for `adr`/`rfc` under the current
  hardcoded list, so code claiming to implement a superseded ADR would pass. That is arguably a
  second, different defect worth its own rule rather than overloading this one.

## Non-goals

- **Reopening `RFC-5` §2.** The record-side pointer remains the addressing mechanism. This validates
  a claim that already exists; it does not make in-code claims required, canonical, or a way to
  discover governance.
- **Making the status lifecycle mean anything to the engine beyond terminal/non-terminal.** No
  transition validation, no ordering, no workflow enforcement. `statuses` is declared so the engine
  can stop guessing, not so it can start managing a state machine.
- **Fixing this repository's own spec statuses.** That is corpus work, tracked as `BUG-26`, and it
  is deliberately separable: the rule is worth having even if the corpus were already clean, and the
  corpus is worth fixing even if this RFC is rejected.
- **Inferring implementation.** Nothing here decides whether code *actually* does what a record
  says. That remains a permanent ceiling (`SPEC-1`'s own structural-presence-is-not-content-scope
  point), and the drift signal for it is `ADR-32`'s git-blame mechanism, not this.

## References

- ADR-44 / MILE-90 / BUG-8 — the same "declared, not hardcoded" argument, applied to relationship
  fields; this RFC applies it to the one axis left.
- RFC-5 §2 — why the record-side pointer is the addressing mechanism, and the boundary this proposal
  stays inside.
- ADR-5 / ADR-6 — the purity boundary and git-tracked discovery constraint §3 must hold to.
- ADR-32 — git-blame drift detection, the existing answer to "the pointer exists but is it true".
- SPEC-1 — lists status enums as configurable; this RFC is the code catching up to that claim.
- SPEC-2 — `rules_executed` honesty requirement the new rule must satisfy.
- BUG-26 — this repository's own five Draft-but-implemented specs, the corpus half.
- `rust/crates/urzua-core/src/rules.rs` — `is_terminal_status`, the hardcoded list.
- `rust/crates/urzua-cli/src/main.rs` — the live `Implements: SPEC-0001, SPEC-0002, SPEC-0003` claim
  against three Draft specs.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Initial RFC, `Status: Draft`. | **structural** |
