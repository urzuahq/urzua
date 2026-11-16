---
Stable-Id: 01M7K4YDM538RP2Z5Y4EXAQN78
Status: Planned
Phase: '1'
Track: integrations
Blocked-on: null
---
# 311 — The post-command report on changes a pull brought in is one plain line

## What

When a command's only gated changes are ones already on the configured base branch, have the post-command report say so in one plain sentence at a level below error, instead of one error paragraph per file.

## Why

On 2026-11-16 a fast-forward `git pull` of this repository's default branch, bringing in one merged and CI-checked pull request, produced five paragraphs each labelled `error`, each listing the whole gate and saying "a decider must make it". The person reading it could not tell what it meant or whether anything was wrong; nothing was. An earlier design that skipped changes arriving by a git move was rejected because local git state (the reflog, the stash) can be written by the same agent the report is about, so the report must stay. What can change is its weight and wording: content that is byte-identical to the base branch's tree is not a pending change by this session, and the report can say that while still being printed.

## Done means

- When every gated path a command changed has, after the command, the content recorded for it in the configured base branch's tree, the post-command report is one sentence naming the count and the branch, at a level below error, shown by a test with a fast-forward pull.
- A gated path whose content differs from the base branch's tree is still reported at `error`, the default of the level that the last item makes configurable, including when it arrived in the same command as paths that match, shown by a test.
- Every post-command report, matching or not, names each gate once, not once per file, and the one-sentence report says that nothing needs doing, shown by a test of its text.
- A change made to look as if it matches by moving a local ref or committing on the local base branch is still printed, as the one sentence; the existing tests of those cases are updated to assert it, and the spec and decision record that state the report's rule are amended.
- The level and the comparison branch come from configuration, and the reference page for the harness states that the comparison uses local refs and so is advisory, with CI as the guarantee.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** The maintainer reported on 2026-11-16, with a screenshot of the post-command report after a pull of the default branch, "we have another potential usability issue ... I don't know what they mean". | **structural** |
