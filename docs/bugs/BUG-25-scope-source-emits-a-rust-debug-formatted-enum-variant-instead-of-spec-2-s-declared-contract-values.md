---
Stable-Id: 01M28KB048PY0D3H4T5CGQXRPW
Status: Open
Found-in: 'reading `check`''s raw JSON while investigating BUG-24 -- `scope.source` came back as the literal string `"GitTracked"`, which is not one of the four values SPEC-2 declares'
Regression-test: 'not yet written -- `rust/crates/urzua-cli/src/main.rs`, asserting `scope.source` is one of the declared contract values for a sweep run, planted-failing against the current `format!("{:?}", ...)`'
---
# 25 — scope.source emits a Rust Debug-formatted enum variant instead of SPEC-2's declared contract values

## What was wrong

SPEC-2 declares four values for `scope.source`: `tracked-sweep`, `git-diff`, `argv`, `none`, in a
table whose whole purpose is that a consumer does not have to key "did this run" on a missing field.
Its worked example prints `"scope": { "source": "tracked-sweep", "base": null }`.

What `check` actually emits is `"source": "GitTracked"`, produced by
`source: format!("{:?}", discovered.source)` -- the `Debug` rendering of a `urzua-io` enum variant,
serialized straight into the public JSON contract. No declared value is ever emitted by a real run.
`base` is absent from the struct entirely, and an undeclared `record_types` array is present
instead.

The contract is the product here (ADR-23, RFC-3): the reason stdout is always JSON is that an agent
branches on these fields without interpreting anything first. An agent written against SPEC-2 that
branches on `scope.source == "tracked-sweep"` takes the wrong branch on every invocation. Worse,
the value is a `Debug` rendering of an internal type, so it is not merely undeclared but *unstable*:
renaming the `DiscoverySource::GitTracked` variant -- an ordinary internal refactor with no
contract intent -- silently changes public output. `report_could_not_run` separately emits
`"unavailable"`, a fifth undeclared value, from a hand-written string.

## Why nothing caught it

No test asserts the *value* of `scope.source` against SPEC-2's declared set; the field's presence
is what gets exercised. `ScopeInfo.source` is typed `String` rather than a serde-tagged enum, so
there is no compile-time relationship between the contract and what is emitted -- the `format!`
call is invisible to both the type system and the test suite. `DiscoverySource` having a single
variant also means the mismatch never varies from run to run: output that is uniformly wrong reads
as consistent, and consistent output is what a reviewer skims past.

Same class as BUG-4 (`doctor` emitted plain text where the contract said JSON) and BUG-6 (`render`
wrote unescaped YAML): the shape of the output was never asserted against the document that
specifies it.

## References

- `rust/crates/urzua-core/src/report.rs` -- `ScopeInfo`, whose `source: String` is the type-level
  gap; a serde-renamed enum would close it.
- `rust/crates/urzua-cli/src/main.rs` -- the two `format!("{:?}", discovered.source)` sites and the
  `"unavailable"` literal in `report_could_not_run`.
- `rust/crates/urzua-io/src/lib.rs` -- `DiscoverySource`, the internal type currently leaking into
  public output.
- SPEC-2 -- the `scope.source` table and the worked example this output contradicts, including the
  `base` field that is not implemented.
- ADR-23, RFC-3 -- always-JSON stdout and the agent-native contract this undermines.
- BUG-24 -- filed from the same run; its reproduction shows this value.
- BUG-4, BUG-6 -- same class: output shape never asserted against its own spec.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-11 | Initial bug record, `Status: Open`. Not fixed -- the fix touches the published JSON contract (a serde-tagged enum, the missing `base`, the undeclared `record_types`), so it needs a decision and a changeset, not a silent rename. | **structural** |
