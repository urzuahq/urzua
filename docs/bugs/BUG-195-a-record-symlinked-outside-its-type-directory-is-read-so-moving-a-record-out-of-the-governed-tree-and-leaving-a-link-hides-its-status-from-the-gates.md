---
Stable-Id: 01M7CAFWXCEGEFGQXWG7S1ZYXR
Status: Fixed
Found-in: 'an external review of urzua, items 2.1 and 4.1 (2026-11-14)'
Realized-by: test:rust/crates/urzua-cli/tests/record_symlink_escape_integration.rs, code:rust/crates/urzua-cli/src/discovery.rs, code:rust/crates/urzua-core/src/rules/identity.rs
---
# 195 — A record symlinked outside its type directory is read, so moving a record out of the governed tree and leaving a link hides its status from the gates

## What was wrong

A tracked record file that was a symlink was read through the link, whatever it pointed at. Two consequences. A non-decider could land an Accepted record in two pull requests: the first moves a Proposed record out of its governed directory and leaves a symlink in its place (no status changes, so no gate fires), the second sets `Status: Accepted` in the moved copy, a path no record type governs and no gate names, while the link now reads as an Accepted record. And a link to a file outside the repository was read and, by `urzua export`, published; only `export` guarded against that, so `check`, `fix` and the MCP tools followed the link.

## Why nothing caught it

The gates judge a record by the path it is tracked at and assumed the file at that path was the record. The one guard (`export`'s) was local to one command instead of in the shared session load, and no test moved a record out of the governed tree and edited the moved copy.

## References

Fixed by the rule `identity.record-symlink-escapes` (`ADR-89`, amendment of 2026-11-14): on for every configuration and not switchable off, it refuses the record in the shared session load; `check` reports a blocking, unwaivable finding and every other command exits 2 naming the file. The regression tests reproduce the two-pull-request attack end to end (`the_two_pr_symlink_attack_is_blocked_at_both_steps`) and the acceptance case `a-record-symlinked-out-of-its-directory-is-refused` tags this bug.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-14 | Filed and fixed in one pass. **Why:** an external review reproduced a non-decider landing an Accepted record through a symlink; the fix is in the same pull request. | **substantive** |
