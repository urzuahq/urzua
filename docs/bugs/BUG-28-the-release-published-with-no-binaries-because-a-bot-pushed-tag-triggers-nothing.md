---
Stable-Id: 01M2M7H44YC7BG9X06VNK35K6N
Status: Fixed
Found-in: 'testing the ADR-0045 release flow end to end for the first time -- v0.2.0 published with a correct changelog and zero attached archives, while v0.1.0 has three'
Regression-test: 'the upload job now asserts the release carries three archives and fails the publish if it does not -- the assertion is the test, and it runs on every release rather than once'
Realized-by: code:.github/workflows/publish-release.yml
---
# 28 — The release published with no binaries, because a bot-pushed tag triggers nothing

## What was wrong

`v0.2.0` published successfully: correct tag, correct GitHub Release, a real 14,380-character
changelog compiled from twenty changesets. **Zero attached archives.** `v0.1.0`, released under the
previous flow, carries three (`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`x86_64-unknown-linux-gnu`).

`release.yml` built and uploaded those archives, triggered by `on: push: tags: ["v*"]`. Under
`ADR-0029`'s direct-push flow a human pushed the tag, so the workflow fired. `ADR-0045` replaced that
with a two-step flow where `knope release` creates the tag using `GITHUB_TOKEN` — and **GitHub never
triggers a workflow from a `GITHUB_TOKEN` event**, by design, to prevent recursive runs.

So `release.yml` has not run since 2026-09-05. Nothing reported that. `publish-release.yml`'s own
header comment asserted the opposite:

> the tag push fires the existing tag-triggered release.yml (cross-compiled binaries) unchanged

That sentence was the entire basis for believing binaries still shipped, and it was written from the
mechanism's intent rather than from an observed run.

A second instance of the same root cause surfaced in the same session: the release PR's own `ci` run
sat in `action_required`, because knope pushed the release branch with the same token. It had to be
approved by hand before the PR could merge. `ADR-0045` documents no such step, so the two-step flow
has been carrying an undocumented manual gate since it shipped.

## Why nothing caught it

**Nothing asserted the release was complete.** `knope release` returning success means a release
object exists — not that it can be installed. No step compared the published release against what a
release is supposed to contain, and the one signal that would have shown it (`release.yml`'s run
history, empty since September 5th) is only visible to someone who thinks to look.

This is `BUG-0027`'s shape one layer out, found in the same hour: there, a workflow reported success
while preparing nothing; here, a workflow reported success while publishing something incomplete.
Both passed because the check was "did the step exit zero," never "did the thing that was supposed to
happen, happen."

It also would not have been caught by testing the mechanism in parts. Every individual piece worked:
knope tagged correctly, `release.yml` was valid and had succeeded before, the permissions were right.
The defect lived only in the *coupling* — an assumption about cross-workflow triggering that is
false, stated in a comment, and never exercised until the first real end-to-end run.

## The fix

`publish-release.yml` now owns the whole publish — `verify-ci`, `release`, `build`, `upload` — and
`release.yml` is deleted. One workflow, no cross-workflow trigger to be silently absent:

- **The build matrix and attestation move over unchanged**, and now check out the tag `knope release`
  created rather than a branch head, so a push landing on `main` mid-publish cannot produce binaries
  that disagree with the release they attach to.
- **`verify-ci` checks the release PR's head sha**, not the merge commit. The head has definitively
  finished (it gated the merge); `ci` on the merge commit races this workflow. Same tree either way,
  since the merge is a squash of a PR whose checks passed.
- **The upload job asserts the result**: it reads the release back and fails if fewer than three
  archives are attached. A release that looks published and cannot be installed is the defect this
  record exists for, so it is checked rather than assumed.

`v0.2.0` itself remains binary-less — it is already published, and the fix is forward-looking. Whether
to retro-attach archives to it is a separate call, not made here.

## References

- ADR-0045 — the two-step flow whose tag-creation mechanism this bug is a consequence of; amended to
  record the single-workflow publish and the `GITHUB_TOKEN` trigger constraint.
- ADR-0029 — the superseded direct-push flow, under which the tag-triggered build did fire.
- ADR-0013 — release and distribution; names the cross-compiled archives as the distribution
  mechanism this bug silently removed.
- BUG-0027 — the same "reported success, did nothing" shape one layer earlier in the same pipeline.
- SPEC-0020 — the CI/CD feature area; its workflow inventory no longer lists `release.yml`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Filed and fixed in the same change. **Why:** found by running the ADR-0045 flow end to end for the first time -- every part worked in isolation and the coupling between two of them was false, which is exactly the class a parts-level test cannot reach. The assertion added to the upload job is the durable half: it converts "we believe binaries ship" into something that fails loudly when they do not. | **substantive** |
