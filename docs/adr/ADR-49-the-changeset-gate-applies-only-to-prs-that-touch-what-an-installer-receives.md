---
Stable-Id: 01M2MRXWKQW6E283350K1SJ0YQ
Status: Accepted
Embodiment: Implemented
Realized-by: code:.github/release-paths, code:.github/workflows/ci.yml
Date: 2026-09-16
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: ADR-29
---
# 49 — The changeset gate applies only to PRs that touch what an installer receives

## Context

`ADR-29`'s gate asks every PR one question: does this change need a changelog entry? Answer with a
`.changeset/*.md` fragment, or with the `no-changeset` label. Neither, and CI fails. The point is that
"I forgot" must not be indistinguishable from "I decided it wasn't needed" — the same silent-failure
class as `BUG-27` and `BUG-28`.

The question is worth asking. It is not worth asking of every PR.

This repository tracks 225 files under `docs/`, 39 under `rust/`, plus `ts/`, `platform/` and
`scripts/`. `[package]` versions only `rust/Cargo.toml`, and the published archive contains the
built binary, `README.md` and `LICENSE`. A PR touching only `docs/` or `.github/` **cannot** affect
what an installer receives — the answer is determined by the repository's structure, not by a
judgement anyone needs to make.

Of the four PRs merged on 2026-09-16, three touched zero files under `rust/`. Each required a human
to apply a label asserting something already known, and each produced a spurious failed run while the
label caught up with the already-started job — the ordering case `BUG-34` documents.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Keep asking every PR | Simple; one rule, no exceptions | Ceremony on the majority of PRs, and a spurious red run each time a label lands after the run starts |
| **Scope the gate to declared release-affecting paths** | The question is asked only where a human judgement exists; the label keeps its meaning instead of becoming reflex | A second declared list to keep true |
| Auto-apply the label from paths | No new concept | A label asserting "I decided" that nobody decided is worse than no label -- it launders a computation as a judgement |
| Drop the gate | No friction | Restores exactly the ambiguity `ADR-29` exists to remove |

## Decision

In the context of a repository where most PRs cannot affect the published artifact, we decided: **the
changeset gate applies only to PRs touching paths declared in `.github/release-paths`.** A PR touching
none of them passes with a log line saying why, and needs no label.

The declared set is `rust/`, `README.md`, `LICENSE`.

**The paths are declared in one file, not inlined in the workflow.** When the release surface changes,
there is one place to change. This is `ADR-44`'s move applied to CI: stop hardcoding a list that
something else already determines.

**Deliberately not the same list as `publish-release.yml`'s packaging step.** That step copies files
*into* the archive; this one names paths whose *change* can affect it. They overlap on `README.md` and
`LICENSE` and differ on `rust/`, which is source that produces the binary rather than a file that
ships. Deriving one from the other would be wrong because they are different questions.

**The third option deserves its rejection recorded.** Auto-applying `no-changeset` from paths would
remove the same friction, and it is worse: the label would then mean "a script computed this" on some
PRs and "a human decided this" on others, with nothing distinguishing them. A label that sometimes
records a judgement and sometimes records a computation records neither.

## Reversibility

Delete `.github/release-paths` and the `scope` step; the gate returns to asking every PR. No data
format changes.

## Consequences

- **The label keeps its meaning.** It is now applied only where a human judgement genuinely exists: a
  `rust/` change that is internal-only, like a refactor or a test. On those PRs it carries
  information rather than being reflex.
- **Most PRs lose a manual step** and the spurious failed run that came with it.
- **A second list must stay true.** Adding a file to the published archive without adding it here
  silently narrows the gate. No drift check is built: the archive's contents have changed once in
  this repo's history, so a checker would be speculative capability (`AGENTS.md`). If it drifts once,
  that is the evidence to build one.
- **`SPEC-20`'s "deliberately unfiltered" warning still holds and is not contradicted.** That warns
  against `paths:` filters on the *workflow trigger*, where a filtered required check can fail to
  report and deadlock a merge. This computes paths *inside* a job that always runs: the check always
  reports, it just decides differently.
- **A PR moving a file into `rust/` is caught**, since the gate reads the changed-path list rather
  than the current tree.

## References

- ADR-29 -- the gate this narrows; its purpose is unchanged.
- ADR-44 -- declaring a list once rather than hardcoding it in the code that consumes it.
- BUG-34 -- the ordering case that made this friction visible three times in one day.
- BUG-35, ADR-48 -- the adjacent question of what triggers a release, decided the same day.
- SPEC-20 -- the CI/CD reference; its trigger-filter warning is distinguished above.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Initial decision. **Why:** three of four PRs merged that day touched nothing an installer receives, and each still required a label asserting it -- a question the repository's own structure already answers. Recorded as a decision rather than a workflow tweak because it narrows what `ADR-29`'s gate means. | **structural** |
