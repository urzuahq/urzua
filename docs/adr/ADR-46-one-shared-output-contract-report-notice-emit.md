---
Stable-Id: 01M286A8CHGVMQ2WTSPVX8G3CK
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/report.rs@sha256:e4387ad0f746f9d6e23f403c03d2488d12a5993bf0bf80dbfa69b5ff612c3a82, code:rust/crates/urzua-cli/src/main.rs@sha256:6aea9772c9364ad43f895e05d5ea7f394f8593400a13428cd545e356bfbd1a1e, test:rust/crates/urzua-core/src/report.rs@sha256:e4387ad0f746f9d6e23f403c03d2488d12a5993bf0bf80dbfa69b5ff612c3a82, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:a60c9c61b94c5707e0aa4e96713a843556cd50bce97e01c514826d61ca0aac08
Date: 2026-09-11
Accepted-on: 2026-09-16
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: BUG-19
---
# 46 — One shared output contract: `Report`, `Notice`, `emit()`

## Context

BUG-19 found `check`, `fix`, and `new`/`explain`/`graph`/`doctor` each hand-rolling a different,
incompatible JSON shape, contradicting ADR-7's own stated rule that every future command must share
one contract. The immediate trigger was narrower: `resolve_identity`'s gh-vs-`--by` divergence notice
was a bare `eprintln!` citing an internal ADR number -- meaningless to a tool consumer with no such
ADR in their own repo, and unparseable by an agent. Fixing that one call site surfaced the wider gap.

Three real design questions had to be settled before writing any of this, in order:

1. **How does a non-fatal observation (a "notice") coexist with `check`'s existing `Finding`?**
   `Finding.file: PathBuf` is mandatory -- every `check` finding is about a file. A notice like the
   identity divergence isn't about a file at all; forcing it into `Finding` means lying with a
   placeholder path.
2. **Does a shared shape mean one generic envelope, or something narrower?** An early design
   (`Envelope{status, notices}` composed in via `#[serde(flatten)]`) was killed by a compiled repro:
   flattening a struct with its own `status` field into `CheckReport` (which already has one)
   produces silent duplicate `"status"` keys in the output JSON -- `serde_json` emits both, no error,
   and any real consumer resolves that by silently dropping one. Renaming fields around this doesn't
   remove the hazard class; only removing the composition mechanism does.
3. **Can a non-fatal notice ever contradict the exit code?** Not if the type makes it
   unrepresentable -- see Decision below.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Generic `Envelope` struct, `#[serde(flatten)]`-composed into every report | One literal shared type | Confirmed duplicate-key hazard on any report that already owns a same-named field; degrades typed payloads (`GraphEdge`, etc.) toward an untyped `serde_json::Value` if pushed further |
| Bare `"warnings": [String]` (the stopgap already shipped for the identity notice) | Cheapest | No `severity`/`subject` an agent can match on; a second command copying it becomes a third incompatible shape by omission |
| `Report` trait (behavior, not shared fields) + a real `Notice` type, no envelope | Every report stays its own fully-typed struct; `notices()`/`exit_code()` are the only shared surface | Slightly more boilerplate per report type (each declares its own `notices: Vec<Notice>` field directly) |

## Decision

In the context of five incompatible output shapes and one confirmed duplicate-key bug in the
composition approach, facing the need for one real contract without sacrificing type safety, we
decided:

1. **No `Envelope` struct, no `#[serde(flatten)]` anywhere in this design.** `CheckReport` keeps its
   own `status: ReportStatus` field exactly as before. Every report struct (`CheckReport`,
   `GraphReport`, `ExplainReport`, `NewReport`, `FixReport`, `MigrateSchemaReport`, `DoctorReport`,
   `CouldNotRun`) plainly declares its own `notices: Vec<Notice>` field directly -- nothing composed
   in, nothing that can collide.
2. **A `Report` trait carries behavior, not shared fields**: `fn notices(&self) -> &[Notice]` and
   `fn exit_code(&self) -> ExitCode`. One shared `emit<T: Report>(report: &T) -> ExitCode` function
   (in `urzua-cli`, not `urzua-core` -- printing is I/O, and `urzua-core` stays pure per ADR-5)
   replaces the five hand-rolled `println!`/`json!` call sites.
3. **`Notice`'s severity is a deliberately smaller enum than `Finding`'s** -- `NoticeSeverity` has
   only `Info`/`Warning`, no `Error` variant, so a notice that would contradict "notices never move
   the exit code" is unrepresentable, not just undocumented. `subject` is a `const &str` per emitter,
   matching `Finding.rule`'s existing convention exactly (an owned `String` field populated from a
   constant), not a new, subtly different convention.
4. **Every fatal "could not run" path collapses onto one `CouldNotRun` type.** Before this, there were
   three different patterns: `report_could_not_run` (a full `CheckReport` shape with empty fields,
   used by `check`/`audit`), `report_fix_could_not_run` (`fix`'s own smaller shape), and bare
   `eprintln!` with nothing on stdout at all (`new`, `explain`, `graph`, `doctor`'s early guard,
   `migrate schema --report`). `check`/`audit` specifically lose the `scope`/`rules_executed`/
   `files_examined` keys they used to emit even on failure (present-but-empty before, absent now) --
   a real, narrower breaking change than "previously-silent stdout now emits JSON," which is what
   changes for the other commands.
5. **`DoctorReport` keeps its own real exit code, not one derived from `status` alone.** `run_doctor`
   has three real exit codes (2 = config missing, 1 = config parse error or any check `Error`, 0 =
   otherwise) but the first two both produce the same `DoctorStatus::Error` value -- deriving
   `exit_code()` from `status` alone would silently collapse them. `DoctorReport` carries an explicit,
   non-serialized `exit_code: u8` set by whichever branch constructs it, so the JSON shape (`status`,
   `checks`) is untouched but the real 2/1/0 split can't be lost to a future refactor.
6. **`Cli::parse()` becomes `Cli::try_parse()`**, with an explicit carve-out: `ErrorKind::DisplayHelp`/
   `DisplayVersion` print as clap already rendered them and exit 0 (never JSON-wrapped) -- matching
   `cargo`'s own convention that help/version are never gated behind a machine-format flag. Every
   other parse error goes through the same `CouldNotRun`/`emit()` path as any other fatal error.
7. **A `std::panic::set_hook` closes the one remaining stderr leak**: Rust's default panic hook always
   writes raw text to stderr regardless of anything else in this decision. The replacement hook writes
   a minimal ad hoc JSON object (`{"status": "not-run", "panic": ...}`) via `write_all` with the
   `Result` ignored -- deliberately not `println!`/any panicking macro, since a broken pipe during the
   hook itself would double-panic and abort with no output at all. Deliberately **not** routed through
   `Notice`/`Report` -- a panic is a lower-level escape hatch outside the five-command contract, not
   another instance of it.
8. **Stderr is eliminated for anything this code can structure into JSON, including genuine errors** --
   narrower than but building on ADR-26 (see that ADR's own dated Amendment for the specific reasoning
   and the sentence it supersedes).

## Reversibility

Every report type's JSON shape changes (new `notices` field; `check`/`audit`'s failure path loses
three keys; `"warnings": [String]` becomes `"notices": [Notice]`). Pre-1.0, no known external
consumer -- cheap now, expensive once one exists, the same reversibility profile ADR-23 already
named for this exact contract.

## Consequences

- Every future command's report type must implement `Report` and go through `emit()` -- a new
  command inventing its own `println!`/`json!` block is now a real regression, not just untidy.
- `Notice.severity` can never be `Error` by construction -- a future maintainer who wants to add a
  fatal-severity notice will hit a compile error reaching for a variant that doesn't exist, which is
  the point.
- `urzua-core` gained zero new dependencies -- `emit()`'s `serde_json`/`println!` live in
  `urzua-cli`, keeping `urzua-core`'s "no I/O, no filesystem, no process" claim (ADR-5) intact and
  mechanically checked (`urzua-core/tests/purity.rs`).
- `run_migrate_ids` and `run_init` remain outside this contract -- both are plain-text prose end to
  end today (not just their error paths), a materially different redesign than a drop-in `Report`
  struct. Filed as its own bug, not fixed here.

## References

- BUG-19 -- the finding this ADR resolves.
- ADR-7 -- "every future command must emit this same shape, not invent its own," the rule this
  finally realizes.
- ADR-23 -- stdout is always JSON, unconditionally; this ADR's contract is the concrete shape that
  claim now has for every command, not just `check`.
- ADR-26 -- narrowed further by this ADR's point 8; see its own dated Amendment.
- ADR-5/ADR-9 -- `urzua-core`'s purity boundary, which `emit()` living in `urzua-cli` (not
  `urzua-core`) respects.
- `rust/crates/urzua-core/src/report.rs` -- `Report`, `Notice`, `NoticeSeverity`, `CouldNotRun`, and
  every concrete report type.
- `rust/crates/urzua-cli/src/main.rs` -- `emit()`, the panic hook, `try_parse()`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Embodiment` recomputed by `urzua fix`: `Drift detected`. **Why:** every file named in `Realized-by` has changed since this line was last verified, and this pass did not re-verify it, so the record states the drift instead of a stale `Verified`; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-core/src/report.rs, code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-core/src/report.rs, test:rust/crates/urzua-cli/tests/check_integration.rs` -- stated `Verified` disagreed with the computed value. | **structural** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 687269a79. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 4 locators pinned, 4 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/report.rs@sha256:60b6889bf4eb, rust/crates/urzua-cli/src/main.rs@sha256:9fae4c3edf84, rust/crates/urzua-core/src/report.rs@sha256:60b6889bf4eb, rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b833 Reason: Pinned the files as they are now after checking each against the record it evidences: header.rs still parses the declared YAML frontmatter shape (ADR-17), fix.rs still has detect_repairs (ADR-19), urzua-id still builds ULID stable ids (ADR-21), main.rs still prints JSON with no --format flag on check (ADR-23), urzua-io still resolves the attesting identity in the ADR-31 order, scripts/generate-dashboard.py still generates the static dashboard (ADR-37), report.rs still defines Report and Notice and main.rs still emits them (ADR-46), publish-release.yml still verifies the tree it ships through checks.yml (ADR-47), knope.toml still releases from changesets (ADR-48), and ci.yml still gates changesets on .github/release-paths (ADR-49). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-core/src/report.rs@sha256:60b6889bf4eb91c1658c5c90d4f08b4e8b3b6f193d408015508ad53888e9b37f, code:rust/crates/urzua-cli/src/main.rs@sha256:9fae4c3edf849d76d0f53912bc9c090f34f0666ef1a85b2a002b29747fe2ea14, test:rust/crates/urzua-core/src/report.rs@sha256:60b6889bf4eb91c1658c5c90d4f08b4e8b3b6f193d408015508ad53888e9b37f, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b8337c1172d3c4825859e00a0344dd4c35e6f013d802e509b1688ecb` -- stated `Drift detected` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` 9fae4c3edf84 -> f317c199a33d. Reason: Read the diff of each pinned file since the old pin and the claim still holds. main.rs: only the doc comment on the Init command changed, so its --help text now says --dry-run prints the config the real run would write instead of calling it byte-identical to the real run; no flag, argument or behaviour changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` f893cb99b833 -> 8538aedb7492. Reason: the tests in check_integration.rs now name real tracked files; the behaviour this record relies on is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` 8538aedb7492 -> 5ab61b95fd58. Reason: check_integration.rs: the only change is in pre_commit_hook_stops_a_planted_bad_header_with_the_real_binary, which now symlinks the urzua binary into the fixture instead of copying it (a copy raced a forking test thread and failed with ETXTBSY under load); the test's setup, planted defect and assertions are otherwise unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 60b6889bf4eb -> f8332710ffd7, `rust/crates/urzua-cli/src/main.rs` f317c199a33d -> e3b1b6050ff1. Reason: Read the diff of each re-pinned file. main.rs: added the global --as-of option on Cli, the fix_evaluation_date gate that refuses it with --apply and on commands that compare no dates, the as_of() report helper, and parse_cli, which now wraps the unchanged argument parse and that gate; command dispatch otherwise unchanged; report.rs: added the AsOf and AsOfSourceKind types and an as_of field on CheckReport and FixReport; the existing fields, notices and exit_code contract are unchanged. Nothing this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` f8332710ffd7 -> 96587e15e363. Reason: Read the diff of each pinned file or range since the old pin against what this record evidences: report.rs gained an additive Population counter excused_lines (skipped from the JSON when zero) with an accessor and a with_excused_lines builder; nothing else in the report contract changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 96587e15e363 -> fd0b8c483216, `rust/crates/urzua-cli/src/main.rs` e3b1b6050ff1 -> c7e27293c39c. Reason: Read the diff of each re-pinned file against main. core/report.rs: Notice gains an optional detail map of untagged string or string-list values, with DetailValue and Notice::new; cli/main.rs: the command about text states the product positioning; no behaviour change. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` c7e27293c39c -> 0b6a713c8273. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: main.rs: split main into install_panic_hook, panic_if_requested, dispatch_migrate, dispatch_harness and dispatch_export to stay under the SPEC-22 function bound; main's #445 --help about text is unchanged. Behaviour unchanged, outputs byte-identical (check, check --base origin/main, fix, fix --rebaseline, reverify, reverify --as-of 2027-06-01 and --help stdout, stderr and exit compared against a binary built from origin/main). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` fd0b8c483216 -> 83a590ba0c3e. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: report.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged; adds Finding::new, at, at_opt, declared, declared_opt and RuleExecution::ran, not_applicable, not_enabled; no existing item changed. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` 5ab61b95fd58 -> a60c9c61b94c. Reason: Read the diff of each pinned file since the previous pin. rust/crates/urzua-cli/tests/check_integration.rs: adds one regression test that a claim file with an upper-case .MD extension under claim_paths is read; nothing else changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 83a590ba0c3e -> e4387ad0f746, `rust/crates/urzua-cli/src/main.rs` 0b6a713c8273 -> bb8f51c2f61e. Reason: Read the diff of each re-pinned file against main. main.rs gained a Waive variant of the Command enum (with its --from-report, --rule, --path and --by flags), one dispatch arm calling commands::waive::run and one command_name arm; every existing variant and arm is unchanged; report.rs: added WaiveReport, WaiveDraft and WaiveFindings with their Report impl for urzua waive; no existing type or function changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` bb8f51c2f61e -> 6aea9772c936. Reason: Read the diff of each re-pinned file against main. since the previous pin of this change: the Waive dispatch arm passes rule, path and by straight to commands::waive::run instead of building an args struct in main (main was over the function-size bound); the Command enum and every other arm are unchanged, no behaviour changed. [first delta of this change: main.rs gained a Waive variant of the Command enum (with its --from-report, --rule, --path and --by flags), one dispatch arm calling commands::waive::run and one command_name arm; every existing variant and arm is unchanged]. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
