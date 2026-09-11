---
Stable-Id: 01M28H5J42CN3DB4DJGKMQND6B
Status: Open
Found-in: 'observed live while verifying BUG-18 -- ran `urzua new adr` to confirm the tool''s auto-filled `Author` matched the corpus form, and the record it produced carried `Status: null` and `Deciders: null`; reproduced deliberately with `urzua new bug`, which produced `Status: null`, `Found-in: null`, `Regression-test: null`'
Regression-test: 'not yet written -- fix scope not yet decided (see What was wrong: whether the synthesized path should splice the template''s header placeholders, carry an enumerated vocabulary from config, or stay blank by design is an open question, not an obvious patch)'
Implements: —
---
# 22 — urzua new's synthesized header emits YAML null, collapsing Placeholder into Blank

## What was wrong

`render_synthetic_yaml` (`rust/crates/urzua-core/src/new_record.rs:145-152`) fills `Date` and
`Author` with real values and emits every other required field as `Value::Null`:

```rust
let value = match field.as_str() {
    "Date" => Value::String(params.today.to_string()),
    "Author" => Value::String(params.author.to_string()),
    _ => Value::Null,
};
```

Reproduced directly with `urzua new bug`:

```
Stable-Id: 01M28H2KGQVARRF8CEE9CVB24J
Status: null
Found-in: null
Regression-test: null
```

The `bug` type has a checked-in template, and that template's header carries real, useful guidance —
`Status: Open | Fixed | WontFix`, `Found-in: how/where this was actually discovered`,
`Regression-test: path/name of the test proving this is fixed`. None of it survives. BUG-3's fix
made the configured `header_shape` win unconditionally over a template's shape, which was correct,
and `template_body` splices the template's `## ` body sections underneath — but the header half is
dropped and replaced with nulls. So for every `yaml-frontmatter` type, the enumerated vocabulary a
human needs in order to fill the field in is silently discarded.

The consequence that makes this more than cosmetic: `field_state::classify` reads the template path's
`Status: Open | Fixed | WontFix` as **Placeholder** (unedited template text) and the synthesized
path's `Status: null` as **Blank** (the YAML null extracts as absent). The same field on the same
type lands in two different field states purely according to which path produced it. Blank and
placeholder being distinct states is this project's own stated most-repeated bug class (AGENTS.md;
`field_state.rs:1-5`; BUG-5) — `urzua new`'s own output cannot currently express "pending, awaiting
the author," only "empty."

Scope honesty, checked before filing rather than assumed:

- **This is not a silent pass.** `check` does catch it. Staging the generated record and running
  `check docs/bugs/` reports three blocking `field.quality` errors — `field 'Status' is Blank -- not
  a real, present value`, and the same for `Found-in` and `Regression-test`. The defect is the
  field-state collapse and the lost vocabulary, not a missed finding.
- **SPEC-12 arguably describes this behaviour already.** Its path 2 says "every configured field
  emitted as a blank frontmatter key, so the config alone drives what an author has to fill in."
  A YAML `null` does behave as blank. So this may be a design question (should path 2 reach parity
  with path 1?) rather than a deviation from spec — which is exactly why `Regression-test` is not
  yet named and `Status` is `Open`.
- A literal `null` also reads as a value rather than an absence to anyone skimming the file, unlike
  an empty key.

## Why nothing caught it

`new_record.rs`'s tests cover what this change would need to notice, but never across the two paths
together: `render_synthetic_yaml`'s tests assert `Date`/`Author` filling and BUG-6's YAML
round-trip/escaping behaviour (a colon-bearing value, a numeric-looking string), and the template
path's tests assert `Date`/`Author` land on their template lines. Nothing asserts what *state* the
remaining required fields end up in, and nothing compares the two paths' output for the same type —
so the divergence had no test to fail. The gap only surfaced by running the command and reading the
file it produced, rather than reading either path's tests, which is the failure mode AGENTS.md's
"Verify before trusting" section describes.

## References

- `rust/crates/urzua-core/src/new_record.rs` -- `render_synthetic_yaml`, whose `_ => Value::Null`
  arm this bug is about, and `template_body`, which splices body sections but not header fields.
- `rust/crates/urzua-core/src/field_state.rs` -- `classify`, and the module doc stating why Blank
  and Placeholder must stay distinct.
- BUG-3 -- the fix that made configured `header_shape` win unconditionally; correct on its own
  terms, and the reason the template's header placeholders stopped being used.
- BUG-5 -- the previous instance of a Blank/Placeholder collapse, in `field_state`'s token list.
- BUG-6 -- `render_synthetic_yaml`'s prior defect; its tests are the ones that cover this function
  today without covering this.
- SPEC-12 -- `urzua new`'s three paths; path 1 leaves enumerated placeholder text, path 2 emits a
  blank key. Whether that difference is intended is this bug's open question.
- ADR-27 -- the decision SPEC-12 details: what `new` fills in versus leaves as placeholder.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-11 | Initial bug record, `Status: Open`. Not yet fixed -- whether path 2 should splice the template's header placeholders, synthesize an enumerated vocabulary from config, emit a truly empty key instead of `null`, or stay as-is by design is undecided. | **structural** |
