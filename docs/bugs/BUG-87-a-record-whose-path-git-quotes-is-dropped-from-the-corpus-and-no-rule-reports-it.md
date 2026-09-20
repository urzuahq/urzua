---
Stable-Id: 01M2YR3GD6J5NXWZ28MZ5EFVP9
Status: Fixed
Found-in: "Round 10 of the 0.4.0 review, reproduced against a scratch repository"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::a_record_whose_name_git_quotes_is_still_examined"
---
# 87 — A record whose path git quotes is dropped from the corpus and no rule reports it

## What was wrong

Discovery reads the raw stdout of `git ls-files` and `git diff --cached`. With `core.quotepath` at
its **default of true**, git C-quotes any path containing a non-ASCII byte -- and a backslash, a
quote, or a newline. The path arrives as the literal string `"docs/adr/ADR-2-caf\303\251.md"`,
surrounded by double quotes, so its `parent()` is `"docs/adr` and never equals the declared
`docs/adr`.

The record is dropped from the corpus. `type.record-outside-declared-dir` -- the rule that exists to
report a record-shaped file no type owns -- misses it too, because its `starts_with` test fails on
the same mangled string.

Reproduced with a two-record corpus where `docs/adr/ADR-2-café.md` is missing a required field:

| | result |
|---|---|
| default config | `ok`, `files_examined: 1`, exit 0 |
| `git config core.quotepath false` | `findings-present`, `files_examined: 2`, exit 1 |

A planted violation is invisible and the run certifies the corpus clean, on nothing more unusual than
an accented character in a filename.

Fixed by passing `-z` to all three git invocations and splitting on NUL, which is verbatim regardless
of `core.quotepath`.

## Why nothing caught it

Every fixture in the suite uses ASCII filenames, so the quoting path has never executed in a test.
The corpus this tool checks daily is also all-ASCII, so the defect could not surface in use either.

The tool is built to be adopted by repositories it has never seen. A non-ASCII record name is
ordinary in most of the world and would have been the adopter's first encounter with it.

## References

- `ADR-55`.
- `BUG-62`, which added the rule that should have caught the orphaned path.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
