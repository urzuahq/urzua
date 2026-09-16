---
Stable-Id: 01M2P88GH66BQFHARXA411NZVM
Status: Open
Found-in: 'MILE-51 -- running `urzua init` against npryce/adr-tools, a real Nygard corpus whose records live under `doc/adr/`'
Regression-test: 'not yet written -- needs a fixture corpus rooted somewhere other than `docs/`, asserting `init` proposes a type whose `dir` is the real parent path and that `check` then examines a non-zero count with the config `init` wrote'
---
# 36 — urzua init cannot adopt a corpus outside docs/, and writes a dir under docs/ regardless

## What was wrong

`SPEC-5` describes `init` as adopt mode: *"propose and write `.urzua/config.toml` from what's already
there."* It only looks in one place.

```
$ urzua init --dry-run          # in a corpus with 9 records under doc/adr/
{"status": "not-run", "error": "no record-shaped files found under docs/ -- nothing to adopt"}
```

Four separate hardcodes, not one:

| Location | What |
|---|---|
| `init.rs:25` | `let docs_dir = PathBuf::from("docs")` — the scan root |
| `init.rs:56` | `dir: format!("docs/{dir}")` — the **proposed output path** |
| `init.rs:150` | the `CouldNotRun` message |
| `init.rs:21-23`, `SPEC-5` | the doc comment and the spec's worked example |

The second matters most: fixing only the scan root would propose `dir = "docs/adr"` for a corpus at
`doc/adr`, and `check` would then examine zero files while reporting success.

## Why nothing caught it

`docs/` is this repo's own layout, and every fixture in `check_integration.rs` builds a corpus under
`docs/` because that is what the tool writes. Nothing exercised adoption of a corpus this project did
not lay out. `MILE-51` exists to exercise exactly that and had never been run.

## Hazards a fix must handle, found while scoping one

Replacing the scan root with "group record-shaped files by parent directory" is the obvious fix and is
not sufficient on its own:

- **Nested directories.** `docs/adr/2023/0001-x.md` counts toward `adr` today (`init.rs:32-40`
  requires only *at least* one further component). Grouping by parent turns a corpus split by year
  into one record type per year.
- **A record directly under the scan root.** `docs/0001-x.md` yields a type `doc` with `dir = "docs"`,
  and `load_records` matches by path prefix (`discovery.rs:60`) — so every record in the repository
  would additionally load as that type. A record at the repository root is worse: the parent is empty
  and `Path::starts_with("")` is true for everything.
- **TOML injection.** `render_config_toml` interpolates the derived name and path unquoted
  (`init.rs:90-91`). Constrained today to `docs/` subdirectory names; after the fix it is every
  directory in the repository. A directory named `adr.v2` emits `[record_types.adr.v2]` — a nested
  table — which `deny_unknown_fields` (`config.rs:21`) then rejects, making `init`'s own output
  unreadable by `check`.

## References

- MILE-51 -- the validation run that found this; the full verdict and its four sibling gaps.
- SPEC-5 -- describes adopt mode, and its worked example shows `"dir": "docs/adr"`.
- BUG-37 -- the other half of adoption's filename assumptions, found in the same run.
- ADR-6 -- discovery is git-tracked, which a fix must preserve rather than becoming a filesystem walk.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Initial record, `Status: Open`. **Why:** filed rather than fixed -- the obvious two-line fix proposes a wrong `dir` and opens three further hazards, so this needs a real `init` rework rather than riding along with a validation result. | **structural** |
