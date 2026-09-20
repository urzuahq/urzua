---
Stable-Id: 01M2YN6BHEFWJE09P5FTE8YGKT
Status: Fixed
Found-in: "Round 8 of the 0.4.0 review"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::a_claim_paths_root_symlinked_to_a_real_directory_is_usable"
---
# 80 — A claim_paths root that is a symlink to a real directory aborts the run

## What was wrong

`BUG-69`'s root guard rejects any symlink:

```rust
if !std::fs::symlink_metadata(&declared).is_ok_and(|m| m.is_dir())
```

`symlink_metadata` does not follow the link, so `is_dir()` is false for a symlink whatever it points
at. A `claim_paths` root that is a symlink to a real directory *inside* the repository now aborts the
entire run:

```json
{"status": "not-run", "error": "claim.status-agreement: claim_paths entry 'changes' is not a readable directory"}
```

Exit 2, and no other rule runs.

Failing closed is the right direction and the message is wrong: the entry is a readable directory.
The goal was to stop the walk leaving the declared prefix, which is served by resolving the target
and confirming it stays under the repository root -- not by rejecting links outright.

Lower severity than its siblings because it fails loudly. It is recorded because the diagnosis is
wrong, and a wrong diagnosis costs the reader more than a missing one.

## Why nothing caught it

The guard was written against a root symlinked to an *ancestor*, which is the escape it had to stop,
and tested against exactly that. A link to a legitimate directory is the same construct used
correctly and was not tested.

## References

- `BUG-69`, whose fix introduced this.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
