---
Stable-Id: 01M7GY81CTD2A94A06SRNN9ENM
Status: Done
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 278 — The post-command hook is silent unless a command changed a governed file

## What

Make the post-command hook report only what the command itself changed, say nothing when it changed nothing governed, and send each message to one audience.

## Why

The maintainer asked on 2026-11-16 why the same multi-line report appears after every command. The hook compares the working tree with a saved baseline, so after a pull it repeats the same protected-path changes on every later command, including read-only ones, and it writes the same text to both the person and the agent. The harness research of 2026-11-16 compared the hook, approval, sandbox and audit features that current coding agents document and reports that the host can tell a hook which files a shell command changed.

## Done means

- A read-only command produces no hook output, shown by a test. — evidence: code:rust/crates/urzua-cli/src/harness/post_command.rs@sha256:f8cb3059634e9ae4887aeeb944c01262d7f35cc94d8c9cdaa1a8575c301e5511, code:rust/crates/urzua-cli/bundle/claude-code/files/.claude/hooks/urzua-claude-code.py@sha256:b39594287b55a57351a1e2396fd2aa77fc6b69ae0f2ae410bbaea3f29cd37d55, test:rust/crates/urzua-cli/tests/harness_post_command_integration.rs#the_adapter_sends_findings_to_the_agent_only_and_a_protected_change_to_both_without_repeating@sha256:47665daa5bf0eaab33d2e251335fb3fe50648b479543cb751f40d4e8fcec321c
- A change brought in by `git pull`, `git checkout` or a merge is not reported as the agent's edit, and is not repeated after later commands. — evidence: code:rust/crates/urzua-cli/src/harness/post_command.rs@sha256:f8cb3059634e9ae4887aeeb944c01262d7f35cc94d8c9cdaa1a8575c301e5511, test:rust/crates/urzua-cli/tests/harness_post_command_integration.rs#a_pull_that_changes_a_protected_path_is_not_the_commands_and_is_not_repeated@sha256:6b508a6ad35670ea521cdd5a5dd8cc6dd663ae98049ad5f03137b66287121d27, test:rust/crates/urzua-cli/tests/harness_post_command_integration.rs#a_stash_pop_restores_changes_that_are_not_the_commands@sha256:9e568e60449a733f974510e46153162c21ed9c04302645f91bb73f4e42d38524
- A gated move or a protected-path change made by the command is reported once: one line to the agent, and a message to the person only for those two cases and for a hook that could not run. — evidence: code:rust/crates/urzua-cli/src/harness/post_command.rs@sha256:f8cb3059634e9ae4887aeeb944c01262d7f35cc94d8c9cdaa1a8575c301e5511, code:rust/crates/urzua-cli/bundle/claude-code/files/.claude/hooks/urzua-claude-code.py@sha256:b39594287b55a57351a1e2396fd2aa77fc6b69ae0f2ae410bbaea3f29cd37d55, test:rust/crates/urzua-cli/tests/harness_post_command_integration.rs#a_command_that_edits_a_protected_path_is_reported_once_to_each_audience@sha256:7036a4cadf18fc267849c25fec3d3d6cd03550630671d50159b86f42dcdf57ef, test:rust/crates/urzua-cli/tests/harness_post_command_integration.rs#the_adapter_relays_a_finding_to_the_agent_alone@sha256:b1bf0df4050960b82504d9da673b296c017acb4c2d2013199a2d9b56a0398dd1
- The hook does not start the engine for a command the host reports as having changed no file, where the host provides that. — evidence: code:rust/crates/urzua-cli/bundle/claude-code/files/.claude/hooks/urzua-claude-code.py@sha256:b39594287b55a57351a1e2396fd2aa77fc6b69ae0f2ae410bbaea3f29cd37d55, test:rust/crates/urzua-cli/tests/harness_post_command_integration.rs#a_host_list_of_changed_files_skips_the_engine_only_when_empty@sha256:4548bda4bae859a18cf3904546c91b50bc70cbba7648b8c1cca5ed037dccd15a
- The edit hook prints nothing for a write it allows outside the repository unless `harness.report_outside_writes` is `true`; a refused write and an unjudged edit inside the repository still print, shown by a test. — evidence: code:rust/crates/urzua-cli/src/harness/run.rs@sha256:55c9b081e9cdc7e4d72b88a1597d490346ec6288a0678dfd293252883f63e980, test:rust/crates/urzua-cli/tests/harness_outside_writes_integration.rs#an_allowed_write_outside_the_repository_is_silent_unless_reported@sha256:4e128f7248069adb904b5365ad9258e0b333f1aeedd39c07adeb722f2e6e6779, test:rust/crates/urzua-cli/tests/harness_outside_writes_integration.rs#a_refused_outside_write_and_an_unjudged_inside_edit_still_speak@sha256:c20b40d0ba10122ec10640557f41d244b943b9dd2e3f56c3da531f01f59194df

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** The maintainer asked about the repeated post-command report on 2026-11-16; the fix follows the harness research of that day. | **structural** |
> | 2026-11-16 | Added a Done-means item: the edit hook is silent for an allowed write outside the repository by default, with a configuration key that restores the message. **Why:** Asked "should the edit hook stay silent for writes outside the repository by default?", the maintainer answered on 2026-11-16 "yes hook should be silent by default and n config". | **structural** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (5 of 5 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
