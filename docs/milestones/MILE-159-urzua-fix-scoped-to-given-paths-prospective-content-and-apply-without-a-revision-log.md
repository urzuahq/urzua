---
Stable-Id: 01M6JKNWP4NMYSTVM6QQNMV9P9
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/fix_paths_integration.rs#a_path_limits_the_examination_and_the_repairs_to_that_record@sha256:1d68f8b2b3b4fd9d4e016bc501bb737f8cbd5020539c18d5715810a487b11d64, test:rust/crates/urzua-cli/tests/fix_paths_integration.rs#apply_with_a_path_writes_only_that_record@sha256:6a6bd813c8ecc1469d291ffacee27334fc97fa448aaa85659241f31d48121a4b, test:rust/crates/urzua-cli/tests/fix_stdin_integration.rs#stdin_content_is_judged_and_nothing_is_written@sha256:b58fff42500a5af8b609994213ea808b694859a99325333fee4fcf0f2d4a0d8e, test:rust/crates/urzua-cli/tests/fix_revision_log_required_integration.rs#not_required_apply_writes_the_field_without_a_log_row@sha256:a4bfebcb43ef2b6530b1fdb08d5c368d66371daa9ef78219c10b5cdef75fd16c, code:.urzua/config.yaml, code:rust/crates/urzua-cli/src/commands/fix.rs
---
# 159 — Urzua fix scoped to given paths, prospective content, and apply without a revision log

## What

Make `urzua fix` accept a path argument or prospective content, make `--apply` work for records that have no revision log section, and give the repair list a stable machine-readable form for an editor hook.

## Why

`urzua fix` is whole-corpus (several seconds on a few hundred records) with no path argument, and `--apply` requires a revision log section the adopter's records do not have, so there is no equivalent of a quick single-file refresh. An editor hook must parse the whole repair list to find its own file. Authoring speed is the point of `MILE-103`. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- `urzua fix <path>` examines and repairs only that path. — evidence: test:rust/crates/urzua-cli/tests/fix_paths_integration.rs#a_path_limits_the_examination_and_the_repairs_to_that_record@sha256:1d68f8b2b3b4fd9d4e016bc501bb737f8cbd5020539c18d5715810a487b11d64, test:rust/crates/urzua-cli/tests/fix_paths_integration.rs#apply_with_a_path_writes_only_that_record@sha256:6a6bd813c8ecc1469d291ffacee27334fc97fa448aaa85659241f31d48121a4b
- Prospective content on stdin is judged without writing. — evidence: test:rust/crates/urzua-cli/tests/fix_stdin_integration.rs#stdin_content_is_judged_and_nothing_is_written@sha256:b58fff42500a5af8b609994213ea808b694859a99325333fee4fcf0f2d4a0d8e
- `--apply` does not need a revision log section. — evidence: test:rust/crates/urzua-cli/tests/fix_revision_log_required_integration.rs#not_required_apply_writes_the_field_without_a_log_row@sha256:a4bfebcb43ef2b6530b1fdb08d5c368d66371daa9ef78219c10b5cdef75fd16c
- A single-file run completes in well under a second on a large corpus.
- The capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml, code:rust/crates/urzua-cli/src/commands/fix.rs

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Scope extended: the path-scoped run must also check prospective content for an editor hook (a file judged before it is saved), not only repair on disk. **Why:** a re-run of urzua over a real adopter corpus showed the editor-hook use needs it; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Progress, part 1 of 2: the drift cost and git-error surfacing landed (`fix` gates drift on the rule, per-record `git blame` runs on threads, git failures and shallow clones are `embodiment-drift-git` notices; whole-corpus `fix` 1.10s to 0.71s, `check` 1.32s to 0.89s on this corpus). The path/id scope, prospective content and apply-without-a-revision-log remain. **Why:** the foundation review named these causes of the whole-corpus cost and of the earlier false zero. | **substantive** |
> | 2026-11-04 | Progress, part 2a of 2: `urzua fix [PATHS...]` examines, computes drift for and repairs only the selected records, and reports a path matching no record (whole-corpus 0.78s, one record 0.15s on this corpus). `--stdin-path` and the revision-log `required` declaration remain. **Why:** the maintainer-approved design splits the remaining work into path scoping, prospective content and apply-without-a-log. | **substantive** |
> | 2026-11-04 | Progress, part 2b of 2: `urzua fix --stdin-path <path>` judges proposed content from stdin without writing, for a path that need not exist yet, with drift for that record alone (whole-corpus 0.89s, one stdin path 0.13s on this corpus); repairs carry a `line`. An example editor hook is `.git-hooks/examples/editor-fix-hook.sh`. The revision-log `required` declaration remains. **Why:** the maintainer-approved design for an editor hook. | **substantive** |
> | 2026-11-04 | Progress, part 2c of 2 (part 2 complete): a type's `revision_log` takes `required: true|false` (default `true`); `urzua fix --apply` writes the field without a log row where it is `false`, and where it is `true` a record with no section fails naming the record and the identity looked for. Declared (`true`) on `adr` in this repository's `.urzua/config.yaml`. **Why:** an adopter's records have no revision log section. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #214 (drift gated on the rule, git failures and shallow clones surfaced), #226 (`fix [PATHS...]` scopes examination, drift and repair), #229 (`fix --stdin-path` judges proposed content without writing) and #232; the maintainer relayed on 2026-11-04 that the milestone is fully delivered. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
