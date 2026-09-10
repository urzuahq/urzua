---
Stable-Id: 01M2405K7X2B0QKCB68M4NTZ85
Status: Open
Found-in: 'noticed live while fixing graph.rs''s own tests for PR #12 (ADR-36''s legacy-filename removal) -- a test had to use an unpadded `RFC-1` reference instead of a hand-typed `RFC-0001` to pass, which only makes sense if `graph()` treats the two as different identifiers'
Regression-test: 'not yet written -- fix scope (normalize inside `graph()` directly, or share `pointer_resolution`''s index-building helper) not yet decided'
---
# 11 — `urzua graph` reports false dangling edges due to missing ID normalization

## What was wrong

`pointer_resolution` (`rules.rs`) builds its reference index keyed by `normalize_id(&id)` and looks
up every reference via `normalize_id(&reference)` -- explicitly so a filename's own padding (or lack
of it) never has to match a hand-typed reference's padding exactly (`BUG-2`'s original fix). `graph`
(`graph.rs`) builds a separate index for the same underlying data, but keys and looks up by the raw,
unnormalized string:

```rust
// pointer_resolution
index.insert(normalize_id(&id), record);
...
match index.get(&normalize_id(&reference)) { ... }

// graph
index.insert(id, record);
...
dangling: !index.contains_key(&to),
```

Concretely: if a record's real filename is `RFC-1-x.md` (unpadded, the corpus-wide convention since
`ADR-36`'s amendment) and some other record writes `Implements: RFC-0001` (hand-typed, padded, or
just an old habit), `pointer_resolution` correctly resolves it -- `urzua check` reports no finding.
`urzua graph` on the exact same corpus reports that same edge as `dangling: true`, because
`"RFC-0001"` never equals the index's `"RFC-1"` key as a raw string. The two commands can disagree
about whether the same fact is true.

## Why nothing caught it

Every test in `graph.rs` uses a reference whose padding already matches its target's filename
exactly (verified by grepping the test fixtures) -- none plant the mismatched-padding case
`pointer_resolution`'s own equivalent test (`a_reference_resolves_regardless_of_zero_padding`)
exists specifically to cover. `graph()`'s own doc comment even says it's "read-only, same data
`pointer.resolution`... already parse[s] -- this is a dump of it, not a new computation," which is
true for what fields it reads but not for how it matches them.

## References

- `rust/crates/urzua-core/src/graph.rs` -- `graph()`'s index-building and lookup.
- `rust/crates/urzua-core/src/rules.rs` -- `pointer_resolution`, `normalize_id`, the equivalent,
  already-correct pattern.
- BUG-2 -- the original defect `normalize_id` was built to fix, for `pointer_resolution` only;
  `graph()` was added later (`ADR-24`) and never picked up the same fix.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-09 | Initial bug record, `Status: Open`. Not yet fixed -- whether to normalize inline in `graph()` or extract a shared index-building helper both `pointer_resolution` and `graph()` call is not yet decided. | **structural** |
