---
Stable-Id: 01M2YR3GD6J5NXWZ28MZ5EFVP9
Status: Fixed
Found-in: "Round 10 of the 0.4.0 review, reproduced against a scratch repository"
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#a_record_whose_name_git_quotes_is_still_examined@sha256:8c414d3d19955d2b2f3d1043a41639f562f42846071b231acbd1055e3aa28b67
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
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#a_record_whose_name_git_quotes_is_still_examined` (lines 2573-2602) f9d65ede5e24 -> 8c414d3d1995. Reason: Read the diff of each pinned span since the old pin and the claim still holds. check_integration.rs#a_record_whose_name_git_quotes_is_still_examined: the output assertions now read the JSON field with json_field instead of substring-matching the printed text, with the same expected values. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
