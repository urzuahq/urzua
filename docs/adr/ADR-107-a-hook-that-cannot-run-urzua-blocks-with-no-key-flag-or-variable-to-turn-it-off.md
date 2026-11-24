---
Stable-Id: 01M85PSJ029AHNXTWXMF6YXP19
Status: Proposed
Date: 2026-11-24
Author: beauwilliams
Deciders: beauwilliams
Embodiment:
Supersedes / Superseded-by:
---
# 107 — A hook that cannot run urzua blocks, with no key, flag or variable to turn it off

## Context

The published agent CLI contract's RFC-4 ("Structured in and out", section 5.1) states one rule for every tool it covers: a hook that cannot get a verdict from its tool blocks. It applies when the tool's binary is missing or cannot start, the tool died without printing a report, or the tool answered with a `not-run` report. It covers every hook the host consults before it acts and whose answer can stop the action (a git `pre-commit` or `pre-push` hook, an agent harness hook that runs before an edit or a command). A git hook that cannot run exits 2 (a hook that ran and found something blocking exits 1), and a harness hook gives the answer in its host's format that refuses the action. "Nothing turns this off: no configuration key, no flag and no environment variable." The host's own way round (`git commit --no-verify`) is the host's and is untouched. Passing the contract's runner in CI, and printing contract version 1.1, require this (`MILE-359`, and the milestones listed under Consequences).

The maintainer accepted that RFC and confirmed the edge of the rule, in the words of the contract's RFC-4 revision row of 2026-11-24: "I accept RFC, and yes allow option can be outside rule" (the contract's RFC-4, revision row of 2026-11-24, which records the maintainer's answers to "Do you accept RFC-4 as it now reads?" and "Does urzua's `on_unjudged: allow` option sit outside the hook rule?"). RFC-4 is `Accepted` (`Accepted-on: 2026-11-24`) and its section 5.1 says the maintainer confirmed that the allow option is outside this rule: `on_unjudged: allow` covers the tool running and having no opinion, not the tool being unable to run.

Today urzua decides the opposite for its git hooks, and in one case for its editor hook:

- `MILE-179` (Done; its text stays) decided on 2026-11-04 that an absent binary, or a `check` that exits 2, makes the commit hook warn with a `NOT RUN` banner and exit 0, because `ADR-71` says only CI is the guarantee and a fresh clone must be able to commit the build that creates the binary. `.git-hooks/pre-commit` (`not_run`, exit 0) and the test `pre_commit_hook_says_loudly_when_the_binary_is_absent` assert it; `.git-hooks/pre-push` fails under `set -e` only because `make rust-build` builds the binary first.
- The hooks `urzua hooks install` writes (`rust/crates/urzua-cli/src/commands/hooks.rs`: `shell_line`, `shell_block`, `framework_entry`, `not_run_hook`) carry a policy, `hooks.on_not_run` (`SPEC-3`), `warn` by default and `block` as the opt-in, passed to `urzua hooks run` in `URZUA_HOOKS_ON_NOT_RUN`. Both are a configuration key and an environment variable that turn the rule off, and `block` exits 1 (shell block) and 3 (`hooks run`, `NOT_RUN_BLOCK`), not 2. They also print banners and a closing line on stderr, and the hook `exit 0` branches print prose.
- The editor hook, when the binary cannot run, decides from the session alone (`docs/reference/agent-harness.md`, "When the binary cannot run at all"; `ADR-89`, amendments of 2026-11-17; `SPEC-19`). With a configuration it reads with certainty as open it lets the call through with a notice; with none found it says nothing; with any other it refuses. The first of these three is a hook that could not run its tool letting the action happen.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Keep warn-and-exit-0 as `MILE-179` decided | Nothing to build; a fresh clone commits before it has built | urzua cannot print contract version 1.1 or pass the contract's runner; the contract's rule is one for every tool |
| B. Block, but keep `hooks.on_not_run` and `URZUA_HOOKS_ON_NOT_RUN` as an opt-out | Keeps today's escape for an adopter without the binary | A key and a variable that turn the rule off are what the rule forbids |
| C. Block with no key, flag or variable, for the git hooks and the editor hook's fallback (chosen) | Conforms to the contract; a hook that read "NOT RUN" as success can no longer be mistaken for a check; matches `ADR-71`'s own principle that a layer that cannot run says so | A fresh clone cannot commit until urzua is built or installed; `--no-verify` is the only way past |

## Decision

In the context of the published agent CLI contract's RFC-4 section 5.1 and the maintainer's acceptance of it, facing git hooks and an editor-hook fallback that let an action through when urzua could not run, we decided that every hook urzua installs or answers that is consulted before an action blocks when urzua cannot get a verdict, with no configuration key, flag or environment variable that turns it off, to conform to the contract and to stop a skipped check reading as success, accepting that a fresh clone cannot commit until urzua is built or installed.

Specifically:

1. **Git hooks.** `pre-commit` and `pre-push` exit 2 when urzua is not found, cannot start, dies without a report, or answers `not-run`; exit 1 for blocking findings; exit 0 otherwise. When the binary is missing the hook prints the contract's `not-run` report as a literal (`status` `not-run`, an `error`, `error_kind` `environment`, `next_action` null, `next_action_reason` null, `next_action_argv` null, `next_action_inputs` `[]`) and nothing else: no banner, no closing line. When the binary runs, the hook prints urzua's own report and nothing else. This covers this repository's two hooks and the hooks and stanza that `urzua hooks install` writes. A hook that runs after the fact cannot block and prints the report only; urzua installs none.
2. **`hooks.on_not_run` is removed**, with `URZUA_HOOKS_ON_NOT_RUN` and the `hooks run` exit code 3. No replacement key is added. What happens to an existing configuration that still names the key is for `MILE-368` to settle with the engine's normal unknown-key behaviour and to state in `SPEC-3`; this decision adds no tolerance for it (`AGENTS.md`, "Don't build speculative capability").
3. **The editor hook's fallback** (the binary cannot run) refuses every edit and call in a governed session. The case that lets an edit through with a notice when the configuration reads with certainty as open is removed. The text reader's certainty rules stay, because they still pick the refusal's wording, but no outcome of them is "allow with a notice".
4. **`on_unjudged: allow` is unchanged**, by the maintainer's clarification above: it governs the binary running and having no opinion, and is not a switch for the binary being unable to run. A binary that runs and whose engine panics, does not answer before the hook's deadline or cannot be read already ends in a refusal (`agent-harness.md`), and stays one.
5. **`--no-verify` stays** as the host's own way round. This decision says nothing about it.

Left to the maintainer inside this decision (the contract does not settle them, and this record proposes the first reading in each case):

- *No configuration found.* `ADR-89`'s amendment of 2026-11-17 (second review, pending maintainer review) decided that a repository with no `.urzua/config.yaml` is not governed and the editor hook allows silently, with or without the binary. The proposal keeps it: the contract's rule is about a hook whose tool could not look, and with no configuration there is nothing for urzua to judge; blocking every edit on a machine with a user-level install and no binary would be a different, much larger rule. If the maintainer reads section 5.1 as covering it, the fallback refuses there too.
- *The post-command check's "NOT judged" statements* (`SPEC-19`, lines 127 to 139: a `git` that is absent, a configuration that does not load, a `check` that cannot run). That check is a PostToolUse hook that runs after the command and never blocks, and `on_unjudged` plays no part in it. Section 5.1 says a hook that runs after the fact cannot block and prints the `not-run` report in the host's format, and that is all it can do. The question is therefore whether the check's "NOT judged" prose becomes that `not-run` report (the proposal: yes, `MILE-368` carries the item), not whether `on_unjudged` covers it. `on_unjudged: allow` itself is unaffected: it applies to the editor hook and the gates when the binary ran and gave no opinion.

## Reversibility

Moderate, and cheap in code: the new behaviour is a smaller template and a removed key. The cost is the contributor experience of a fresh clone, which cannot commit until `make build` or an install has run (this repository's own `make build` sets `core.hooksPath` and builds the binary, and the first commit after a clone is usually that build's). Walking it back means a new decision that reintroduces a switch, and urzua could then not print contract version 1.1.

## Consequences

- **Superseded in part, once this is Accepted** (the maintainer's act; nothing here sets it): `MILE-179`'s decision of 2026-11-04 on the absent binary (the milestone is Done and its text stays; a revision row points here), and the `on_not_run` part of `ADR-71`'s amendment of 2026-11-16 and of `SPEC-3` (the `hooks` key). `ADR-71`'s principle (only CI is relied on as the guarantee) is not reversed: a blocked commit is not a check that ran.
- **`ADR-89` needs an amendment once this is Accepted**, not before: its amendments of 2026-11-17 (the fallback "lets the call through with a notice" when the configuration reads as open) change as item 3 says, and `docs/reference/agent-harness.md` (lines 60 to 106), `SPEC-19` (lines 138 and 226 to 229, and their revision rows) and `ADR-89`'s own prose follow.
- **Documents that state the old behaviour** and change with the build, each with a revision row where it is a spec: `docs/reference/ci.md` ("Local hooks", lines 181 to 205), `docs/reference/use-cases.md` (line 596), `docs/reference/adopt.md` (line 218), `docs/reference/commands.md` (line 591), `SPEC-15` (line 147, the `hooks run` paragraph) and `SPEC-3` (the `hooks` key).
- **Tests that change**: `pre_commit_hook_says_loudly_when_the_binary_is_absent` (`rust/crates/urzua-cli/tests/check_integration.rs`) asserts exit 0 and "NOT RUN" and is flipped to assert exit 2 and the `not-run` literal; `pre_commit_hook_reports_a_check_that_could_not_run` and the `hooks run` tests that pin exit 3 or the `warn` policy follow.
- **Protected-file edits only the maintainer makes** (`.git-hooks` and `.claude` are protected paths; nothing is hand-edited by an agent):
  - `.git-hooks/pre-commit`: replace `not_run()` so that it prints the literal `{"status":"not-run","error":"urzua is not built at <path>; run make build","error_kind":"environment","next_action":null,"next_action_reason":null,"next_action_argv":null,"next_action_inputs":[]}` on stderr and does `exit 2`; delete the banner, the `echo "pre-commit: urzua check --base HEAD"` line and the closing line; replace the `*)` branch's `not_run` call so that it prints urzua's output and `exit 2` when that output is one JSON `not-run` report, and prints the same literal itself (with `error` saying `urzua check exited <code> without a report`, `error_kind` `environment`) when urzua died without printing one or printed something else, since there is then nothing of urzua's to relay; the shell hook `urzua hooks install` writes does the same; rewrite the header comment, which states the warn decision.
  - `.git-hooks/pre-push`: add the same missing-binary branch before the `urzua check` line (today a missing binary is covered only by `make rust-build` building it), and make the final `check` exit 2 on exit status 2; drop the `echo "pre-push: ..."` progress lines or keep them off stdout and stderr when the hook blocks.
  - `.claude/settings.json`: the hook commands it carries are written by `urzua harness update` from the code (`urzua-cli`), so after the build the maintainer runs `urzua harness update` and commits the regenerated entries and the lock.
  - `.urzua/config.yaml` needs no edit for this decision: it carries no `hooks.on_not_run`.
  - The hook text `urzua hooks install` writes lives in `rust/crates/urzua-cli/src/commands/hooks.rs` and is code, not protected; this repository's own hooks are `managed: declared` and are not rewritten by it.
- **Contract items outside this decision** are tracked as milestones: `MILE-359` (describe), `MILE-365`, `MILE-366`, `MILE-367`, `MILE-368`, `MILE-369` and `MILE-370`.

## References

- The published agent CLI contract's RFC-4, section 5.1 (and sections 5, 9 and 12), vendored contract at `rust/crates/urzua-cli/tests/fixtures/agent-cli-contract/VERSION`.
- `MILE-179`, `ADR-71`, `ADR-89`, `SPEC-3`, `SPEC-15`, `SPEC-19`, `MILE-220`, `MILE-359`.
- `docs/reference/ci.md`, `docs/reference/use-cases.md`, `docs/reference/adopt.md`, `docs/reference/commands.md`, `docs/reference/agent-harness.md`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Filed as Proposed. It records the maintainer's acceptance of the contract's RFC-4 and the `on_unjudged` clarification, quoted from the contract's RFC-4 revision row of 2026-11-24, and decides nothing until a person accepts it. | **substantive** |
> | 2026-11-24 | Replaced the relayed acceptance with the maintainer's words from the contract's RFC-4 revision row; reframed the post-command question (a hook that runs after the fact prints the `not-run` report and cannot block; `on_unjudged` plays no part); marked the `ADR-89` amendments it relies on as pending maintainer review. Still Proposed. | **substantive** |
> | 2026-11-24 | The `*)` branch of `.git-hooks/pre-commit` prints the `not-run` literal itself when urzua left no report. Still Proposed. | **structural** |
