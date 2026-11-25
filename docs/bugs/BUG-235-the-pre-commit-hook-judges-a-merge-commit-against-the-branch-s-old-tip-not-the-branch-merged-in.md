---
Stable-Id: 01M88RFDDAHDGN16Y4RMCBK8X1
Status: Fixed
Found-in: the merge commit of pull request 649, which the hook blocked with 40 errors while `urzua check --base origin/main` was clean
Realized-by: code:.git-hooks/pre-commit@sha256:7760e6461a4dc989df5829371b4802b97c5a24c0eb1851969ac841d1acb220f3, test:rust/crates/urzua-cli/tests/hooks_verify_integration.rs#pre_commit_hook_judges_a_merge_against_the_branch_merged_in@sha256:4f70ab8ec1fb07addd7f3a8f20d0dea64cad0782a002a5b71d5de62d706eed79
---
# 235 — the pre-commit hook judges a merge commit against the branch's old tip, not the branch merged in

## What was wrong

`.git-hooks/pre-commit` ran `urzua check --base HEAD` for every commit. During a merge `HEAD` is the branch's old tip, so the check compared the merged tree with the tip from before the merge, and everything that came in from main read as a change this commit made, with no attesting rows. Re-pins that chain from main's hashes gave `diff.pin-changed-unattested`, and decided-record revision rows from main gave `section.decided-body-changed`. The hook blocked the merge commit of pull request 649 with 40 such errors, while `urzua check --base origin/main`, which is what CI runs, was clean. `MERGE_HEAD` is the commit being merged in, so a merge judged against it is judged the way CI judges a pull request against its base.

## Why nothing caught it

Nothing ran the hook in the middle of a merge. A builder's throwaway worktree rarely has a release build, so there the hook printed "gate NOT RUN" and let the commit through (`ADR-71`: CI is the guarantee), and the defect showed only in a clone that had built the binary. The hook's tests ran it with no merge in progress.

## References

The maintainer edited the hook so that, when `MERGE_HEAD` exists, it runs `urzua check --base MERGE_HEAD`; the edit is theirs and is committed as written. `pre_commit_hook_judges_a_merge_against_the_branch_merged_in` runs the hook in a fixture left mid-merge by `git merge --no-commit` and asserts the arguments `check` receives; it was observed failing on the old hook (`--base HEAD`) and passes on the new one, and also asserts a commit outside a merge still uses `HEAD`. `BUG-236` is the same defect in the hooks `urzua hooks install` writes for other repositories.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-25 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 1 file-level, 1 scoped. Evidence: .git-hooks/pre-commit@sha256:7760e6461a4d, rust/crates/urzua-cli/tests/hooks_verify_integration.rs#pre_commit_hook_judges_a_merge_against_the_branch_merged_in@sha256:4f70ab8ec1fb Reason: Pins the maintainer's edit of the hook, which runs check against MERGE_HEAD during a merge, and the test that runs it mid-merge and reads the base it passes to check. **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
