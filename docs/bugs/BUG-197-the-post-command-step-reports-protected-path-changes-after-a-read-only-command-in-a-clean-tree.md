---
Stable-Id: 01M7KF9SXQPJJBZCCKWKHQTYMK
Status: Open
Found-in: "An agent session in this repository on release 0.4.6, 2026-11-17"
---
# 197 — The post-command step reports protected-path changes after a read-only command in a clean tree

## What was wrong

On 2026-11-17 the agent harness's post-command step reported "the command made 0 gated move(s) and 4 protected-path change(s); a decider must make or approve these" after a command that only listed CI runs (`gh run list ...`) and wrote nothing in the repository. Checked immediately afterwards in that checkout:

- `git status --short` printed nothing and `HEAD` was unchanged from before the command.
- No file under any protected path (`.claude`, `.git-hooks`, `.github/workflows`, `.github/actions`, `.github/scripts`, `knope.toml`, `.urzua/config.yaml`, `.urzua/templates`) had a modification time in the previous fifteen minutes.
- The harness cache's protected-hashes file had been rewritten at the moment of the report; its post-command-head file was last written about three hours earlier, at the time of a fast-forward pull of the default branch that the step had reported at the time as five protected-path changes.
- Many other read-only commands had run in the same checkout between the pull and this report without any report.

So the step found the protected files differing from its stored baseline, although git showed no change and no protected file had a recent modification time, and told the person that an approver must act on four changes that git does not show. The four paths were named only in the text shown to the person and were not captured.

Not yet established, and the first things to find out:

- Which four paths were named. They are in the `protected_changes` field of the step's JSON verdict; the adapter passes on only the two texts.
- Whether an untracked or ignored file under a protected path was created, renamed or deleted between the last quiet command and the report. The step's walk hashes every file on disk under a protected entry, not only tracked ones, and reports a file that has gone; a deletion leaves no modification time to find, so the checks above could not see it.
- Whether another session's start in this checkout rewrote the baseline, silently, while such a file existed. Linked worktrees keep their own harness cache, but a session started in this checkout shares it, and session start reseeds without reporting.
- The baseline moves with each report, so the evidence is overwritten: to investigate, copy the cache's protected-hashes file before every run.

Ruled out by reading the code: file mode (the hash is of content only), and hashes recorded from the tree before the pull (every baseline write is taken from disk at that moment). The hashes file is rewritten only when its content differs, so the rewrite seen means the walked state did differ from the stored one.

## Why nothing caught it

The report prints the changed paths to the person and gives the agent only a count, so the agent that could investigate at once cannot see what was reported, and nothing is written to a log. No test runs the step repeatedly over a clean tree with a baseline left by an earlier pull, or with a second session sharing the cache.

## References

- `MILE-311` (the report after a pull) and `MILE-314` (the report when the cache is missing) touch the same baseline.
- The cache as found is already the baseline written by the report, so re-running the step there is quiet; a reproduction needs a transient file under a protected directory, present at one run and gone at the next.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Open`. **Why:** met by the orchestrating agent on 2026-11-17 and reported to the maintainer, who said "file that as a bug to investigate". | **structural** |
