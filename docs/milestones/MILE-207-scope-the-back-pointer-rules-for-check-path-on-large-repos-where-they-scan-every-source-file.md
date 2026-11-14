---
Stable-Id: 01M6SNCTYDQX8F9XMB862VHV42
Status: Done
Phase: '1'
Track: scale-and-analytics
Blocked-on: null
---
# 207 — Scope the back-pointer rules for check PATH on large repos, where they scan every source file

## What

Scope the source-comment back-pointer rules (`embodiment.back-pointer-resolution` and
`embodiment.back-pointer-decision-accepted`, and `code.dangling-bug-reference`, which shares the comment scanner) for a path-scoped run, so `urzua check <path>` on a large
repository does not read every source file under the declared `back_pointer_paths` to report on a few
records. The mechanism is for the milestone's design to settle; this record only names the cost to remove.

## Why

On a large repository `check <path>` takes about 4.6 s, because these rules scan every source file whatever
the path asked about: the scan is the cost and the records in scope are not. Other corpus-wide work is
already scoped to the records a path or `--base` run reports on (`SPEC-2`: drift and claim ages are computed
for those records only), and a path-scoped run is the one the editor hook and the post-command check make
after every edit, so the scan is paid on every edit. The maintainer placed this in Phase 1 on 2026-11-06:
it does not block the Phase 0 exit.

## Done means

- A path-scoped `urzua check <path>` on a repository with the back-pointer rules enabled reads only the source files it needs for the records in scope, and reports the same findings for those records as a whole-corpus `check` does. — evidence: code:rust/crates/urzua-cli/src/discovery.rs@sha256:8a1352dbe444cdcbcd1f4e2a6ba6f53065dec5271f4c500bc6e8a27db8ae5908, code:rust/crates/urzua-cli/src/commands/check.rs@sha256:8b9d115c598a22aad073eec3cea1f20047f283771b05222e0d7e15b52750d714, test:rust/crates/urzua-cli/tests/scoped_source_read_integration.rs@sha256:6ae7a53f63fb068778ececed3a035a31b6dddf7445a0c32b8a7b1174ffe8acb9
- A test with a source file that cites a record outside the scope shows it is not read, and one that cites a record inside the scope shows it is. — evidence: test:rust/crates/urzua-cli/tests/scoped_source_read_integration.rs@sha256:6ae7a53f63fb068778ececed3a035a31b6dddf7445a0c32b8a7b1174ffe8acb9
- A whole-corpus `check` and a `--base` run are unchanged: the same findings, the same populations. — evidence: test:rust/crates/urzua-cli/tests/scoped_source_read_integration.rs@sha256:6ae7a53f63fb068778ececed3a035a31b6dddf7445a0c32b8a7b1174ffe8acb9
- The rule population still says what was examined, so a scoped run is distinguishable from one that read nothing (`ADR-55`). — evidence: code:rust/crates/urzua-core/src/rules/back_pointer.rs@sha256:7722c19b1731a0abeeaf69b6fc35f68968440c5baf9520a2c117decf38a6c634, test:rust/crates/urzua-cli/tests/scoped_source_read_integration.rs@sha256:6ae7a53f63fb068778ececed3a035a31b6dddf7445a0c32b8a7b1174ffe8acb9
- The saving is measured on a repository of the size that costs about 4.6 s today, and the before and after times are recorded in this record's revision log. — evidence: code:scripts/bench-scoped-check.sh@sha256:088351caa652327abf8c265a1af82ed1076a81f5f85a8bb7453570970441f482
- `SPEC-2` states the scoped read, with a revision row. — evidence: spec:docs/specs/SPEC-2-urzua-check.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Initial milestone. **Why:** found while scoping `check <path>` on a large repository, where the back-pointer rules scan every source file and the run takes about 4.6 s; the maintainer placed it in Phase 1 on 2026-11-06. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-14 | Delivered in full: a path-scoped `check` reads the source files under the requested paths and those whose `Implements:` or `Verifies:` comments cite a record in scope, for `embodiment.back-pointer-resolution`, `embodiment.back-pointer-decision-accepted` and `code.dangling-bug-reference` (which shares the comment scanner and is scoped the same way); the others are counted in each rule's population as `deferred_by_scope` with an `info` notice. Measured with `scripts/bench-scoped-check.sh` on a 2,361-tracked-file repository (this repository's own tree with two further copies of `rust/` under `back_pointer_paths`), median of five runs, release builds: `check <one record>` 4.61 s before and 1.91 s after, `check docs/adr` 4.74 s and 2.16 s, `check <one source file>` 4.51 s and 1.97 s; whole-corpus `check` 5.80 s and 5.75 s and `check --base` 4.49 s and 4.23 s (unchanged). On 5,580 source files the one-record run takes 12.96 s before and 2.28 s after. What a scoped run still costs is not the source scan (the three rules read the files in about 0.3 s) but the rest of the rule set. A finding on a file outside the requested paths that is not about a record in scope (a back-pointer or comment citing no record) is no longer reported by a run that did not ask about that file, which is what a scoped read means; a whole-corpus run reports it as before. **Why:** a path-scoped run is the one the editor hook and the post-command check make after every edit, and it scanned every source file to report on a few records. | **substantive** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-cli/src/commands/check.rs` f942ff143328 -> 762c7c56bdb5, `rust/crates/urzua-cli/tests/scoped_source_read_integration.rs` 867ae489625e -> 6ae7a53f63fb. Reason: check.rs: the path-scope notice now says source files are 'not judged' instead of 'not read' (one string); scoped_source_read_integration.rs: the notice test asserts the new wording; no behaviour change **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/check.rs` 762c7c56bdb5 -> fe2fce22eac3. Reason: Read the diff of each pinned file since the old pin and the claim still holds. check.rs: blocking comes from the fail_on threshold (gate_blocking), a path argument that selects no record adds a warning notice, last in the notices (unmatched_path_notices), and a not-run full report adds a reason notice unless that notice already says why (not_run_reason); session open, scope, rule running, waivers and status derivation are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/check.rs` fe2fce22eac3 -> 8b9d115c598a. Reason: Read the diff of each pinned file against main and the claim still holds. check.rs: run_after_scoping computes the repository-relative config file and hands it to the rules in RunState; no other check logic changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
