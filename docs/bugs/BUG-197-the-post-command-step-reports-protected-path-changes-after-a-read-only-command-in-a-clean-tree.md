---
Stable-Id: 01M7KF9SXQPJJBZCCKWKHQTYMK
Status: Open
Found-in: 0.4.6
Realized-by: null
---
# 197 — The post-command step reports protected-path changes after a read-only command in a clean tree

## What was wrong

On 2026-11-17 the agent harness's post-command step reported "the command made 0 gated move(s) and 4 protected-path change(s); a decider must make or approve these" after a command that only listed CI runs (`gh run list ...`) and wrote nothing in the repository. Checked immediately afterwards in that checkout:

- `git status --short` printed nothing and `HEAD` was unchanged from before the command.
- No file under any protected path (`.claude`, `.git-hooks`, `.github`, `knope.toml`, `.urzua/config.yaml`, `.urzua/templates`) had a modification time in the previous fifteen minutes.
- The harness cache's protected-hashes file had been rewritten at the moment of the report; its post-command-head file was last written about three hours earlier, by a fast-forward pull of the default branch that the step had reported at the time as five protected-path changes.
- Many other read-only commands had run in the same checkout between the pull and this report without any report.

So the step compared the protected paths against a baseline that no longer described the tree, although nothing in the tree had changed, and told the person that an approver must act on four changes that do not exist. The four paths were named only in the text shown to the person and were not captured.

Not yet established, and the first things to find out: which four paths were named; whether the baseline was rewritten by another process (the checkout has about eighty linked worktrees and several agent sessions; linked worktrees keep their own harness cache, but a session started in this checkout would share it); whether a change of file mode or of an ignored file under a protected path counts as a change; and whether the hashes recorded after the earlier pull were those of the tree before it.

## Why nothing caught it

The report prints the changed paths to the person and gives the agent only a count, so the agent that could investigate at once cannot see what was reported, and nothing is written to a log. No test runs the step repeatedly over a clean tree with a baseline left by an earlier pull, or with a second session sharing the cache.

## References

- `MILE-311` (the report after a pull) and `MILE-314` (the report when the cache is missing) touch the same baseline.
- To reproduce the investigation: `urzua harness run post-command` in the affected checkout with the cache as found, and a diff of the cache's protected-hashes file before and after.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Open`. **Why:** met by the orchestrating agent on 2026-11-17 and reported to the maintainer, who said "file that as a bug to investigate". | **structural** |
