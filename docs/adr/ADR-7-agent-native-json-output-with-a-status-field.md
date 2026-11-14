---
Stable-Id: 01M6R3NK4S1K3KKMGNV0RC498S
Status: Superseded
Embodiment: Deprecated
Realized-by: code:rust/crates/urzua-core/src/report.rs@sha256:5285d870dcf601bc0d5b58c4be42d9fb12845b16eb345526ee6e34d595e87215, spec:docs/specs/SPEC-2-urzua-check.md
Date: 2026-09-04
Accepted-on: 2026-09-05
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: ADR-23
Derives-from: RFC-3
---
# 7 — Every entry point emits JSON with a status field on stdout

## Context

RFC-3 proposed that every `urzua` entry point emit a structured JSON report on stdout — not just
as an opt-in flag, but as the primary contract every downstream consumer (a reviewing agent, a CI
gate, a human scripting against it) can act on without re-parsing prose. Phase 3 built exactly this
for `urzua check`: `status`, `files_examined`, `rules_executed` (with per-rule input population),
`scope`, `blocking`, and `findings`, selectable via `--format json` alongside a human-readable
default.

This ADR exists to decide the RFC now that the contract has been built and run against a real
corpus, rather than leaving a working, exercised design sitting in `Draft`.

## Decision

In the context of a CLI whose output needs to be consumed by both humans and agents/scripts, facing
the choice between a human-readable-only format and a structured one, we decided that **every entry
point emits a JSON report by default option (`--format json`), with the same shape `urzua check`
already ships**: `status` (`ok | findings-present | not-run` — never `ok` on a zero selection),
`files_examined`, `rules_executed` (each with the count of records it actually examined),
`scope`, `blocking`, and `findings` (each with `rule`, `severity`, `file`, `line`, `message`).

Accepting the field names as implemented (`snake_case`, matching `serde`'s default rather than the
RFC's illustrative `camelCase`) rather than re-deriving them — the implementation is the more
concrete artifact.

## Amendment (2026-11-14): a repository with no record yet is `ok`, with a notice

*Status of this amendment: decided by the orchestrator on 2026-11-14, pending maintainer review. `Status` stays `Accepted` and the Decision text above is unchanged.*

The Decision says `status` is never `ok` on a zero selection. A repository that has just run `urzua init --types` holds no record, and `doctor` and `check` on it must pass (the first-run review, finding 6.1); `not-run` there made the first command a new adopter runs in CI exit 2. This amendment narrows what "zero selection" means without changing the rule it states. A selection that is empty because the run could not look stays `not-run`: a path that matched nothing, a configuration with no rule enabled, a declared directory that is missing, a directory holding a file no type reads as a record. A run over a repository whose declared record directories all exist and hold no record is not an unestablished run: there is nothing to select, and the report says so with an info notice (`empty-corpus`) and `files_examined: 0`, so it is told apart from a pass over records. `git-diff` runs already worked this way for a diff that reaches no record (`SPEC-2`).

## Reversibility

The output contract is consumed by anything scripting against `urzua`. Changing field names or the
status enum after external consumers exist is a breaking change, not a patch — this is exactly the
kind of commitment that motivates a format/config stability policy (still open, tracked separately).
Cheap to revise before v0 ships to any external user.

## Consequences

- Every future command (`audit`, `migrate`, `export`/`import`, `doctor`) must emit this same shape,
  not invent its own — a second, incompatible JSON shape would defeat the point.
- `rules_executed`'s per-rule input population is now a hard requirement for every future rule, not
  an aspiration: a rule that can't report how many records it actually examined isn't done.
- The human-readable format is not contractual and may change formatting without a breaking-change
  concern; only the JSON shape is the contract.

## References

- RFC-3 — the proposal this decides.
- `docs/specs/SPEC-2-urzua-check.md` — the concrete output contract this ADR ratifies.
- `rust/crates/urzua-core/src/report.rs` — the implementation.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Pinned `rust/crates/urzua-core/src/report.rs` by content hash after re-verifying it against the Decision (attested by beauwilliams). **Why:** Re-verified for a Superseded record: report.rs changes are the refs and citation.issue-exists additions; the status-field JSON output this record decided is unchanged. | **structural** |
> | 2026-11-05 | `Embodiment` set to the declared sticky state `Deprecated` (was `Implemented`). Reason: drift review of 2026-11-04. This record is Superseded by `ADR-23` and its Decision no longer describes the code: it decides an opt-in `--format json` flag with human-readable output by default, while the CLI has no `--format` flag and prints only JSON. Re-baselining the pin on `report.rs` would certify a claim the code does not make. **Why:** a Superseded record's evidence is not re-verified; `Deprecated` is human-set and `embodiment.consistency` leaves it alone. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/report.rs` 81ca2e7df37e -> 206e1e3374db. Reason: report.rs gains the FindingSeverity::Info variant (serialized "info"), FindingSeverity::rank and an Implements: ADR-87 comment (ADR-87, MILE-194); the status field and the report shape this decision records are unchanged, so the evidence still holds **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 206e1e3374db -> 60b6889bf4eb. Reason: Read the diff of each pinned span since the old pin and the claim still holds. urzua-core/src/report.rs: added FindingSeverity::at_least, the deferred-by-scope population count and the untracked and pull-refs report fields, the JSON envelope (status values and exit codes) is as before. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 60b6889bf4eb -> f8332710ffd7. Reason: Read the diff of each re-pinned file. report.rs: added the AsOf and AsOfSourceKind types and an as_of field on CheckReport and FixReport; the existing fields, notices and exit_code contract are unchanged. Nothing this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` f8332710ffd7 -> 96587e15e363. Reason: Read the diff of each pinned file or range since the old pin against what this record evidences: report.rs gained an additive Population counter excused_lines (skipped from the JSON when zero) with an accessor and a with_excused_lines builder; nothing else in the report contract changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 96587e15e363 -> fd0b8c483216. Reason: Read the diff of each re-pinned file against main. core/report.rs: Notice gains an optional detail map of untagged string or string-list values, with DetailValue and Notice::new. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` fd0b8c483216 -> 83a590ba0c3e. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: report.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged; adds Finding::new, at, at_opt, declared, declared_opt and RuleExecution::ran, not_applicable, not_enabled; no existing item changed. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 83a590ba0c3e -> e4387ad0f746. Reason: Read the diff of each re-pinned file against main. report.rs: added WaiveReport, WaiveDraft and WaiveFindings with their Report impl for urzua waive; no existing type or function changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` e4387ad0f746 -> c870766d6d47. Reason: Re-pinned after PR4 of MILE-205 (urzua waive --apply), the stated delta being: report.rs: WaiveReport gains an optional error field and exits 2 when not-run; other reports unchanged. The claim holds as the changed code does not alter what this record states. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` c870766d6d47 -> a8e66b9f93f2. Reason: Re-pinned after PR4 of MILE-205 (urzua waive --apply), the stated delta being: report.rs: WaiveReport gets a WaiveStatus (ok, partial-failure exit 1, not-run exit 2) and an optional error field; other reports unchanged. The claim holds as the changed code does not alter what this record states. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` a8e66b9f93f2 -> 49678e010c60. Reason: report.rs gained the optional `baseline` object (`BaselineInfo`) on the check report, omitted when no baseline is declared; no existing field changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 49678e010c60 -> fad938df4898. Reason: Read the diff of rust/crates/urzua-core/src/report.rs against main. The Report trait gains a defaulted next_action() method, and report.rs gains NextAction, next_action_for_findings (it builds one runnable command from the unwaived findings' run-command argv, fix before index), the shell_word and union_command helpers, next_action impls for CheckReport and FixReport, and unit tests (MILE-72). No existing field, struct or serialization of any report changed, so the claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/report.rs` fad938df4898 -> b7d7648f4822. Reason: merge of origin/main into the MILE-207 branch: the pinned files differ from main's pins only by this branch's path-scoped source read (scoped sources in discovery.rs, check.rs, back_pointer.rs, runner.rs, table.rs, report.rs, fix.rs) and the 'not judged' notice wording; each record's own claim is untouched **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` b7d7648f4822 -> ab83ad474a70. Reason: Read the diff of each pinned file since the old pin and the claim still holds. report.rs: WaiveReport gains a notices field (skipped when empty) that Report::notices returns, and the doc comments on CheckReport's blocking and fail_on fields now say blocking is true exactly when the run exits 1 and the threshold moves the exit code and blocking; no code in the CheckReport or the shared report, notice and emit contract changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Amended: a repository with no record yet, every declared record directory present and empty of records, reports `ok` with `files_examined: 0` and an `empty-corpus` info notice; every other zero selection stays `not-run`. The Decision text is unchanged; `Status` stays `Accepted`. Decided by the orchestrator on 2026-11-14; wording pending maintainer review. **Why:** the first-run review (finding 6.1): `init`'s own output was `not-run` on an empty repository. | **substantive** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` ab83ad474a70 -> 80282b91b96b. Reason: Read the diff of each pinned file against main and the claim still holds. report.rs: WaiveDraft gained optional variant and key_path, serialized only when present; nothing else changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 80282b91b96b -> 5285d870dcf6. Reason: Read the diff of each re-pinned file against main. RuleExecution gained a skipped-from-JSON unread_markers field, the with_unread_markers builder and the matching test fixture literals; the other types and functions are unchanged. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
