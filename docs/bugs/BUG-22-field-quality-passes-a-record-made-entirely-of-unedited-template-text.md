---
Stable-Id: 01M28H5J42CN3DB4DJGKMQND6B
Status: Open
Found-in: 'found by an adversarial review of this record''s own first draft. The draft claimed `field_state::classify` reads unedited template header text as `Placeholder`; checking that against `PLACEHOLDER_TOKENS` rather than assuming it showed the opposite -- it classifies `Present`, and a record built entirely of unedited template text passes `check` with zero findings. The original investigation started from `urzua new` emitting `Status: null`, which turned out to be deliberate and correctly caught.'
Regression-test: 'not yet written -- needs a planted-violation test asserting that a record whose header is verbatim template text produces `field.quality` findings, observed failing on current code first (it currently produces none). Fix scope undecided: extending PLACEHOLDER_TOKENS reaches only the exact strings this repo''s own templates use, which is the narrow-but-brittle option BUG-5 already took once.'
---
# 22 — field.quality passes a record made entirely of unedited template text

## What was wrong

`field_state::classify` (`rust/crates/urzua-core/src/field_state.rs:30-53`) recognises placeholders
by exact match against a fixed list:

```rust
const PLACEHOLDER_TOKENS: &[&str] = &[
    "name", "name(s)", "yyyy-mm-dd", "tbd", "todo",
    "(project lead)", "(session author)",
];
```

The checked-in templates do not use those forms for most fields. `.urzua/templates/bug.md` uses an
enumerated-choice form and prose instructions:

```
> Status: Open | Fixed | WontFix
> Found-in: how/where this was actually discovered
> Regression-test: path/name of the test proving this is fixed
```

None of those match a token, so `classify` falls through to `FieldState::Present` — a real,
author-supplied value. Verified by planting one: a record whose entire header is verbatim template
text, staged and checked, produces **zero findings** and `blocking: false`.

```
findings on a record of PURE UNEDITED TEMPLATE TEXT: 0
```

That is a false negative, and it is precisely the failure `field_state`'s own module doc says the
module exists to prevent: *"a check that treats 'unedited template text' the same as ... 'empty'
cannot tell an author who forgot a field from one who filled it in correctly."* Here it is worse
than collapsing the two — unedited template text is not classified as either placeholder or blank,
but as a correctly-filled field.

Same class as BUG-5, which extended `PLACEHOLDER_TOKENS` to cover this project's retired
`(project lead)`/`(session author)` convention after a hand-edit reverting `Author` to one of them
classified as `Present`. That fix addressed the two tokens in front of it; the enumerated-choice and
prose-instruction forms sitting in the templates the whole time were never added.

### Reachability, honestly

All six configured types declare `header_shape = "yaml-frontmatter"`
(`.urzua/config.toml`), and `urzua new` takes the synthesized path for exactly those — so
`render_from_template` is currently unreachable for every type in this corpus, and `urzua new`
cannot itself produce such a record today. The gap is reachable by a human copying a template by
hand, and by any adopter whose types use `blockquote`/`bold-list`. The templates carrying this text
are checked in and are what a contributor reads.

### What this record originally claimed, and why that was wrong

The first draft indicted the opposite path: `render_synthetic_yaml`
(`new_record.rs:145-152`) emits `Value::Null` for every required field except `Date`/`Author`, and
the draft called that a defect that collapsed Placeholder into Blank. On checking:

- The null is **deliberate and asserted**. `new_record.rs:271` is a test named
  `render_synthetic_yaml_lists_required_fields_as_blank`, asserting `Status: null` with a comment
  explaining that a real YAML serializer emits the null scalar rather than a bare trailing colon.
  SPEC-12's path 2 describes it as "every configured field emitted as a blank frontmatter key."
- The extraction is correct: `header.rs:306` maps `Value::Null` to the empty string, so
  `classify` returns `Blank`, and `field.quality` reports three blocking errors on a freshly
  generated record. Working as designed.

So the path this record accused behaves correctly, and the path the draft held up as the good
example is the one that silently passes. Kept here rather than deleted because the inversion is the
useful part: the draft reasoned from what the code *should* do instead of running it, which is the
failure mode AGENTS.md's "Verify before trusting" section names.

## Why nothing caught it

`field_quality`'s own test for the passing case (`rules.rs:2181`,
`field_quality_passes_a_real_value`) supplies a single ordinary value, so it asserts that a real
value passes without ever asking whether a *fake* value also passes. No test feeds template text
through `classify` at all, and no test reads `.urzua/templates/*.md` to check that what the
templates actually contain is what the placeholder list actually recognises — the two have drifted
independently since BUG-5 last touched either.

## References

- `rust/crates/urzua-core/src/field_state.rs` -- `PLACEHOLDER_TOKENS` and `classify`, the exact-match
  list this bug is about, and the module doc stating the property being violated.
- `.urzua/templates/bug.md`, `adr.md`, `rfc.md` -- the checked-in text no token matches.
- BUG-5 -- the same defect in the same list, fixed for two tokens; this is the unfixed remainder.
- `rust/crates/urzua-core/src/new_record.rs` -- `render_synthetic_yaml` and
  `render_synthetic_yaml_lists_required_fields_as_blank`, the deliberate `null` behaviour this
  record's first draft wrongly indicted.
- `rust/crates/urzua-core/src/header.rs` -- `Value::Null` to empty string, why the synthesized path
  classifies Blank correctly.
- SPEC-12 / ADR-27 -- `urzua new`'s paths and what it fills versus leaves as placeholder.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-11 | Initial bug record, `Status: Open`, filed against `render_synthetic_yaml`'s `null` emission. | **structural** |
> | 2026-09-11 | Rewritten before merge. The original finding was false and inverted: `classify` reads unedited template text as `Present`, not `Placeholder`, and the `null` emission it indicted is deliberate, asserted by a named test, and correctly caught by `field.quality`. Re-aimed at the real defect the same investigation exposed -- a record of pure template text passing with zero findings. The original claim is preserved above rather than deleted, since how it was wrong is the instructive part. | **substantive** |
