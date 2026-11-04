---
Stable-Id: 01M3689KQV7S5AEX1VPCCJQ6VZ
Status: Fixed
Found-in: "Asked directly whether generated records were consistent; verified by running urzua new for every type in a scratch repo and diffing the raw output"
Regression-test: "rust/crates/urzua-cli/tests/own_config_integration.rs::new_opens_a_spec_title_with_its_declared_prefix_and_other_types_with_the_bare_number"
Blocked-on: —
---
# 123 — urzua new never generates the SPEC-N H1 prefix every existing spec file carries

## What was wrong

`new_record.rs:229` has exactly one H1 template, used identically for every record type:

```rust
out.push_str(&format!("# {} — {}\n", params.display_number, params.title));
```

No type-specific branching exists anywhere in `new_record.rs`. Verified directly: ran `urzua new` for
`spec`, `adr`, `rfc`, `bug`, and `milestone` in a disposable scratch repo and diffed the raw scaffolds.
Every one produced a bare-number H1 — `# 1 — Test spec scaffold`, `# 1 — Test adr scaffold`, etc. —
with no type prefix, for every type without exception.

But every `spec` record actually in this repository's corpus (`SPEC-1`, `SPEC-18`, `SPEC-21`, and
every other) carries an H1 reading `# SPEC-N — Title`, not `# N — Title`. `adr`/`rfc`/`bug`/
`milestone` records consistently use the bare-number form the generator produces. The `SPEC-N` prefix
on every existing spec was hand-typed after generation, at some point predating this session, and
never made consistent with what the tool itself writes.

## Why nothing caught it

`filename.title-consistency` (`rules.rs`) checks that a filename's claimed number matches the
document's H1 title — it compares the *number*, not the prefix text before it, so `# 18 — X` and
`# SPEC-18 — X` both satisfy it identically. Nothing in the rule set has ever compared a record's H1
prefix style against its own type's convention, or against the generator's own template, because no
rule was ever written to check the prefix text at all.

## What this needs before a fix

A design decision, not an obvious patch. Superseded by `RFC-44`, filed the same day: rather than
picking one of the three narrow directions this record originally listed (fix the generator to match
specs, backfill every other type to match specs, or drop the prefix from specs), the real gap sits one
level up — the document model has no field-type system at all, of which the H1's own identity shape is
one instance (`RFC-44`'s `identity` field type), alongside the already-separately-filed `BUG-32`
(the H1 number-extraction is dangerously permissive) and `SPEC-2`'s unbuilt `Status` enum validation.
This bug closes once `RFC-44` is decided and its `identity` type is implemented.

## References

- `rust/crates/urzua-core/src/new_record.rs:229` — the one H1 template, uniform across every type.
- `filename.title-consistency` (`rules.rs`) — checks the number, not the prefix text; the reason
  nothing caught this drift.
- `SPEC-22` — the new Rust-conventions spec whose own H1 (`# SPEC-22 — ...`) was written by hand to
  match the existing corpus, the exact hand-typing this bug is about.
- `RFC-44` — the general declared-field-type mechanism this bug's fix now belongs inside.
- `BUG-32` — the H1 number-extraction defect `RFC-44`'s `identity` type also closes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed, not fixed. **Why:** the user asked directly whether generated records were consistent; verified by running the actual generator for every type in a scratch repo rather than trusting memory, and the divergence turned out to be between the corpus's existing specs and the generator's own template, not between the tool's handling of different types. The fix direction is a real editorial/design decision, not an obvious patch. | **substantive** |
> | 2026-11-04 | Unblocked by `RFC-44`'s outcome ("declared per type"): each type declares its title form through `MILE-98`'s identity pattern, and this repository declares the `SPEC-` prefix for specs and the bare number for every other type. `Status` is unchanged. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | `Blocked-on` moved from `RFC-44` to `MILE-98`. **Why:** `RFC-44` is decided (declared per type); the fix now waits on the identity declaration it is built on. | **substantive** |
> | 2026-11-04 | Fixed. The `spec` type in `.urzua/config.yaml` declares `identity: {from: filename, pattern: '^(?P<prefix>SPEC)-(?P<number>\d+)-(?P<slug>.+)$'}`, so `urzua new spec` opens the title `# SPEC-N` (`SPEC-12`, `MILE-151`) and `filename.title-consistency` checks it; the other types declare none and keep the bare number, as the maintainer decided. `Blocked-on` cleared (`MILE-98` is Done). `Regression-test` filled. **Why:** the test ran `urzua new` for every type against this repository's own config and failed on the spec title before the declaration was added; the engine needed no change, the declaration was the fix. | **substantive** |
