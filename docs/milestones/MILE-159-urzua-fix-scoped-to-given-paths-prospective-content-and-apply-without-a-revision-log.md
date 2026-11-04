---
Stable-Id: 01M6JKNWP4NMYSTVM6QQNMV9P9
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/fix.rs, code:rust/crates/urzua-cli/src/discovery.rs, code:rust/crates/urzua-core/src/fix.rs, test:rust/crates/urzua-cli/tests/fix_paths_integration.rs, test:rust/crates/urzua-cli/tests/fix_stdin_integration.rs, test:rust/crates/urzua-cli/tests/fix_drift_git_integration.rs
---
# 159 — Urzua fix scoped to given paths, prospective content, and apply without a revision log

## What

Make `urzua fix` accept a path argument or prospective content, make `--apply` work for records that have no revision log section, and give the repair list a stable machine-readable form for an editor hook.

## Why

`urzua fix` is whole-corpus (several seconds on a few hundred records) with no path argument, and `--apply` requires a revision log section the adopter's records do not have, so there is no equivalent of a quick single-file refresh. An editor hook must parse the whole repair list to find its own file. Authoring speed is the point of `MILE-103`. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

`urzua fix <path>` examines and repairs only that path; prospective content on stdin is judged without writing; `--apply` does not need a revision log section; a single-file run completes in well under a second on a large corpus; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

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
