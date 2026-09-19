---
Stable-Id: 01M2WG81X2FW1A4T3833PXVEMP
Status: Fixed
Found-in: "Round 7 of the 0.4.0 review"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::a_claim_paths_entry_naming_a_file_does_not_load"
---
# 68 — The claim_paths guard accepts a file so the rule scans nothing and reports a clean run

## What was wrong

`BUG-56` added a load-time guard so a `claim_paths` entry that does not name a readable directory
fails the run instead of letting the rule report a clean pass over nothing. The guard was later
relaxed from `is_dir()` to `exists()` so that an empty declared directory -- git tracks no empty
directory, and a release consumes the last fragment -- would not abort `check`.

`exists()` is true for a file. A `claim_paths` entry naming one passes the guard,
`read_dir` fails with ENOTDIR, and `read_claim_files`'s `let Ok(entries) = ... else { continue }`
swallows it. The rule reports `records_examined: 0, status: ran`, no findings, exit 0 -- over a
corpus whose `BUG-1` is `Open` while the named file says `Fixes BUG-1.`

The comment directly above the guard still argues for `is_dir()` and explains why it is the right
verdict. The code beneath it does something else.

`p.exists() && p.is_dir()` satisfies both requirements: an empty directory still passes, a file does
not.

## Why nothing caught it

The relaxation was made to fix a real failure -- `check` aborting once a release consumed the last
fragment -- and was verified against that case. Nothing asked what else `exists()` now admits that
`is_dir()` had excluded.

The guard has no test that a non-directory is rejected; `BUG-56`'s tests cover the absent path.

## References

- `BUG-56`, the guard this weakens, and the reason it exists.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
