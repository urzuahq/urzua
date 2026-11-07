---
Stable-Id: 01M6K4FM2RJYDTE8JK0M2B7PH5
Status: InProgress
Phase: '0'
Track: governance-process
Implements: RFC-7, ADR-71
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/doctor.rs, test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_reports_a_hook_that_is_not_installed@sha256:e7a6c182ff2a8ddbb8a9621c36c2358cc1e45e30d28e4ff7c02ef122f57e3432, test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_reports_a_hook_that_runs_urzua@sha256:d12e188385bdc0a4327d1c6fc562f43a8f18fce09877d9dcffb8be5c97915cc9, test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_reports_a_hook_that_does_not_run_urzua@sha256:6fe8f45d5fd8d072d25d15a3afe56c14f94fd887b16a59aaad273f6d50352766, code:Makefile, code:.git-hooks/pre-commit, test:rust/crates/urzua-cli/tests/check_integration.rs#pre_commit_hook_stops_a_planted_bad_header_with_the_real_binary@sha256:f03f66432bb514351d60746417d4f56fa1fe0c4359a793cac582bb0d4e41a6d9, test:rust/crates/urzua-cli/tests/check_integration.rs#pre_commit_hook_says_loudly_when_the_binary_is_absent@sha256:db7f73009f144a4b6a3876dcb9983248788cfd252e76f153bbc2e66c5754c9e5
---
# 179 — Git hooks run the records gate for every contributor and doctor verifies they are wired

## What

Git hooks that run the records gate for every contributor, and a `doctor` check that says whether they are wired. `urzua doctor` reports, for the pre-commit and pre-push hooks, whether a hook is installed (in `.git/hooks`, a configured `core.hooksPath`, or a hook manager's configuration) and whether it runs `urzua`, and distinguishes not installed from installed. The hooks stop being opt-in per clone through a documented, one-step install that the first build performs or prompts for.

## Why

A real adopter environment runs its records gate from a commit hook that its package manager installs for every contributor, so the gate does not depend on anyone remembering to enable it. Here `make hooks-install` is opt-in, `.git-hooks/pre-commit` exits 0 with a notice when the binary is not built (a skip that reads as success, the shape `AGENTS.md` forbids), and `urzua doctor` examines the config and workflows but not hooks, so a repository cannot verify its own hook is wired. `.git-hooks/pre-push` already records the lesson: "a check that lives only here, wired to no actual gate, is the worst of both postures". `MILE-99` owns what the pre-push hook does with a non-blocking finding and is not duplicated here. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## The decision this needs

Whether a hook fails or warns when the binary is absent. Failing makes a fresh clone unable to commit until it builds; warning is what exists. `ADR-71` states the principle (a layer that cannot run says so, never reports success); this milestone owns the choice.

## Done means

- `urzua doctor` reports each hook as not installed, installed and running `urzua`, or installed and not running it, with a planted case for each in a test. — evidence: code:rust/crates/urzua-cli/src/commands/doctor.rs, test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_reports_a_hook_that_is_not_installed@sha256:e7a6c182ff2a8ddbb8a9621c36c2358cc1e45e30d28e4ff7c02ef122f57e3432, test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_reports_a_hook_that_runs_urzua@sha256:d12e188385bdc0a4327d1c6fc562f43a8f18fce09877d9dcffb8be5c97915cc9, test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_reports_a_hook_that_does_not_run_urzua@sha256:6fe8f45d5fd8d072d25d15a3afe56c14f94fd887b16a59aaad273f6d50352766
- This repository's hooks install in one documented step and run `urzua check --base HEAD` on commit. — evidence: code:Makefile, code:.git-hooks/pre-commit
- A planted bad header is stopped by the commit hook in a test. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#pre_commit_hook_stops_a_planted_bad_header_with_the_real_binary@sha256:f03f66432bb514351d60746417d4f56fa1fe0c4359a793cac582bb0d4e41a6d9
- The absent-binary behaviour is the decided one and is visible in the commit output. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#pre_commit_hook_says_loudly_when_the_binary_is_absent@sha256:db7f73009f144a4b6a3876dcb9983248788cfd252e76f153bbc2e66c5754c9e5, code:.git-hooks/pre-commit
- `urzua doctor` run in this repository reports its own hooks as wired.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed that environment's commit-time gate and its automatic installation have no counterpart here; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Delivered and shipped: hooks are versioned in `.git-hooks/` and wired by `core.hooksPath`, which the first `make build` sets (`make hooks-install` does only that, and never overrides a path the contributor chose); `urzua doctor` reports each layer as not installed, installed and running `urzua`, installed and not running it, or installed without the execute bit, each planted in a test; a planted bad header is stopped by the commit hook in a test. **Decision:** an absent binary, or a `check` that exits 2, makes the commit hook warn with a `NOT RUN` banner on stderr and exit 0 rather than fail. **Why:** `ADR-71` says only CI is the guarantee and a git hook cannot be forced onto a clone; failing would stop a fresh clone from committing the build that creates the binary, and the banner keeps the skip from reading as success. Blocking findings still stop the commit, and the findings are now shown (the hook used to discard them). | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | `Status` `Done` -> `InProgress`. Status set from its Done-means items (ADR-96, MILE-215): unresolved item 5. **Why:** the item has no evidence clause, and `DoneWithUnresolvedItems` forbids `Done` while an item is unresolved; `fix` never downgrades, so the status is set by hand to the computed value. | **structural** |
