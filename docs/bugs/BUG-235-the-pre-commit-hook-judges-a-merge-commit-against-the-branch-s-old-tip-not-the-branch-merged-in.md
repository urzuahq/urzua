---
Stable-Id: 01M88RFDDAHDGN16Y4RMCBK8X1
Status: Fixed
Found-in: the merge commit of pull request 649 (5831780c, parents 97d5f1ee and a0f416fb), where the hook, run with `--base HEAD`, blocked it with 27 errors (20 `diff.pin-changed-unattested`, 5 `section.decided-body-changed`, 2 `status.delivery-agreement`)
Realized-by: code:.git-hooks/pre-commit@sha256:7760e6461a4dc989df5829371b4802b97c5a24c0eb1851969ac841d1acb220f3, test:rust/crates/urzua-cli/tests/hooks_verify_integration.rs#pre_commit_hook_judges_a_merge_against_the_branch_merged_in@sha256:4f70ab8ec1fb07addd7f3a8f20d0dea64cad0782a002a5b71d5de62d706eed79, code:rust/crates/urzua-io/src/lib.rs#changed_paths_since@sha256:ae1d22efe51c8241b618667c2a0c319a15b8f1cada99e1c52095cd810716b44b, code:rust/crates/urzua-io/src/lib.rs#merge_heads@sha256:3bf5282c958d8670ecd1c8254b3f5957f04be198d8b3ce8c4a24fbfb33394db9, test:rust/crates/urzua-cli/tests/hooks_verify_integration.rs#check_against_merge_head_mid_merge_matches_the_committed_merge_against_main@sha256:ad30cb68a6c5090e73c6c47b12cffb48d48e79f679c84f110b44033c1dc46f51
---
# 235 — the pre-commit hook judges a merge commit against the branch's old tip, not the branch merged in

## What was wrong

`.git-hooks/pre-commit` ran `urzua check --base HEAD` for every commit. During a merge `HEAD` is the branch's old tip, so the check compared the merged tree with the tip from before the merge, and everything that came in from main read as a change this commit made, with no attesting rows. Replaying the merge of pull request 649 (`97d5f1ee` with `a0f416fb`, resolved as `5831780c`), `--base HEAD` examined 96 changed paths and reported 27 errors: 20 `diff.pin-changed-unattested` (re-pins that chain from main's hashes), 5 `section.decided-body-changed` (decided-record revision rows from main) and 2 `status.delivery-agreement`. A later staged merge of the same branch was reported at 40 errors; that figure is not reproduced here and is not relied on.

Pointing the hook at `--base MERGE_HEAD` alone was not enough. `changed_paths_since` (`rust/crates/urzua-io/src/lib.rs`) took `git merge-base <base> HEAD`, and mid-merge `HEAD` is still the old tip, so `--base MERGE_HEAD` resolved to the old fork point and charged main's incoming commits to the merge: 165 paths. That removed the 5 decided-body false positives and nothing else. CI, on the committed merge, sees 123 paths. The merge base is now taken against the pending merge commit, whose parents are `HEAD` and every line of `MERGE_HEAD` (`git merge-base <base> HEAD <merge heads...>`), so `--base MERGE_HEAD` resolves to `MERGE_HEAD` and the diff is the one CI judges after the commit: 123 paths, the same findings.

Two gaps remain and match CI's own. A conflict-free `git merge` that commits itself runs no `pre-commit` hook (git has no `pre-merge-commit` hook wired here), so only a merge stopped before its commit (`--no-commit`, or a conflict) is judged locally. A record added and decided on the branch is not judged by `section.decided-body-changed` under `MERGE_HEAD`, which is also CI's gap.

## Why nothing caught it

Nothing ran the hook in the middle of a merge. A builder's throwaway worktree rarely has a release build, so there the hook printed "gate NOT RUN" and let the commit through (`ADR-71`: CI is the guarantee), and the defect showed only in a clone that had built the binary. The hook's tests ran it with no merge in progress.

## References

The maintainer edited the hook (their edit, committed as written) so that, when `MERGE_HEAD` exists, it runs `urzua check --base MERGE_HEAD`; the edit is theirs and is committed as written. `pre_commit_hook_judges_a_merge_against_the_branch_merged_in` runs the hook in a fixture left mid-merge by `git merge --no-commit` and asserts the arguments `check` receives; it was observed failing on the old hook (`--base HEAD`) and passes on the new one, and also asserts a commit outside a merge still uses `HEAD`. `check_against_merge_head_mid_merge_matches_the_committed_merge_against_main` (observed failing before the engine change, paths `ADR-a` and `ADR-b` against `ADR-a`) asserts that `check --base MERGE_HEAD` mid-merge reports the paths and findings `check --base main` reports on the committed merge. The engine change is `run_merge_base` and `merge_heads` in `rust/crates/urzua-io/src/lib.rs`, used by `changed_paths_since` and `merge_base_with_head`. `BUG-236` is the same defect in the hooks `urzua hooks install` writes for other repositories.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-25 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 1 file-level, 1 scoped. Evidence: .git-hooks/pre-commit@sha256:7760e6461a4d, rust/crates/urzua-cli/tests/hooks_verify_integration.rs#pre_commit_hook_judges_a_merge_against_the_branch_merged_in@sha256:4f70ab8ec1fb Reason: Pins the maintainer's edit of the hook, which runs check against MERGE_HEAD during a merge, and the test that runs it mid-merge and reads the base it passes to check. **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-25 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 0 file-level, 3 scoped. Evidence: rust/crates/urzua-io/src/lib.rs#changed_paths_since@sha256:ae1d22efe51c, rust/crates/urzua-io/src/lib.rs#merge_heads@sha256:3bf5282c958d, rust/crates/urzua-cli/tests/hooks_verify_integration.rs#check_against_merge_head_mid_merge_matches_the_committed_merge_against_main@sha256:ad30cb68a6c5 Reason: Pins the engine change that takes the merge base against the pending merge (changed_paths_since and merge_heads) and the test that compares check --base MERGE_HEAD mid-merge with check --base main on the committed merge. **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
