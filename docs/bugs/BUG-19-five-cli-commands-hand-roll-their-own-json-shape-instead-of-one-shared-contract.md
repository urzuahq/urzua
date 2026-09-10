---
Stable-Id: 01M26JPHSH390CDZV1VQVP45FG
Status: Open
Found-in: 'reviewing where resolve_identity''s gh-vs-by divergence warning should live -- asked whether there''s a single generic output function every command already goes through, and found there isn''t'
Regression-test: 'not yet written -- fix scope (one shared Report/Notice type all five commands adopt, vs. a narrower convention) not yet decided'
---
# 19 — Five CLI commands hand-roll their own JSON shape instead of one shared contract

## What was wrong

ADR-7's own Consequences section states plainly: "every future command (`audit`, `migrate`,
`export`/`import`, `doctor`) must emit this same shape, not invent its own -- a second, incompatible
JSON shape would defeat the point." That never happened. Today there are three different, mutually
incompatible output paths:

- `check` -> `print_report(&CheckReport)`, the rich shape (`status`, `files_examined`,
  `rules_executed`, `scope`, `blocking`, `findings`).
- `fix` -> `print_fix_report(...)`, a second, different ad hoc `serde_json::json!({...})` block.
- `new`, `explain`, `graph`, `doctor` -- each builds its own one-off `serde_json::json!({...})`
  inline in its own `run_*` function, no shared function at all.

Compounding this: `Finding` (the type `check`'s rich report uses for anything worth surfacing) has a
mandatory `file: PathBuf` field, so it can't represent a notice that isn't about a specific file --
exactly the shape of `resolve_identity`'s gh-vs-`--by` divergence warning, which is about a flag
value, not a record. That warning was originally implemented as a bare `eprintln!` from inside
`urzua-io` (a library crate, with no access to any of the CLI's JSON machinery even if a shared shape
existed) -- unstructured stderr prose an agent has no reliable way to parse, in a tool whose own
entire pitch is agent-native structured output. It was reworked to return the warning as data
(`ResolvedIdentity { name, warning }`) and threaded into `new`'s and `fix`'s own existing per-command
JSON as an ad hoc `"warnings"` array -- a real fix for that one call site, but it papers over the
actual gap rather than closing it: the next command that needs to surface a non-blocking notice has
nothing to reuse and will invent a fourth shape.

A related, smaller instance of the same root problem: `rust/crates/urzua-cli/src/main.rs`'s
`run_fix`'s tier-not-implemented guard (`"only tier 1 is implemented so far (ADR-0015 defines tiers 2
and 3, not yet built)"`) and the identity-divergence warning both cited an internal ADR number
directly in user-facing output -- meaningless to anyone running this tool against their own corpus,
where no such ADR exists. The identity-divergence instance was fixed (the ADR reference dropped from
the message text); the tier-not-implemented one was not touched, since it's a pre-existing message on
a code path unrelated to this fix and re-plumbing it means also giving that early-exit guard clause
the same treatment described below.

## Why nothing caught it

`check`'s single, well-exercised output path made it easy to assume the "one shape" decision was
actually enforced everywhere, when it was only ever built where `check` needed it. Nothing greps for
`serde_json::json!(` call sites outside `report.rs` and flags a shape that doesn't reuse
`CheckReport`/`Finding` -- there's no rule or test asserting "every command's stdout parses as one of
N declared shapes." Each new command shipped independently, and each one's own `json!({...})` block
looked locally reasonable in isolation.

## References

- `rust/crates/urzua-core/src/report.rs` -- `CheckReport`, `Finding`, `RuleExecution`: the one shape
  that actually exists.
- `rust/crates/urzua-cli/src/main.rs` -- `print_report`, `print_fix_report`, and the inline
  `serde_json::json!({...})` blocks in `run_new`, `run_explain`, `run_graph`, `run_doctor`.
- ADR-7 -- "every future command ... must emit this same shape, not invent its own," the decision
  this bug reports as unrealized.
- ADR-23 -- "every entry point" emits JSON unconditionally; doesn't specify a shared shape beyond
  `check`/`fix`/`migrate schema --report`.
- `rust/crates/urzua-io/src/lib.rs` -- `resolve_identity`/`ResolvedIdentity`, the specific call site
  that surfaced this.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-10 | Initial bug record, `Status: Open`. Not yet fixed -- whether the right shape is one generic envelope every command adopts, a narrower shared `warnings: Vec<String>` convention, or something else is not yet decided. | **structural** |
