---
Stable-Id: 01M2YN698JZ9AMBY8JNYNYNGZX
Status: Fixed
Found-in: "Round 8 of the 0.4.0 review"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::an_unreadable_tracked_record_stops_the_run_rather_than_shrinking_the_corpus"
---
# 76 — An unreadable tracked record drops out of the corpus and check reports ok

## What was wrong

`load_records` drops a file it cannot read:

```rust
Err(_) => continue,
```

`files_examined` counts records, not discovered files, so a record that vanishes this way leaves no
trace in the report. The corpus is smaller and every rule agrees it is clean.

Reproduced: two tracked ADRs with `header.required-fields: error`, one missing its required `Status`.
Normal run: `findings-present`, `files_examined: 2`, exit 1. After `chmod 000` on the offending
record: `status: ok`, `files_examined: 1`, **exit 0**.

Permissions are the easy reproduction; invalid UTF-8 takes the identical path through
`read_to_string` and is the realistic trigger in CI.

A record the tool cannot read is not a record that passes. This is the same judgement `BUG-75` and
`read_claim_files` already make for claims, not yet made for records.

## Why nothing caught it

`BUG-71`'s fix reasoned about this hole explicitly -- its comment notes that the loaded record set
"also excludes a file the loader could not read" -- and then patched only the one rule that tripped
over it, leaving the loader itself unchanged.

Nothing asserts that `files_examined` accounts for every discovered record-shaped file. The two
numbers have never been compared.

## References

- `BUG-71`, which named this hole while fixing a symptom of it.
- `ADR-55`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
