---
Stable-Id: 01M2WG828ADQJWBQ5T43XSENE0
Status: Fixed
Found-in: "Round 7 of the 0.4.0 review"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::a_symlink_inside_a_claim_path_is_not_descended_into"
---
# 69 — claim_paths recursion follows symlinks so one claim file is read dozens of times under paths that do not exist

## What was wrong

`BUG-63` made `claim_paths` read to any depth. The traversal tests `path.is_dir()`, which follows
symlinks, and keeps no visited set.

With `changes/loop` symlinked to `..` and one real claim file present, the rule reports
`records_examined: 65` and 33 findings:

```text
changes/0001-f.md                            | claims to close BUG-1 ...
changes/loop/changes/0001-f.md               | claims to close BUG-1 ...
changes/loop/changes/loop/changes/0001-f.md  | ... (x33)
```

Every path after the first does not exist. The traversal also leaves the declared prefix entirely --
`..` re-enters the repository root -- so the rule reads files the configuration never pointed it at.
It terminates only because the operating system caps symlink resolution depth.

`symlink_metadata` rather than `is_dir`, or a set of canonicalised visited directories.

## Why nothing caught it

The recursion was added to fix a rule going inert and was tested against the nested layout it was
meant to reach. A traversal's failure modes are cycles and escapes, and neither was considered when
one directory listing became a walk.

## References

- `BUG-63`, whose fix introduced this.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
