---
Stable-Id: 01M6R3NK4AD6DE9KGA6Y2X428S
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/fix.rs@sha256:db0e96b74575090e7b3ee95ce37f766fea9da6f9a75155e951c11aa0f0bd08e9, code:rust/crates/urzua-io/src/lib.rs@sha256:af982e92b370a88a91d9765ab6859df869e1dcac60bf3b7eb7ccece60dbb5506, test:rust/crates/urzua-core/src/fix.rs@sha256:db0e96b74575090e7b3ee95ce37f766fea9da6f9a75155e951c11aa0f0bd08e9
Date: 2026-09-05
Accepted-on: 2026-09-05
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: RFC-8, ADR-15, ADR-19
---
# 20 — `urzua fix --apply`: writes real files, gated hard

## Context

ADR-19 shipped `urzua fix` detect-mode only, naming apply mode's three missing dependencies:
real file mutation of a specific field without disturbing anything else, identity resolution
(RFC-2 §2's tiering), and a revision-log write-path (ADR-14 defined the shape, never the
append mechanics). All three now exist.

## Decision

In the context of ADR-19's deferred apply mode, facing the same eligibility test ADR-15 already
set (derivable, single-valued, non-destructive, reversible), we decided apply mode ships with every
one of the following as a hard gate, not a nicety:

- **`--ids <record,...>` or `--force` is required.** Nothing applies by default; `--force` is never
  implied.
- **Identity is required.** `--by` wins outright; else `gh api user`; else `git config user.name`
  (RFC-2 §2's tiering); none resolving is a hard error, never a placeholder author.
- **A missing Revision log section refuses that record's write outright.** ADR-14's reversibility
  clause is not optional per-write — a repair with nowhere to record itself is not applied, the
  record is reported as failed, and every other selected repair still proceeds independently.
- **Re-verification at write time.** The exact file is re-read from disk immediately before writing;
  `apply_repair` itself fails if the current value it expects is no longer present, which is
  precisely the shape a concurrent edit since detect ran would take (RFC-8 §4).
- **A corpus-level lock** (`.urzua/cache/fix.lock`, atomic `create_new`) serializes concurrent
  `--apply` invocations; a second process fails loud rather than interleaving writes.
- **The edit touches only the one field's line.** The rest of the record — every other field, all
  prose — is untouched, byte-for-byte, verified by test.

Partial failure is a first-class outcome: selecting five repairs where one record has no revision
log applies the other four and reports the fifth as failed, rather than the batch succeeding
silently short or aborting everything for one record's gap.

## Reversibility

Every applied write is itself reversible by construction: `git diff`/`git revert` on the commit, and
the revision-log row the write appends names the exact evidence and prior value, so "what changed
and why" is answerable without git at all.

## Consequences

- `Command::Fix` gains `--apply`, `--ids`, `--by`, `--force`. None of these existed as flags before
  this ADR — ADR-19 deliberately didn't add them ahead of a real apply path.
- `urzua-io` gains `resolve_identity` and `FixLock` — the first genuinely mutating capability
  anywhere in this codebase. Both live in `urzua-io`, not `urzua-core`, keeping the purity boundary
  (ADR-5/6) intact: `urzua-core::fix::apply_repair` is a pure `&str -> Result<String, String>`
  transform; the CLI performs the actual read, lock, identity resolution, and write.
  `urzua-core`'s purity test must still pass with this ADR's dependencies in place — asserted, not
  assumed.
- `migrate schema --apply`'s planned delegation to `urzua fix` (designed alongside RFC-8's
  acceptance) is now genuinely unblocked, since a real apply path exists to delegate to.
- Two things this ADR deliberately does not build: a stale-lock cleanup mechanism (a crashed process
  leaves `fix.lock` behind, requiring manual removal — acceptable for a first cut, a real gap) and
  Tier 2/3 repairs (still only Embodiment, still only Tier 1).

## Amendment (2026-11-04): identity order and the revision-log requirement as built

*Status of this amendment: accepted 2026-11-04 by the maintainer (drafted 2026-11-04 from the drift review the maintainer approved). The Decision text above is unchanged.*

Two statements in the Decision no longer match the code. The hard gates `--ids` or `--force`, the
re-read before writing, the corpus lock and the single-line edit are unchanged.

- **Identity order.** The Decision says "`--by` wins outright; else `gh api user`; else `git config
  user.name`". `resolve_identity` (`urzua-io/src/lib.rs`) now resolves `gh api user`, then an explicit
  `--by`, then `git config user.name`, then an error. `gh api user` outranks `--by`; when the two
  differ the caller reports a warning instead of discarding `--by` silently. This is the order
  `ADR-31`'s amendment of 2026-09-10 decided; this ADR was not amended when it was built.
- **Missing Revision log.** The Decision says a missing Revision log section refuses that record's
  write outright. It now refuses only for a type that requires a log: a type that declares
  `revision_log: {required: false}` is applied without one. A type that requires a log, and every
  `--rebaseline` write, still refuse a record with no log section.

## References

- ADR-19 — the detect-mode-only decision this ADR completes.
- ADR-15/RFC-8 — the eligibility test and tier model this apply path implements.
- ADR-14 — the revision-log shape every applied write appends to.
- RFC-2 §2 — the identity-resolution tiering reused here.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | `Embodiment` set to `Drift detected`. **Why:** Re-verified against current code; maintainer decision 2026-11-04. The Decision says `--by` wins outright, but `resolve_identity` in `urzua-io/src/lib.rs` now lets `gh api user` outrank `--by` (`ADR-31` amendment). | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 45f33fd12. | **structural** |
> | 2026-11-04 | Amendment added: the identity order (`gh api user`, then `--by`, then `git config user.name`, per `ADR-31`) and the per-type Revision log requirement, both of which the Decision states otherwise. **Why:** drift review of 2026-11-04, approved by beauwilliams; proposed, pending maintainer acceptance. The Decision is unchanged. | **substantive** |
> | 2026-11-04 | Pinned `rust/crates/urzua-core/src/fix.rs`, `rust/crates/urzua-io/src/lib.rs` by content hash after re-verifying it against the Decision (attested by beauwilliams). **Why:** Re-pinned with the amendment above: the identity order and Revision log requirement are recorded as built; the hard gates the Decision lists are unchanged. | **structural** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-core/src/fix.rs@sha256:bc5037c801aaea61e15d8997187f705fce7d92c39e90c4e96679cc5a5c88524a, code:rust/crates/urzua-io/src/lib.rs@sha256:671f9e9b0ff5bc3afc72a7ef898610551f4bb00ed91179bd1a0d35d24b57602d, test:rust/crates/urzua-core/src/fix.rs@sha256:bc5037c801aaea61e15d8997187f705fce7d92c39e90c4e96679cc5a5c88524a` -- stated `Drift detected` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-04 | Amendment of 2026-11-04 accepted. **Why:** Maintainer decision, 2026-11-04; the Decision text is unchanged. | **substantive** |
> | 2026-11-04 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/fix.rs` bc5037c801aa -> 772cef0da7b4. Reason: Re-verified: the only change to the pinned file is an Implements: ADR-67 comment; behaviour is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/fix.rs` 772cef0da7b4 -> 5b3a0904b521, `rust/crates/urzua-io/src/lib.rs` 671f9e9b0ff5 -> 06c483542fda. Reason: Re-verified against the Decision and its accepted amendment of 2026-11-04. Since the pin, fix.rs reads the Embodiment vocabulary from config (the stated drift state is config.embodiment.is_drift, MILE-144), edits a field value by position on the field's own line and never by substring (BUG-192, a stricter form of the single-line edit gate), and gained PinRead and FailedPin for re-pinning scoped pins (ADR-74); urzua-io/src/lib.rs only gained an added set on GitChanges and sha256_hex_of_str. The hard gates (--ids or --force, re-read before writing, corpus lock, one-line edit) and the identity order recorded in the amendment are unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/fix.rs` 5b3a0904b521 -> c706b63f9582. Reason: BUG-74 renamed Waiver to Suppression (and load_waivers to load_suppressions) in this file's internals; the behaviour this record cites is unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` c706b63f9582 -> c9cd87e7b00e, `rust/crates/urzua-io/src/lib.rs` 06c483542fda -> 671703be6a33. Reason: Read the diff of both pinned files: fix.rs only widened the private log_row helper to pub(crate) so reverify writes its revision rows with the same layout as fix --rebaseline, apply mode itself is untouched; urzua-io lib.rs only gained the new commits_touching_since git-log helper beside commit_for_line, no existing function changed; the apply-mode claim still holds. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Correction to the `Re-baselined` row of 2026-11-08 that re-pinned `urzua-core/src/fix.rs` c706b63f9582 -> c9cd87e7b00e and `urzua-io/src/lib.rs` 06c483542fda -> 671703be6a33. That reason described only the engine pull request's last change and so understated the delta since the old pins; the earlier row stays as written. Real delta, from the diff of the old pinned content against the current files: `fix.rs` (+667/-76) gained `IdentitySource`, `ActorKind` and `Attestation` (a revision row names the identity source and the actor kind), `replace_exact_pin` (a re-baseline rewrites the exact pin and never one whose spelling merely ends with it), `log_row` with a declared revision-log shape (the row's tool class and column layout come from the type's declaration, and `apply_rebaseline` writes through it) and tests for each; `lib.rs` (+1090/-80) gained `IdentityPolicy` and `resolve_identity_with` (a configurable identity source, and a `--by` or login holding a pipe character or a control character is refused), `confine_to_root`, `write_atomic` and `write_atomic_in`, the untracked-file discovery helpers (`untracked_governed_files`, `files_in_dirs_at_commits`, `read_at_commit_if_present`), `commits_touching_since` and `last_commit_for_paths`, and `FixLock` now reports a failure other than contention as that failure. The claim still holds: `apply_repair` still refuses a record with no Revision log when its type requires one and still fails when the stated value is no longer present, `FixLock` still takes the corpus lock by atomic create, and `resolve_identity` under the default policy still resolves the gh login, then `--by`, then `git config user.name`, as the amendment of 2026-11-04 records. **Why:** review of PR 433: the reason stated for a re-pin has to describe the real change between the old pinned content and the new. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` c9cd87e7b00e -> 88c815c0639a. Reason: Read the diff of each pinned file: core fix.rs moved apply_rebaseline's evidence-line lookup and revision-row tail into shared helpers (evidence_edit_lines, append_log_rows, revision_log_refusal) that apply_reverify also calls, with the same rows and error text; cli commands/fix.rs gained validate_reason, back_pointer_evidence, into_failures and identity_notices extracted from fix and reverify with the same behaviour; core reverify.rs calls the helpers and its first-pin matcher no longer splices into a longer anchored locator; cli commands/reverify.rs now refuses a record with no usable revision log in detect, refuses any non-human actor at an error-level period, and honours --ids under --force. Each claim these records make about those files still holds. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-io/src/lib.rs` 671703be6a33 -> d6cb76ca3011. Reason: Read the diff of each re-pinned file. urzua-io lib.rs: today() and now_unix() now read one evaluation clock (set by --as-of, else the system clock read once) instead of calling Utc::now each; added AsOfSource, as_of_source, set_as_of and parse_as_of, and a unit test for strict date parsing; nothing else in the file changed. Nothing this record relies on changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` 88c815c0639a -> db0e96b74575. Reason: fix.rs: this change makes the re-baseline refusal message name the type's declared item section instead of the literal Done means (a new items_section field carried to it, with a test) and moves the failed-pin listing in detect_rebaselines into a helper to stay under the SPEC-22 bound; unrelated to the claim this record evidences, which still holds. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-io/src/lib.rs` d6cb76ca3011 -> af982e92b370. Reason: urzua-io lib.rs gained BlameCache (one git blame per file with a -L range per wanted line, memoized commit times; a panicked blame thread falls back to commit_for_line_in per line), run_blame and a shared cat_file_batch helper; existing functions refactored onto run_blame / cat_file_batch, behaviour unchanged: commit_for_line_in runs through run_blame, files_in_dirs_at_commits and list_trees through cat_file_batch; outputs byte-identical **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
