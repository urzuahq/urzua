---
Stable-Id: 01M36MCH8GCXW17X4JCZ3X4C4K
Status: Open
Found-in: "A /code-review v0.3.0...main pass, round 22"
Regression-test: "not yet written -- Status: Open, no fix decided yet"
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

## Why this isn't fixed yet

Fixing it correctly requires deciding the delimiter/escaping policy for the format, not just correcting
an implementation detail:

- Reject a locator path containing a comma at parse time (simplest, but silently narrows what a
  locator can name -- a comma is a legal filename character on most filesystems, however rare in
  practice).
- Add an escaping or quoting convention (matches free-form CSV-like formats, but complicates the
  "flat and simple" format `ADR-18` explicitly chose, and needs its own decision on syntax).
- Change the field's delimiter entirely (a breaking format change, the same class of decision
  `ADR-54` already made once for this file format's schema version).

Per this project's own workflow, a finding that needs a design decision gets filed, not rushed.

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
