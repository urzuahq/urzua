---
default: minor
closes: [MILE-89]
---

`urzua new` can now skip a number an open pull request already took. With `new: {check_open_prs: true}` in the configuration, `urzua new` fetches pull-request head refs with plain git (no `gh`, no API) into a private `refs/urzua/pr/` namespace before it numbers a record, then skips every number found there, on any branch, or in another worktree of the repository, untracked files included. A lock in the git directory makes two runs started together take two different numbers. `new.pull_ref_patterns` (default `refs/pull/*/head`), `new.remote`, `new.fetch_timeout_seconds` and `new.lock_timeout_seconds` tune it. Pull requests that were closed without merging keep their numbers reserved, which can leave a gap and never a duplicate.

Offline is never an error: when the remote is missing, unreachable or slow, `urzua new` numbers from the refs it has, exits 0 and adds a warning notice. The JSON report gains an always-present `pull_refs` object (whether it fetched and why not, the remote, the ref and worktree counts, and each skipped number with the refs or worktrees that held it). With the option off, `new` touches no remote.

Adds the opt-in rule `identity.open-pr-collision`. Under `urzua check --base`, it warns when a record the diff adds has the same type and number as a file on a fetched pull-request ref under a different `Stable-Id`. `check` never fetches: with no refs present the rule says how to fetch them and counts zero refs.
