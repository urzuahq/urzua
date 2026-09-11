---
Stable-Id: 01M28KB0425HYMCHHAWZTEKDSN
Status: Open
Found-in: 'hit live filing RFC-27: ran `urzua check docs/rfc/RFC-27-....md` on the freshly created record and got `"status": "not-run", "files_examined": 0` with zero findings. The same command after `git add` on the same unchanged file returned `"status": "ok", "files_examined": 1`.'
Regression-test: 'not yet written -- `rust/crates/urzua-cli/src/main.rs`, a test passing an on-disk-but-untracked path as an explicit argv path and asserting it is examined (or that a finding says why it was not), planted-failing before the fix'
---
# 24 — check's explicit argv paths filter the tracked set instead of overriding discovery, so an untracked record examines silently as zero files

## What was wrong

SPEC-2's Discovery section states the contract in one line: **"Explicit paths on argv override
discovery and are used as given."** It also declares `argv` as one of four `scope.source` values,
specifically for that case. Neither is implemented. `check` calls
`urzua_io::discover_tracked_files` unconditionally and then passes argv paths to
`scope_to_requested_paths(&repo_root, &discovered.paths, &paths)`, which *intersects* the requested
paths with the tracked set. An explicitly named path that git does not track survives no step of
that intersection.

The observable result is a silent zero-examination pass, on a path the caller named explicitly:

```
$ urzua check docs/rfc/RFC-27-....md      # file exists, not yet git-added
{ "status": "not-run", "files_examined": 0, "findings": [],
  "scope": { "source": "GitTracked", "record_types": [...] } }

$ git add docs/rfc/RFC-27-....md          # same bytes, now tracked
$ urzua check docs/rfc/RFC-27-....md
{ "status": "ok", "files_examined": 1, ... }
```

This is the "no silent no-op" rule failing at the point it matters most. The obvious workflow --
`urzua new <type> "..."` then `urzua check <the path it just printed>` -- reports nothing wrong
about a record nothing examined, and `new` does not stage what it writes. An agent following
`AGENTS.md`'s own instruction to verify a record with the tool gets a clean-looking result that
proves nothing. `status: "not-run"` is technically honest and practically invisible next to
`files_examined: 0` and an empty `findings` array.

Distinct from BUG-13, which is about the *message wording* when a pointer resolves to an untracked
file during a normal sweep. This one is about an explicitly requested path being dropped entirely,
against a spec line that says it must not be, and it is not a request to change ADR-6's
git-tracked-only *default* sweep -- that default is correct and stays.

## Why nothing caught it

Discovery's own tests assert the opposite-facing guarantee and assert it well:
`rust/crates/urzua-io/src/lib.rs`'s test plants an untracked scratch file and asserts it is *never*
discovered. That is right for the default sweep, and it is the entirety of the untracked-file
coverage. No test passes an untracked path as an explicit argv argument, because
`discover_tracked_files` is the only entry point and it has exactly one `DiscoverySource` variant
(`GitTracked`) -- the argv-override path SPEC-2 describes has no representation in the type, so
there is nothing for a test to assert against.

BUG-1 fixed the adjacent defect (argv paths silently ignored for scoping) by making argv *filter*
the discovered set. That fix was correct for the case it addressed -- scoping a sweep to a
subdirectory -- and it quietly settled the override question in the other direction without the
spec line being revisited. The spec was not updated, so SPEC-2 still specifies override.

## References

- `rust/crates/urzua-cli/src/main.rs` -- `scope_to_requested_paths`, the intersection this bug is
  about, and the `ScopeInfo` construction that cannot report `argv`.
- `rust/crates/urzua-io/src/lib.rs` -- `discover_tracked_files` and `DiscoverySource`, whose single
  variant is why argv has no representation.
- SPEC-2 -- Discovery: "Explicit paths on argv override discovery and are used as given," plus the
  `scope.source` table declaring `argv`.
- BUG-1 -- the adjacent argv-scoping fix whose direction this bug re-opens against the spec.
- BUG-13 -- the untracked-on-disk distinction in dangling-pointer *messages*; same root condition,
  different surface.
- BUG-25 -- filed from the same run: `scope.source` emits a Debug-formatted enum variant, which is
  why the output above reads `GitTracked` rather than any declared value.
- ADR-6 -- git-tracked-only discovery, unchanged by this bug.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-11 | Initial bug record, `Status: Open`. Not fixed -- the fix direction (argv truly overriding discovery vs. amending SPEC-2 to match the implemented intersection, plus a finding when a requested path is dropped) is a real decision, not an obvious patch. | **structural** |
