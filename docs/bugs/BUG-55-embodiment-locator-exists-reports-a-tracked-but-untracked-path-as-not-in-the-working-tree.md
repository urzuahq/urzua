---
Stable-Id: 01M2W3287VC8RN7XCW9DKEA4BB
Status: Fixed
Found-in: 'A cumulative code review of v0.3.0..release'
Regression-test: 'covered by a_locator_naming_a_staged_deletion_is_reported_observed_failing; the message split is prose, asserted by reading'
Blocked-on: —
---
# 55 — `embodiment.locator-exists` reports an untracked path as "not in the working tree"

## What was wrong

The `present` closure returns `false` as soon as a locator is absent from the git-tracked set, before
`try_exists` is consulted. A gitignored file that exists on disk, and a directory locator like
`spec:docs/specs`, both produced a **blocking** error reading *"which is not in the working tree"*.

Both statements are false. The file is in the working tree; it is simply not tracked.

The closure's own comment argues that *"a blocking finding the tool cannot substantiate is worse than
silence"* -- which is exactly what the tracked-set short-circuit produced.

## Fix (shipped)

The message says *"is not a git-tracked file"*, which is true of every case that reaches it: untracked,
absent, and directory alike. The rule's scope is deliberately the tracked set -- discovery is, and a
locator naming something CI will not see is a real finding -- so the behaviour stands and only the
claim about it changes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed and fixed. **Why:** the finding was accurate about the message and not about the behaviour -- scoping to the tracked set is right, saying "not in the working tree" about a file that is in the working tree is not. | **substantive** |
