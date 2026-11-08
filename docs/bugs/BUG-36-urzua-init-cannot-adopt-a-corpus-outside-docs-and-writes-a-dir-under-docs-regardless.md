---
Stable-Id: 01M2P88GH66BQFHARXA411NZVM
Status: Fixed
Found-in: 'MILE-51 -- running `urzua init` against npryce/adr-tools, a real Nygard corpus whose records live under `doc/adr/`'
Realized-by: test:rust/crates/urzua-cli/src/commands/init.rs#a_directory_name_full_of_yaml_metacharacters_survives_render_and_reparse_observed_failing@sha256:d1af0fe994bb8db05af0122b9030213ff89e99dfb567481c1daedfcd760fc6e8, test:rust/crates/urzua-cli/src/commands/init.rs#every_directory_holding_records_becomes_its_own_type@sha256:4a4118c534f304d275296c6fcacd265c40b71751451719ff09bed782c120c312, test:rust/crates/urzua-cli/src/commands/init.rs#detects_one_type_per_populated_docs_subdirectory@sha256:479569a982504078347a515ffd3988a03375c1468da0772c6fa28f85e6a28a1b, test:rust/crates/urzua-cli/src/commands/init.rs#a_disambiguated_name_can_still_collide_with_an_unrelated_directory_observed_failing@sha256:1c5139d0fcc9ba2993d793644c14dd93aaf36f784fb2318c24d250576dde8668
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
> | 2026-09-17 | Still `Open`. **Why:** the `ADR-52` changeset claimed this was closed by rendering the config through a real serializer. It was not. That change fixed a neighbouring hazard -- a directory name carrying the format's metacharacters escaping its value -- which `MILE-51` noted *beside* this bug rather than as part of it. The four hardcoded `docs/` paths are untouched, and `init` still cannot adopt a corpus rooted anywhere else. | **substantive** |
> | 2026-09-17 | Still `Open`, and now mechanically protected. **Why:** the false close claim that named this bug is the reason `claim.status-agreement` exists -- a changeset announcing it closed this record while the record read `Open`, in a file nothing cross-checked. The rule was observed failing against that exact line before the fix landed. | **substantive** |
> | 2026-09-17 | `Status: Open` → `Fixed`. **Why:** `detect_record_types` groups by each record's own parent directory from the tracked set, so `doc/adr/` adopts as `dir: doc/adr` rather than being invisible. All four hardcodes are gone, including `:56`'s `format!("docs/{dir}")`, which was the one that mattered -- fixing only the scan root would have proposed `docs/adr` for a corpus at `doc/adr` and left `check` examining zero files while reporting success. Two hazards `MILE-51` named are handled explicitly: a proposed directory containing another is dropped, because discovery matches by path prefix and the outer one would claim the inner one's records; and two directories ending in the same component are qualified rather than emitted as a duplicate type name. Verified end to end on a three-record corpus at `doc/adr/`: `init` ok, `check` examines 3, non-blocking. | **substantive** |
> | 2026-11-04 | `Regression-test` filled in: the real value. `Status` is untouched. **Why:** `field.pending` now reads a `Regression-test` placeholder on a `Fixed` bug as a real gap (`MILE-169`), and the `Regression-test` field named a test that was not yet written; it now names the `init` tests covering the proposed `dir` and the rendered config. | **structural** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 4 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs#a_directory_name_full_of_yaml_metacharacters_survives_render_and_reparse_observed_failing` (lines 850-867) 45a089680d49 -> d1af0fe994bb. Reason: the pinned test span's only change is the added `exclude_files: vec![]` fixture field; the behaviour this record relies on is unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
