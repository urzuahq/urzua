---
Stable-Id: 01M36MCH8GCXW17X4JCZ3X4C4K
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 22"
Regression-test: 'rust/crates/urzua-core/src/rules/embodiment.rs::a_comma_inside_a_locator_path_is_part_of_the_path_observed_failing, rust/crates/urzua-core/src/rules/embodiment.rs::a_comma_followed_by_a_category_prefix_separates_entries, rust/crates/urzua-core/src/rules/embodiment.rs::a_comma_followed_by_an_unknown_prefix_stays_in_the_path'
---
# 129 — parse_realized_by truncates a locator path at a literal comma before checking for the category delimiter

## What was wrong

`parse_realized_by` (`rules.rs`) splits a `Realized-by` field's value on `,` first, then splits each
resulting segment on `:` to find its category (`spec`/`code`/`test`) and locator path:

```rust
for segment in value.split(',') {
    let segment = segment.trim();
    let Some((prefix, locator)) = segment.split_once(':') else {
        continue;
    };
    ...
}
```

A locator path containing a literal comma is silently truncated. For
`Realized-by: code:src/weird,name.rs`, `split(',')` yields `["code:src/weird", "name.rs"]`. The first
segment parses to category `code`, locator `src/weird` (missing `,name.rs`). The second segment,
`"name.rs"`, has no `:`, so it hits the `continue` and is silently dropped -- no error, no finding,
just a truncated path that either fails a "not in the working tree" check for the wrong reason, or
happens to exist and gets validated as the wrong file with no sign that truncation occurred.

## Why nothing caught it

`ADR-18`/`ADR-32` deliberately kept the `Realized-by` format "flat and simple" -- a comma-separated
list of `category:path` pairs -- but neither decision addresses what happens when a locator path
itself contains the list's own delimiter. Every real `Realized-by` value in this repository's own
corpus uses paths without commas, so the ambiguity has never been exercised here, and no test used a
comma-bearing path.

## Decision (2026-11-05)

The maintainer decided the policy on 2026-11-05: a comma separates entries only when it is followed by a
category prefix (`spec:`, `code:`, `test:`, the declared evidence categories). Any other comma is part of
the path. No escaping or quoting is added and the format stays flat. The options the record weighed:

- Reject a locator path containing a comma at parse time (simplest, but silently narrows what a
  locator can name -- a comma is a legal filename character on most filesystems, however rare in
  practice).
- Add an escaping or quoting convention (matches free-form CSV-like formats, but complicates the
  "flat and simple" format `ADR-18` explicitly chose, and needs its own decision on syntax).
- Change the field's delimiter entirely (a breaking format change, the same class of decision
  `ADR-54` already made once for this file format's schema version).

Fixed in `locator.rs`: one splitter, `realized_by_entries`, now serves `parse_realized_by` and
`realized_by_locators`, the two readers of the field. One shape stays ambiguous by construction: a path that itself contains
`,<category>:` (for example `src/a,test:b.rs`) reads as two entries. A comma followed by text that is not a
category prefix now stays in the path, so a mistyped category (`code:a.rs, docs:b.md`) yields a locator
naming nothing, which `embodiment.locator-exists` reports instead of the entry vanishing.

## References

- `ADR-18` -- the decision that kept `Realized-by`'s format "flat and simple," which this bug's
  ambiguity is a gap in, not a violation of.
- `ADR-32` -- reaffirms the same flat format for drift detection.
- `BUG-85` -- the prior "an edge case in this exact field's parsing was never exercised" shape, for
  symlink handling in `claim_paths` rather than `parse_realized_by`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** found by a full-release code review; verified by tracing the exact split logic against the failure scenario. Left `Status: Open` -- the fix needs a delimiter/escaping decision this record deliberately doesn't make on its own. | **substantive** |
> | 2026-11-04 | Triage: still reproduces on main; `Status` stays `Open`. `parse_realized_by` (`rules/embodiment.rs`) still splits on `,` before `:`, so `Realized-by: code:src/weird,name.rs` yields the locator `src/weird` and silently drops `name.rs`. **Why:** the escaping or rejection policy is the open decision the record names. | **substantive** |
> | 2026-11-05 | Fixed. **Why:** maintainer decision 2026-11-05: a comma separates `Realized-by` entries only when a category prefix follows it; otherwise it is part of the path. `SPEC-2` states the rule. | **substantive** |
