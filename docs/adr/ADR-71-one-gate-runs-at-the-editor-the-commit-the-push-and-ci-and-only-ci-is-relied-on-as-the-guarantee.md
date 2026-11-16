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

## Amendment (2026-11-16): `urzua hooks install` is the one-step install, and an installed hook blocks when urzua cannot run

Decided by the builder on 2026-11-16, pending maintainer review. The Decision is unchanged; this settles the
one-step install the Decision requires and one of the two questions it left open (`MILE-179`). The maintainer asked
for it on 2026-11-16, from a repository whose every agent session started with the hook warning: "that's the one
command fix in mile 220 that you need to do".

- **The install is a command.** `urzua hooks install` wires the pre-commit and pre-push hooks wherever the repository
  keeps them: `.git/hooks`, a `core.hooksPath` inside the repository, husky's `.husky/`, or the pre-commit framework's
  `.pre-commit-config.yaml` (a `repo: local` stanza, appended only with `--write-config`). `hooks status` reports the
  same wiring `doctor` and the session-start check read, and `hooks uninstall` removes only what the install added. It
  writes unless `--dry-run`, as `harness install` does (`MILE-220`).
- **A hook urzua did not write is extended, never replaced.** The install appends a block between
  `# >>> urzua-hooks begin` and `# <<< urzua-hooks end` after what is there, or before a trailing `exec` line, which
  would otherwise replace the shell before the block ran. It refuses, writing nothing and naming the one line to add,
  a hook in another language (a Node or Python script), a hook file without the execute bit, and a `core.hooksPath`
  outside the repository or not writable.
- **The hook runs what configuration names.** The block calls `urzua hooks run <hook>`, which runs `hooks.pre-commit`
  (default `check --base HEAD`: the staged and unstaged changes since HEAD, the scope the Decision gives the commit)
  or `hooks.pre-push` (default `check`) and prints the count and each blocking finding's rule, file and message, and
  the command for the full report, on stderr (`ADR-101`, amendment of 2026-11-15).
- **An absent binary blocks, with the way out printed.** When urzua is not found (`URZUA_BIN`, then `PATH`) or the
  check cannot run, the hook stops the commit or push and prints how to install urzua and how to skip the hook once
  with `--no-verify`. `hooks.on_not_run: warn` lets it through with the same message. Blocking is the default because
  a person or agent ran the install to be gated, and a hook that passes silently when the tool is missing reads as a
  check that ran; CI remains the guarantee either way, and `--no-verify` keeps a contributor without urzua able to
  commit while knowing why. This repository's own versioned hooks, wired by the build of a fresh clone that cannot
  yet have the binary, keep warning.

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
