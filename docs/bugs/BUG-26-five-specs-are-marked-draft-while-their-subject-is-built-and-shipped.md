---
Stable-Id: 01M2M4GYH2DPTYDC01ZXQAKQBS
Status: Open
Found-in: 'asked why SPEC-5 was still Draft while describing a shipped command; checking every spec''s Status found a clean date split -- the five oldest are all Draft, everything written later is Accepted'
Regression-test: 'not yet written -- blocked on RFC-28, which proposes the rule that would catch this class mechanically rather than by inspection'
---
# 26 — Five specs are marked Draft while their subject is built and shipped

## What was wrong

Every spec's `Status`, as a table:

| Status | Specs | Written |
|---|---|---|
| `Draft` | SPEC-1, 2, 3, 4, 5 | 2026-07-29 / 2026-08-20 |
| `Accepted` | SPEC-6, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20 | 2026-09-07 onward |

That is a clean split by date, not by maturity. The five oldest specs predate the
accept-a-spec-when-its-subject-ships habit, and nobody went back. Concretely:

- **SPEC-2** describes `urzua check` — the most-built command in the tool, with its own integration
  test file — and is at `Version: '0.8'` after eight revisions. Still `Draft`.
- **SPEC-3** describes `.urzua/config.toml`, which every command reads on every invocation.
- **SPEC-1** carries `Embodiment: Not started` while `urzua-cli/src/main.rs` declares
  `//! Implements: SPEC-0001` and the CLI is shipped.

The user-visible cost is a spec that documents commands which do not exist. `SPEC-5` §"Type
selection" presents this as a supported invocation:

```
urzua init --types adr,rfc,spec     # non-interactive
```

Running it returns `error: unexpected argument '--types' found`. `init` accepts only `--dry-run`.
The same section describes built-in profiles for `adr`/`rfc`/`spec`/`prd` and an interactive mode,
none of which exist; §Safety claims a re-run "exits 0" when it exits 2; §Success criteria depends on
unclassified-file reporting that was never built.

A second, mechanical cost: because `SPEC-1` is `Draft` and eleven specs name it as `Parent`,
`pointer_resolution` emits 27 live warnings of the form `Parent: SPEC-1 resolves; target Status =
Draft` on every run. They are correct, they are noise, and they will stay until the statuses are
right.

**SPEC-5 is the one genuinely mixed case.** Its adopt path is built and tested; its greenfield mode,
flags, and profile set are not. So flipping it to `Accepted` wholesale would be as wrong as leaving
it `Draft`. Its aspirational sections need to move somewhere that can hold unbuilt design — an RFC
or a milestone — leaving the spec describing only what exists. The other four look like straight
flips, but each needs its own read before the flip, not a bulk edit.

## Why nothing caught it

No rule relates a record's `Status` to whether its subject is built. The engine has the two halves
and never joins them: `is_terminal_status` knows `Accepted` is the only terminal status for a spec,
and `embodiment.consistency` knows how to compare a stated `Embodiment` against `Realized-by`
evidence — but nothing asks whether a `Draft` record has realization evidence pointing at it.

Nor could it, as the corpus currently stands: SPEC-2 through SPEC-5 carry no `Embodiment` or
`Realized-by` field at all, so there is no record-side claim to contradict their `Draft` status. The
only mechanical evidence those specs are implemented is the `Implements:` comments in the Rust
source, and `check` does not read source files. `RFC-28` is the proposal to close that gap; this bug
is the corpus half and does not depend on it landing.

## References

- RFC-28 — proposes the config-declared status lifecycle and the source-claim rule that would catch
  this class mechanically.
- `docs/specs/SPEC-5-urzua-init.md` — the mixed case; §"Type selection" documents flags that do not
  exist.
- `docs/specs/SPEC-2-urzua-check.md` — version 0.8, subject fully shipped, still Draft.
- `rust/crates/urzua-core/src/rules.rs` — `is_terminal_status`, which already knows `Accepted` is
  terminal for a spec.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Initial bug record, `Status: Open`. Not yet fixed -- four specs look like straight flips but each needs its own read first, and SPEC-5 needs its unbuilt sections relocated before it can be accepted honestly. | **structural** |
