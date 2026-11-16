---
Stable-Id: 01M6K4FPRR5MJ8YWAYDGXXGQHB
Status: Accepted
Date: 2026-11-04
Accepted-on: 2026-11-04
Author: beauwilliams
Deciders: beauwilliams
Derives-from: RFC-7
---
# 71 — One gate runs at the editor, the commit, the push and CI, and only CI is relied on as the guarantee

## Context

`RFC-7` section 3 names three places the check can run (the editor, the git hooks and CI) and says none substitutes for another. This repository's gate sits in one of them, and its other layers show how a layer fails.

- `SPEC-1`'s first success criterion names the pre-push hook and CI. Only CI was built (`MILE-99`).
- The local hooks are opt-in (`make hooks-install`); `.git-hooks/pre-commit` exits 0 with a notice when the binary is not built; `urzua doctor` examines the config and workflows but not hooks, so a repository cannot verify its own hook is wired.
- `.git-hooks/pre-push` states the failure it guards: a check that lives only in one place and is wired to no gate is the worst of both postures.
- A guard that runs only where someone remembers to run it did not catch the defect it was written for: the release invariants ran in `make ci` and not on the unattended path (`BUG-96`).
- A check that cannot see history reads clean: a shallow checkout gave an earlier false zero for drift (`MILE-159`), so history depth is a precondition of the result, not a detail.
- `RFC-7` section 4 notes that a check about a pull request's cumulative diff has no useful per-commit form, because it would repeat on every commit of a long branch.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. CI only | One place to maintain | Feedback arrives after the work is packaged (`MILE-99`) |
| B. A different check at each layer | Each layer tuned | Four definitions of the gate that drift apart |
| C. The same command and JSON contract at every layer, differing in scope, with only CI relied on (chosen) | One definition; early feedback locally; a guarantee that does not depend on installation | Local layers can be skipped or absent |
| D. Make the local hooks mandatory | No bypass | Git does not version or force hooks; `--no-verify` exists; not achievable |

## Decision

In the context of a records gate that must hold whether or not a contributor installed anything, facing layers that have failed by being absent or by skipping without saying so, we decided that **one gate runs at the editor, the commit, the push and CI, and only CI is relied on as the guarantee**, to achieve early feedback without making correctness depend on local setup, accepting that a local layer can be missing.

- **One gate.** Every layer runs `urzua check` and reads its JSON. Layers differ in scope: the editor judges one file (`ADR-69`), the commit uses `--base HEAD`, the push and CI run the whole tree, and a pull request in CI also runs `--base <base branch>`. No layer carries rules the others do not.
- **CI is the guarantee.** A local layer is a convenience that brings feedback earlier. A result is not assumed because a hook exists.
- **A layer that cannot run says so.** A missing binary, an unreadable history or a shallow checkout is reported as not-run, in output the contributor or CI sees. It is not a skip that reads as success.
- **Wiring is verifiable.** `urzua doctor` reports whether each local layer is installed and runs `urzua`, and a documented one-step install exists.
- **Diff-level checks run where a diff exists.** A check about a pull request's cumulative change (`MILE-142`) runs in CI, not per commit.
- **Not decided here.** Whether an absent binary fails or warns a hook (`MILE-179`), and what the hook does with a non-blocking finding (`MILE-99`).

## Amendment (2026-11-16): `urzua hooks install` is the one-step install, and an installed hook warns when urzua cannot run

Decided by the builder on 2026-11-16, pending maintainer review; the not-run default was decided by the orchestrator the
same day, also pending maintainer review. The Decision is unchanged; this settles the one-step install the Decision
requires and one of the two questions it left open (`MILE-179`). The maintainer asked for it on 2026-11-16, from a
repository whose every agent session started with the hook warning: "that's the one command fix in mile 220 that you
need to do".

- **The install is a command.** `urzua hooks install` wires the pre-commit and pre-push hooks wherever the repository
  keeps them: `.git/hooks`, a `core.hooksPath` inside the repository, husky's `.husky/`, or the pre-commit framework's
  `.pre-commit-config.yaml` (a `repo: local` stanza, appended only with `--write-config`). `hooks status` reports the
  same wiring `doctor` and the session-start check read, and `hooks uninstall` removes only what the install added. It
  writes unless `--dry-run`, as `harness install` does (`MILE-220`).
- **A hook urzua did not write is extended, never replaced, and only where the block will run.** The install puts a
  block between `# >>> urzua-hooks begin` and `# <<< urzua-hooks end` after the hook's last command, or before it when
  that command is a top-level `exit` or `exec`. It refuses, writing nothing and naming the one line to add and where,
  a hook that could end before the block (an `exit` or `exec` anywhere else, a `trap`, or a last command that is a
  compound such as `fi`), a hook in another language (a Node or Python script), a hook file without the execute bit, a
  symlinked hook file wherever it points, and a hooks directory that resolves outside the working tree and the git
  directory (a linked worktree's and a submodule's git directory are inside) or cannot be written. "Wired" means the
  call can run: a block after an unconditional top-level `exit` or `exec` is reported as unreachable.
- **The hook runs a read-only check named in configuration.** The block calls `urzua hooks run <hook>`, which runs
  `hooks.pre-commit` (default `check --base HEAD`: the changes since HEAD, the scope the Decision gives the commit) or
  `hooks.pre-push` (default `check`) and prints the count and each blocking finding's rule, file and message, and the
  command for the full report, on stderr (`ADR-101`, amendment of 2026-11-15). Only `check` and its alias `audit` may be
  named: another subcommand could write files, reach the network or call the hook again on every commit. `check --base
  HEAD` took about 9 seconds with a release build on this repository, which has about 670 records; a repository that
  finds that too slow can name a cheaper check.
- **When urzua cannot run, the hook warns by default.** When urzua is not found (`URZUA_BIN`, then `PATH`), is a
  release without `hooks run`, or the check cannot run, the hook says loudly that urzua did not run, that the records
  were NOT checked, how to install it and that CI will check them, and lets the commit or push through.
  `hooks.on_not_run: block` stops it instead and names `--no-verify` to skip once. A blocking finding always blocks.
  Warning is the default because, in a versioned layout (husky, an in-repo `core.hooksPath`), one person's install is
  committed and would otherwise stop every contributor who never installed urzua on their next commit after a pull,
  and because CI remains the guarantee; this repository's own versioned hooks warn for the same reason. `hooks run`
  reads the policy from the configuration when it runs; the block carries it only for the cases where it cannot ask
  urzua, and `hooks status` reports a block whose policy differs from the configuration.

## Reversibility

Easy to extend and to relax. The decision adds `doctor` reporting and a rule that layers share one command; dropping a local layer changes nothing for CI. The cost of being wrong is a layer that is installed but loose.

## Consequences

- A contributor who installs nothing is still gated, in CI.
- A repository can ask `doctor` whether its hooks are wired instead of trusting that they are.
- CI must check out enough history for drift; a shallow checkout becomes a stated failure.
- Four layers mean four places to wire; the shared command keeps them one definition.

## References

- `RFC-7` -- section 3 (three integration points) and section 4 (diff-level asymmetry).
- `ADR-69` -- the editor layer.
- `MILE-99`, `MILE-179`, `MILE-180` -- the commit/push hooks and the CI integration this builds.
- `BUG-96`, `MILE-159` -- a guard that ran only where someone ran it, and the history-depth false zero.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-16 | Amendment added: `urzua hooks install` is the one-step install, and an installed hook blocks when urzua cannot run. **Why:** the maintainer asked on 2026-11-16 for the one-command fix of `MILE-220`; decided by the builder, pending maintainer review. | **substantive** |
> | 2026-11-16 | The amendment of 2026-11-16 revised before merge: an installed hook warns by default when urzua cannot run (the orchestrator's decision, pending maintainer review), with the rationale corrected; hooks run only `check`; the block is placed or refused so it always runs, and a block that cannot run is reported unreachable; nothing is written through a symlink or outside the repository; the measured cost of the pre-commit check is stated. **Why:** an independent review of the pull request found a block that could never run reported as wired, writes through symlinks, and a blocking default that would stop contributors who never installed urzua. | **substantive** |
