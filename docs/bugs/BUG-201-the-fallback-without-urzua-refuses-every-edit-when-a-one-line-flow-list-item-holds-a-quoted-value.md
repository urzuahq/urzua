---
Stable-Id: 01M7PEECW8ET6KT77JBQM4V96S
Status: Fixed
Found-in: "an adopting repository's configuration, upgrading to 0.4.9 (2026-11-18)"
Realized-by: test:rust/crates/urzua-cli/tests/harness_fallback_integration.rs#without_urzua_every_real_configuration_is_read_as_the_binary_reads_it@sha256:bb954de02883a0b0b0d177d75e2896a48ff638fc94b6cde3372c64919eef8bca, test:rust/crates/urzua-cli/tests/harness_fallback_integration.rs#without_urzua_every_shape_is_read_as_the_binary_reads_it_or_more_strictly@sha256:4c0544edd46a75d57927256ad369ea872311c8b749aea36d659d8b6ae07d34a6, code:rust/crates/urzua-cli/src/harness/hook/claude_code/command.rs@sha256:1d5138c5dc5eec1e4c1b0ecd9ea4d09a32313259ff42bc96d1361483f63c0c59
---
# 201 — The fallback without urzua refuses every edit when a one-line flow list item holds a quoted value

## What was wrong

With `urzua` missing or too old, the installed hook command decides from the session's configuration
alone. An adopting repository's configuration with open guardrails was read as not certain, so every
edit and MCP call was refused where the result should have been allow with the could-not-run notice.
The trigger was far from `harness.guardrails`: a list item written as a one-line flow map holding a
single-quoted value, anywhere in the file. Repro, run through the PreToolUse command with
`PATH=/usr/bin:/bin`:

```yaml
x:
  - {name: 'TL;DR', shape: y-statement}
harness:
  guardrails:
    editor-hook: {enabled: true}
```

The engine loads it and lets an unjudged edit through; the fallback refused. The reader split every
line at its first colon to find a key, so `- {name: 'TL;DR', ...` gave the value `'TL;DR', shape:
y-statement}`, and its unterminated-quote check (`if(v~/^\047/&&v!~/^\047[^\047]*\047$/)u=1`)
fired. Its comment and bracket checks also counted characters inside quoted strings. The same reader
read a complex key (`? harness`) as open where the engine refuses.

## Why nothing caught it

The fallback's tests fed shapes built around the guardrails, each checked against the engine. Nothing
ran the fallback over configurations people write, where quoted scalars and flow collections sit in
record type declarations, so a check that misread ordinary YAML elsewhere in the file went unseen.

## References

The reader is rewritten to scan each line as YAML reads it (quotes, comments, flow depth across
lines, block scalars) and to look for keys only where they can change the guardrails.
`harness_fallback_integration::without_urzua_every_real_configuration_is_read_as_the_binary_reads_it`
runs it over every real configuration in this repository and requires agreement with the binary;
`without_urzua_every_shape_is_read_as_the_binary_reads_it_or_more_strictly`
holds the repro and the other ordinary shapes as must-be-open.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-18 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 1 file-level, 2 scoped. Evidence: rust/crates/urzua-cli/tests/harness_fallback_integration.rs#without_urzua_every_real_configuration_is_read_as_the_binary_reads_it@sha256:bb954de02883, rust/crates/urzua-cli/tests/harness_fallback_integration.rs#without_urzua_a_shape_the_engine_refuses_or_might_read_differently_is_refused_and_an_open_one_is_open@sha256:93d063bb5f2f, rust/crates/urzua-cli/src/harness/hook/claude_code/command.rs@sha256:f548f0198bd5 Reason: the regression tests and the rewritten reader, read and run green on this branch **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-18 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `Realized-by computes to Verified` -- stated `Open` disagreed with the computed value `Fixed`. | **structural** |
> | 2026-11-18 | Pin moved to another locator (attested by beauwilliams, identity: gh-verified, actor: agent): `rust/crates/urzua-cli/tests/harness_fallback_integration.rs#without_urzua_a_shape_the_engine_refuses_or_might_read_differently_is_refused_and_an_open_one_is_open`@sha256:93d063bb5f2ff25d975fbeed202b4e18d28b824dad841f253e265381b596a986 -> `rust/crates/urzua-cli/tests/harness_fallback_integration.rs#without_urzua_every_shape_is_read_as_the_binary_reads_it_or_more_strictly`@sha256:4c0544edd46a75d57927256ad369ea872311c8b749aea36d659d8b6ae07d34a6. Reason: the shape test was renamed when it began comparing every shape with the binary's own decision; it holds the same repro and more **Why:** the evidence a claim rests on changed and a person or agent attested why, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-18 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/harness/hook/claude_code/command.rs` f548f0198bd5 -> aac336ada820. Reason: command.rs: the reader is extended in review (keys with blanks before the colon, an indented document, line breaks and bytes the parser sees, tabs on every line, a linear scan with a 64 KiB line bound); the repro still reads as open, so the claim holds **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-18 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/harness/hook/claude_code/command.rs` aac336ada820 -> 1d5138c5dc5e. Reason: command.rs: the reader accepts in_session (refuse or ask) under acceptance, a key the engine's configuration now carries; the repro still reads as open, so the claim holds **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
