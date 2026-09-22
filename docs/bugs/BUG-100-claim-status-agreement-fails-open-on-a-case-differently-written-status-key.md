---
Stable-Id: 01M33QPB6WTPP7DJEJR1DVRBHP
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Regression-test: "a_claim_against_a_case_differently_written_status_key_is_not_judged_observed_failing, rules.rs"
---
# 100 — claim status agreement fails open on a case differently written Status key

## What was wrong

`claim_status_agreement` (`rules.rs:1483-1550`) read a claim's target record's `Status` field with:

```rust
let status = target.header.get("Status").unwrap_or("(no Status field)");
if closed_statuses.iter().any(|s| s == status) {
    continue;
}
```

`Header::get` is exact-match (`ADR-57`). A record whose author wrote `status:` instead of `Status:` —
legal YAML, one keystroke, no schema to catch it — made the lookup miss, and the sentinel string
`"(no Status field)"` never appeared in any configured `closed_statuses` list. So the `continue` never
fired, and **every claim citing that record became a blocking `Error`**, with a message —
`"claims to close X, but X has Status (no Status field)"` — that read as though the record's status
disagreed with the claim, when in fact the field could not be read at all. A previously-clean corpus
would start blocking CI on upgrade with no change to the record's actual content. The same fail-open
shape existed in `pointer_target_status`, which shares this call site's exact pattern.

## Why nothing caught it

`ADR-57` decided that a field *name* comparison is exact. It never specified what a rule's *value read*
should do when that exact-cased lookup misses, and no test exercised a record whose declared field was
present but differently cased — the existing `claim_status_agreement` tests only covered a field that
is genuinely absent or genuinely disagrees, never one that is a spelling variant of a field the record
does carry.

The deeper gap: this call site invented its own answer (fail open, fabricate a sentinel, let the
fabricated value flow into a real comparison) independently of the embodiment rule family choosing the
opposite answer (fail closed, silently) for the same category of miss — `BUG-101`.

## What changed

`ADR-58` decided the shared contract `RFC-40` proposed. `Header::read_declared` now returns
`Present`/`Missing`/`Unreadable` instead of `Option<&str>`, and both `claim_status_agreement` and
`pointer_target_status` skip (`continue`) on anything but `Present` — a miss is unjudged, not
judged-and-wrong. The near-miss itself (a declared field written under a different case) is now
reported once, by the new rule `header.field-case-mismatch`, instead of being fabricated here or
dropped silently in the embodiment rules (`BUG-101`).

## References

- `ADR-57` — the field-name exact-match decision this bug was a gap in the specification of.
- `ADR-58` — decides `RFC-40` and is the fix implemented here.
- `BUG-101` — the same root cause, opposite failure direction, fixed in the same pass.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed, with a reproduction and a diagnosis of the shared root cause with `BUG-101`. **Why:** found by a full-release code review. | **substantive** |
> | 2026-09-22 | `Status: Open` → `Fixed`, by `ADR-58`'s decision on `RFC-40`. **Why:** the shared contract closed both this bug and `BUG-101` in one pass, so both were fixed together rather than staying blocked separately. | **substantive** |
